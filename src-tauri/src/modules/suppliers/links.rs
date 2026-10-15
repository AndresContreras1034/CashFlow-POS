use serde_json::{json, Value as JsonValue};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::modules::audit::{
    actor::declared_actor,
    changes::changed_fields,
    dto::NewAuditEvent,
    models::{AuditCategory, AuditModule, AuditOutcome},
    repository as audit_repo,
};
use crate::modules::suppliers::{
    dto::{
        LinkSupplierVariantDto, SetSupplierCategoriesDto, SupplierVariantFilterDto,
        UnlinkSupplierVariantDto, UpdateSupplierVariantDto,
    },
    models::{SupplierCategory, SupplierVariantLink, SupplierVariantView, VariantSupplier},
};

const STOCK_STATUS_CASE: &str = "CASE WHEN v.stock <= 0 THEN 'out_of_stock' \
     WHEN v.stock <= v.stock_min THEN 'low' ELSE 'ok' END";

#[derive(Debug)]
pub enum LinkError {
    Db(sqlx::Error),
    NotFound,
    InactiveSupplier,
    DuplicateLink,
    PreferredConflict,
    InvalidCategory,
}

impl From<sqlx::Error> for LinkError {
    fn from(error: sqlx::Error) -> Self {
        match error.as_database_error().and_then(|db| db.constraint()) {
            Some("uq_supplier_variants_pair") => LinkError::DuplicateLink,
            Some("uq_supplier_variants_preferred") => LinkError::PreferredConflict,
            _ => LinkError::Db(error),
        }
    }
}

async fn record_event(
    conn: &mut sqlx::PgConnection,
    correlation_id: Uuid,
    actor: Option<String>,
    action: &str,
    entity_type: &str,
    entity_id: i32,
    summary: String,
    changes: Option<JsonValue>,
    metadata: JsonValue,
) -> Result<(), sqlx::Error> {
    audit_repo::insert_event(
        conn,
        NewAuditEvent {
            correlation_id: Some(correlation_id),
            category: AuditCategory::Business,
            module: AuditModule::Suppliers,
            action: action.to_string(),
            outcome: AuditOutcome::Success,
            actor,
            entity_type: Some(entity_type.to_string()),
            entity_id: Some(entity_id.to_string()),
            summary,
            changes,
            metadata: Some(metadata),
            error_message: None,
        },
    )
    .await?;
    Ok(())
}

async fn lock_supplier(
    conn: &mut sqlx::PgConnection,
    id: i32,
    require_active: bool,
) -> Result<(), LinkError> {
    let active =
        sqlx::query_scalar::<_, bool>("SELECT is_active FROM suppliers WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *conn)
            .await?
            .ok_or(LinkError::NotFound)?;
    if require_active && !active {
        return Err(LinkError::InactiveSupplier);
    }
    Ok(())
}

async fn lock_variant(conn: &mut sqlx::PgConnection, id: i32) -> Result<(), LinkError> {
    let exists =
        sqlx::query_scalar::<_, i32>("SELECT id FROM product_variants WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *conn)
            .await?;
    exists.ok_or(LinkError::NotFound)?;
    Ok(())
}

pub async fn set_supplier_categories(
    pool: &PgPool,
    supplier_id: i32,
    dto: SetSupplierCategoriesDto,
) -> Result<(), LinkError> {
    let actor = declared_actor(&dto.updated_by);
    let mut tx = pool.begin().await?;
    lock_supplier(&mut tx, supplier_id, false).await?;

    let category_ids = super::service::deduplicate_categories(dto.category_ids);
    let active_ids = if category_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_scalar::<_, i32>(
            "SELECT id FROM categories
             WHERE id = ANY($1) AND is_active
             ORDER BY id
             FOR SHARE",
        )
        .bind(&category_ids)
        .fetch_all(&mut *tx)
        .await?
    };
    if active_ids.len() != category_ids.len() {
        return Err(LinkError::InvalidCategory);
    }

    let before = sqlx::query_scalar::<_, i32>(
        "SELECT category_id FROM supplier_categories WHERE supplier_id = $1 ORDER BY category_id",
    )
    .bind(supplier_id)
    .fetch_all(&mut *tx)
    .await?;
    let (added, removed) = super::service::category_diff(&before, &category_ids);
    if added.is_empty() && removed.is_empty() {
        return Ok(());
    }

    let from_names = if before.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as::<_, (i32, String)>(
            "SELECT id, name FROM categories WHERE id = ANY($1) ORDER BY id",
        )
        .bind(&before)
        .fetch_all(&mut *tx)
        .await?
    };
    let to_names = if category_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as::<_, (i32, String)>(
            "SELECT id, name FROM categories WHERE id = ANY($1) ORDER BY id",
        )
        .bind(&category_ids)
        .fetch_all(&mut *tx)
        .await?
    };

    if !removed.is_empty() {
        sqlx::query(
            "DELETE FROM supplier_categories WHERE supplier_id = $1 AND category_id = ANY($2)",
        )
        .bind(supplier_id)
        .bind(&removed)
        .execute(&mut *tx)
        .await?;
    }
    if !added.is_empty() {
        sqlx::query(
            "INSERT INTO supplier_categories (supplier_id, category_id)
             SELECT $1, unnest($2::INT[])",
        )
        .bind(supplier_id)
        .bind(&added)
        .execute(&mut *tx)
        .await?;
    }

    let correlation_id = Uuid::new_v4();
    record_event(
        &mut tx,
        correlation_id,
        actor,
        "set_categories",
        "supplier",
        supplier_id,
        format!("Categorías del proveedor #{supplier_id} actualizadas"),
        Some(json!({ "categories": { "from": before, "to": category_ids } })),
        json!({
            "from_names": from_names.iter().map(|(_, name)| name).collect::<Vec<_>>(),
            "to_names": to_names.iter().map(|(_, name)| name).collect::<Vec<_>>(),
        }),
    )
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn list_supplier_categories(
    pool: &PgPool,
    supplier_id: i32,
) -> Result<Vec<SupplierCategory>, LinkError> {
    let exists = sqlx::query_scalar::<_, i32>("SELECT id FROM suppliers WHERE id = $1")
        .bind(supplier_id)
        .fetch_optional(pool)
        .await?;
    exists.ok_or(LinkError::NotFound)?;

    sqlx::query_as::<_, SupplierCategory>(
        "SELECT c.id AS category_id, c.name AS category_name
         FROM supplier_categories sc
         JOIN categories c ON c.id = sc.category_id
         WHERE sc.supplier_id = $1
         ORDER BY lower(c.name), c.id",
    )
    .bind(supplier_id)
    .fetch_all(pool)
    .await
    .map_err(LinkError::from)
}

async fn unmark_preferred(
    conn: &mut sqlx::PgConnection,
    variant_id: i32,
    excluding_link_id: Option<i32>,
    supplier_id: i32,
    actor: Option<String>,
    correlation_id: Uuid,
) -> Result<(), LinkError> {
    let existing = sqlx::query_as::<_, SupplierVariantLink>(
        "SELECT id, supplier_id, variant_id, supplier_sku, is_preferred,
                last_purchase_at, notes, created_by, created_at, updated_at
         FROM supplier_variants
         WHERE variant_id = $1 AND is_preferred
           AND ($2::INT IS NULL OR id <> $2)
         FOR UPDATE",
    )
    .bind(variant_id)
    .bind(excluding_link_id)
    .fetch_optional(&mut *conn)
    .await?;
    let Some(before) = existing else {
        return Ok(());
    };

    let after = sqlx::query_as::<_, SupplierVariantLink>(
        "UPDATE supplier_variants SET is_preferred = FALSE
         WHERE id = $1
         RETURNING id, supplier_id, variant_id, supplier_sku, is_preferred,
                   last_purchase_at, notes, created_by, created_at, updated_at",
    )
    .bind(before.id)
    .fetch_one(&mut *conn)
    .await?;
    let changes = changed_fields(
        &before,
        &after,
        &["id", "created_by", "created_at", "updated_at"],
    );
    record_event(
        conn,
        correlation_id,
        actor,
        "update_variant",
        "supplier_variant",
        after.id,
        format!("Variante #{} dejó de ser preferida", after.variant_id),
        Some(JsonValue::Object(changes)),
        json!({
            "supplier_id": after.supplier_id,
            "variant_id": after.variant_id,
            "preferred_by_supplier_id": supplier_id,
        }),
    )
    .await?;
    Ok(())
}

pub async fn link_supplier_variant(
    pool: &PgPool,
    supplier_id: i32,
    dto: LinkSupplierVariantDto,
) -> Result<SupplierVariantLink, LinkError> {
    let actor = declared_actor(&dto.created_by);
    let created_by = actor.as_deref().unwrap_or("system");
    let mut tx = pool.begin().await?;
    lock_supplier(&mut tx, supplier_id, true).await?;
    lock_variant(&mut tx, dto.variant_id).await?;

    let correlation_id = Uuid::new_v4();
    if dto.is_preferred {
        unmark_preferred(
            &mut tx,
            dto.variant_id,
            None,
            supplier_id,
            actor.clone(),
            correlation_id,
        )
        .await?;
    }

    let link = sqlx::query_as::<_, SupplierVariantLink>(
        "INSERT INTO supplier_variants
             (supplier_id, variant_id, supplier_sku, is_preferred, notes, created_by)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, supplier_id, variant_id, supplier_sku, is_preferred,
                   last_purchase_at, notes, created_by, created_at, updated_at",
    )
    .bind(supplier_id)
    .bind(dto.variant_id)
    .bind(&dto.supplier_sku)
    .bind(dto.is_preferred)
    .bind(&dto.notes)
    .bind(created_by)
    .fetch_one(&mut *tx)
    .await?;

    record_event(
        &mut tx,
        correlation_id,
        actor,
        "link_variant",
        "supplier_variant",
        link.id,
        format!(
            "Variante #{} vinculada al proveedor #{supplier_id}",
            link.variant_id
        ),
        None,
        json!({
            "supplier_id": link.supplier_id,
            "variant_id": link.variant_id,
            "supplier_sku": link.supplier_sku,
            "is_preferred": link.is_preferred,
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(link)
}

pub async fn update_supplier_variant(
    pool: &PgPool,
    link_id: i32,
    dto: UpdateSupplierVariantDto,
) -> Result<SupplierVariantLink, LinkError> {
    let actor = declared_actor(&dto.updated_by);
    let mut tx = pool.begin().await?;
    let location = sqlx::query_as::<_, (i32, i32)>(
        "SELECT supplier_id, variant_id FROM supplier_variants WHERE id = $1",
    )
    .bind(link_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(LinkError::NotFound)?;
    lock_supplier(&mut tx, location.0, true).await?;
    lock_variant(&mut tx, location.1).await?;

    let before = sqlx::query_as::<_, SupplierVariantLink>(
        "SELECT id, supplier_id, variant_id, supplier_sku, is_preferred,
                last_purchase_at, notes, created_by, created_at, updated_at
         FROM supplier_variants WHERE id = $1 FOR UPDATE",
    )
    .bind(link_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(LinkError::NotFound)?;
    if before.supplier_id != location.0 || before.variant_id != location.1 {
        return Err(LinkError::NotFound);
    }
    if before.supplier_sku == dto.supplier_sku
        && before.is_preferred == dto.is_preferred
        && before.notes == dto.notes
    {
        return Ok(before);
    }

    let correlation_id = Uuid::new_v4();
    if dto.is_preferred {
        unmark_preferred(
            &mut tx,
            before.variant_id,
            Some(link_id),
            before.supplier_id,
            actor.clone(),
            correlation_id,
        )
        .await?;
    }

    let after = sqlx::query_as::<_, SupplierVariantLink>(
        "UPDATE supplier_variants
         SET supplier_sku = $2, is_preferred = $3, notes = $4
         WHERE id = $1
         RETURNING id, supplier_id, variant_id, supplier_sku, is_preferred,
                   last_purchase_at, notes, created_by, created_at, updated_at",
    )
    .bind(link_id)
    .bind(&dto.supplier_sku)
    .bind(dto.is_preferred)
    .bind(&dto.notes)
    .fetch_one(&mut *tx)
    .await?;
    let changes = changed_fields(
        &before,
        &after,
        &[
            "id",
            "supplier_id",
            "variant_id",
            "created_by",
            "created_at",
            "updated_at",
        ],
    );
    record_event(
        &mut tx,
        correlation_id,
        actor,
        "update_variant",
        "supplier_variant",
        after.id,
        format!("Vínculo de variante #{} actualizado", after.variant_id),
        Some(JsonValue::Object(changes)),
        json!({ "supplier_id": after.supplier_id, "variant_id": after.variant_id }),
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}

pub async fn unlink_supplier_variant(
    pool: &PgPool,
    link_id: i32,
    dto: UnlinkSupplierVariantDto,
) -> Result<(), LinkError> {
    let actor = declared_actor(&dto.updated_by);
    let mut tx = pool.begin().await?;
    let location = sqlx::query_as::<_, (i32, i32)>(
        "SELECT supplier_id, variant_id FROM supplier_variants WHERE id = $1",
    )
    .bind(link_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(LinkError::NotFound)?;
    lock_supplier(&mut tx, location.0, false).await?;
    lock_variant(&mut tx, location.1).await?;
    let link = sqlx::query_as::<_, SupplierVariantLink>(
        "SELECT id, supplier_id, variant_id, supplier_sku, is_preferred,
                last_purchase_at, notes, created_by, created_at, updated_at
         FROM supplier_variants WHERE id = $1 FOR UPDATE",
    )
    .bind(link_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(LinkError::NotFound)?;
    if link.supplier_id != location.0 || link.variant_id != location.1 {
        return Err(LinkError::NotFound);
    }
    sqlx::query("DELETE FROM supplier_variants WHERE id = $1")
        .bind(link_id)
        .execute(&mut *tx)
        .await?;

    record_event(
        &mut tx,
        Uuid::new_v4(),
        actor,
        "unlink_variant",
        "supplier_variant",
        link.id,
        format!(
            "Variante #{} desvinculada del proveedor #{}",
            link.variant_id, link.supplier_id
        ),
        None,
        json!({
            "supplier_id": link.supplier_id,
            "variant_id": link.variant_id,
            "supplier_sku": link.supplier_sku,
            "is_preferred": link.is_preferred,
            "last_purchase_at": link.last_purchase_at,
            "notes": link.notes,
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

fn push_variant_filters(
    qb: &mut QueryBuilder<'_, Postgres>,
    supplier_id: i32,
    filter: &SupplierVariantFilterDto,
) {
    qb.push(" WHERE sv.supplier_id = ").push_bind(supplier_id);
    if !filter.include_inactive.unwrap_or(false) {
        qb.push(" AND v.is_active AND p.is_active");
    }
    if let Some(true) = filter.only_preferred {
        qb.push(" AND sv.is_preferred");
    } else if let Some(false) = filter.only_preferred {
        qb.push(" AND NOT sv.is_preferred");
    }
    if let Some(category_id) = filter.category_id {
        qb.push(" AND p.category_id = ").push_bind(category_id);
    }
    if let Some(search) = filter.search.as_deref().filter(|s| !s.is_empty()) {
        let escaped = search
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = format!("%{escaped}%");
        qb.push(" AND (p.name ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR v.sku ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR v.barcode ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR sv.supplier_sku ILIKE ")
            .push_bind(pattern)
            .push(")");
    }
}

pub async fn list_supplier_variants(
    pool: &PgPool,
    supplier_id: i32,
    filter: &SupplierVariantFilterDto,
) -> Result<(Vec<SupplierVariantView>, i64), LinkError> {
    let supplier_exists = sqlx::query_scalar::<_, i32>("SELECT id FROM suppliers WHERE id = $1")
        .bind(supplier_id)
        .fetch_optional(pool)
        .await?;
    supplier_exists.ok_or(LinkError::NotFound)?;

    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1).saturating_mul(page_size);
    let selection = format!(
        "SELECT sv.id, sv.supplier_id, sv.variant_id, sv.supplier_sku,
                sv.is_preferred, sv.last_purchase_at, sv.notes, sv.created_by, sv.created_at,
                sv.updated_at, p.name AS product_name, v.attributes, v.sku, v.barcode,
                p.category_id, c.name AS category_name, v.stock, v.stock_min,
                {STOCK_STATUS_CASE} AS stock_status, v.cost AS variant_cost, v.price AS variant_price
         FROM supplier_variants sv
         JOIN product_variants v ON v.id = sv.variant_id
         JOIN products p ON p.id = v.product_id
         JOIN categories c ON c.id = p.category_id"
    );
    let mut rows = QueryBuilder::<Postgres>::new(selection);
    push_variant_filters(&mut rows, supplier_id, filter);
    rows.push(" ORDER BY sv.is_preferred DESC, lower(p.name), sv.id LIMIT ")
        .push_bind(page_size)
        .push(" OFFSET ")
        .push_bind(offset);
    let data = rows
        .build_query_as::<SupplierVariantView>()
        .fetch_all(pool)
        .await?;

    let mut count = QueryBuilder::<Postgres>::new(
        "SELECT COUNT(*) FROM supplier_variants sv
         JOIN product_variants v ON v.id = sv.variant_id
         JOIN products p ON p.id = v.product_id",
    );
    push_variant_filters(&mut count, supplier_id, filter);
    let total = count.build_query_scalar::<i64>().fetch_one(pool).await?;
    Ok((data, total))
}

pub async fn list_variant_suppliers(
    pool: &PgPool,
    variant_id: i32,
) -> Result<Vec<VariantSupplier>, LinkError> {
    let exists = sqlx::query_scalar::<_, i32>("SELECT id FROM product_variants WHERE id = $1")
        .bind(variant_id)
        .fetch_optional(pool)
        .await?;
    exists.ok_or(LinkError::NotFound)?;

    sqlx::query_as::<_, VariantSupplier>(
        "SELECT sv.id, s.id AS supplier_id, s.name AS supplier_name, s.phone, s.whatsapp,
                sv.supplier_sku, sv.last_purchase_at, sv.is_preferred
         FROM supplier_variants sv
         JOIN suppliers s ON s.id = sv.supplier_id
         WHERE sv.variant_id = $1
         ORDER BY sv.is_preferred DESC, lower(s.name), s.id",
    )
    .bind(variant_id)
    .fetch_all(pool)
    .await
    .map_err(LinkError::from)
}

pub(crate) fn stock_status_case() -> &'static str {
    STOCK_STATUS_CASE
}
