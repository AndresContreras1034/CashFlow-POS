//! Persisted purchase lists and their editable draft items.
//!
//! List-level actions are audited. Draft item edits are deliberately not
//! audited because the UI will autosave them.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use sqlx::{PgConnection, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::errors::app_error::AppError;
use crate::modules::audit::{
    actor::declared_actor,
    changes::changed_fields,
    dto::NewAuditEvent,
    models::{AuditCategory, AuditModule, AuditOutcome},
    repository as audit_repo,
};
use crate::modules::suppliers::{
    bulk,
    dto::{LinkableFilterDto, PaginatedResponse},
    restock::{self, RestockFilterDto},
};

pub const MAX_TITLE: usize = 150;
pub const MAX_NOTES: usize = 2000;
pub const MAX_ITEM_NOTE: usize = 500;
pub const MAX_QTY: i32 = 100_000;
pub const MAX_ITEMS_PER_LIST: i64 = 500;
const AUDIT_IDS_LIMIT: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PurchaseList {
    pub id: i32,
    pub title: String,
    pub notes: Option<String>,
    pub status: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub ordered_at: Option<DateTime<Utc>>,
    pub received_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PurchaseListSummary {
    pub id: i32,
    pub title: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub item_count: i64,
    pub supplier_count: i64,
    pub estimated_cost: i64,
    pub items_without_cost: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PurchaseSupplier {
    pub id: i32,
    pub name: String,
    pub phone: Option<String>,
    pub whatsapp: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ItemRow {
    item_id: i32,
    variant_id: i32,
    product_id: i32,
    product_name: String,
    brand: Option<String>,
    attributes: JsonValue,
    sku: Option<String>,
    barcode: Option<String>,
    category_name: String,
    stock: i32,
    stock_min: i32,
    variant_cost: i64,
    variant_price: i64,
    quantity: i32,
    source: String,
    suggested_qty: Option<i32>,
    note: Option<String>,
    supplier_id: Option<i32>,
    supplier_name: Option<String>,
    supplier_phone: Option<String>,
    supplier_whatsapp: Option<String>,
    supplier_email: Option<String>,
    supplier_sku: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PurchaseItem {
    pub item_id: i32,
    pub variant_id: i32,
    pub product_id: i32,
    pub product_name: String,
    pub brand: Option<String>,
    pub attributes: JsonValue,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub category_name: String,
    pub stock: i32,
    pub stock_min: i32,
    pub stock_status: &'static str,
    pub variant_cost: i64,
    pub variant_price: i64,
    pub quantity: i32,
    pub line_cost: Option<i64>,
    pub source: String,
    pub suggested_qty: Option<i32>,
    pub note: Option<String>,
    pub supplier: Option<PurchaseSupplier>,
    pub supplier_sku: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PurchaseListDetail {
    pub list: PurchaseList,
    pub items: Vec<PurchaseItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddItemsResult {
    pub requested: usize,
    pub added: usize,
    pub already_in_list: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatePurchaseListDto {
    pub title: String,
    pub notes: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePurchaseListDto {
    pub title: String,
    pub notes: Option<String>,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetListStatusDto {
    pub status: String,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddItemsDto {
    pub variant_ids: Vec<i32>,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddRecommendedDto {
    pub supplier_id: Option<i32>,
    pub variant_ids: Option<Vec<i32>>,
    pub only_urgent: Option<bool>,
    pub window_days: Option<i32>,
    pub cover_days: Option<i32>,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateItemDto {
    pub quantity: i32,
    pub supplier_id: Option<i32>,
    pub note: Option<String>,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PurchaseListFilterDto {
    pub scope: Option<String>,
    pub search: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PurchasableVariant {
    pub variant_id: i32,
    pub product_id: i32,
    pub product_name: String,
    pub brand: Option<String>,
    pub attributes: JsonValue,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub category_name: String,
    pub stock: i32,
    pub stock_min: i32,
    pub variant_cost: i64,
    pub in_list: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PurchasablePage {
    pub data: Vec<PurchasableVariant>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PurchasableIds {
    pub ids: Vec<i32>,
    pub truncated: bool,
}

pub fn normalize_title(value: &str) -> Result<String, AppError> {
    let title = value.trim();
    if title.is_empty() {
        return Err(AppError::validation("El título es obligatorio"));
    }
    if title.chars().count() > MAX_TITLE {
        return Err(AppError::validation(&format!(
            "El título no puede superar {MAX_TITLE} caracteres"
        )));
    }
    Ok(title.to_string())
}

fn optional_text(
    value: Option<String>,
    label: &str,
    max: usize,
) -> Result<Option<String>, AppError> {
    let Some(text) = value
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
    else {
        return Ok(None);
    };
    if text.chars().count() > max {
        return Err(AppError::validation(&format!(
            "{label} no puede superar {max} caracteres"
        )));
    }
    Ok(Some(text))
}

pub fn validate_quantity(quantity: i32) -> Result<i32, AppError> {
    if (1..=MAX_QTY).contains(&quantity) {
        Ok(quantity)
    } else {
        Err(AppError::validation(&format!(
            "La cantidad debe estar entre 1 y {MAX_QTY}"
        )))
    }
}

pub fn clamp_quantity(raw: i64) -> i32 {
    raw.clamp(1, i64::from(MAX_QTY)) as i32
}

pub fn status_label(status: &str) -> &'static str {
    match status {
        "draft" => "borrador",
        "ordered" => "pedida",
        "received" => "recibida",
        "cancelled" => "cancelada",
        _ => "desconocido",
    }
}

pub fn can_transition(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("draft", "ordered")
            | ("ordered", "draft")
            | ("draft", "cancelled")
            | ("ordered", "cancelled")
    )
}

pub fn stock_status(stock: i32, stock_min: i32) -> &'static str {
    if stock <= 0 {
        "out_of_stock"
    } else if stock <= stock_min {
        "low"
    } else {
        "ok"
    }
}

pub fn line_cost(quantity: i32, unit_cost: i64) -> Result<Option<i64>, AppError> {
    if unit_cost <= 0 {
        return Ok(None);
    }
    i64::from(quantity)
        .checked_mul(unit_cost)
        .map(Some)
        .ok_or_else(|| AppError::validation("El costo de la línea supera el máximo permitido"))
}

const LIST_COLS: &str = "id, title, notes, status, created_by, created_at, updated_at, \
                         ordered_at, received_at, cancelled_at";

const ITEM_SELECT: &str = "SELECT i.id AS item_id, i.variant_id, p.id AS product_id,
        p.name AS product_name, p.brand, v.attributes, v.sku, v.barcode,
        c.name AS category_name, v.stock, v.stock_min,
        v.cost AS variant_cost, v.price AS variant_price,
        i.quantity, i.source, i.suggested_qty, i.note,
        i.supplier_id, s.name AS supplier_name, s.phone AS supplier_phone,
        s.whatsapp AS supplier_whatsapp, s.email AS supplier_email, sv.supplier_sku
    FROM purchase_list_items i
    JOIN product_variants v ON v.id = i.variant_id
    JOIN products p ON p.id = v.product_id
    JOIN categories c ON c.id = p.category_id
    LEFT JOIN suppliers s ON s.id = i.supplier_id
    LEFT JOIN supplier_variants sv
           ON sv.variant_id = i.variant_id AND sv.supplier_id = i.supplier_id";

fn to_item(row: ItemRow) -> Result<PurchaseItem, AppError> {
    let supplier = row.supplier_id.map(|id| PurchaseSupplier {
        id,
        name: row.supplier_name.clone().unwrap_or_default(),
        phone: row.supplier_phone.clone(),
        whatsapp: row.supplier_whatsapp.clone(),
        email: row.supplier_email.clone(),
    });
    Ok(PurchaseItem {
        item_id: row.item_id,
        variant_id: row.variant_id,
        product_id: row.product_id,
        product_name: row.product_name,
        brand: row.brand,
        attributes: row.attributes,
        sku: row.sku,
        barcode: row.barcode,
        category_name: row.category_name,
        stock: row.stock,
        stock_min: row.stock_min,
        stock_status: stock_status(row.stock, row.stock_min),
        variant_cost: row.variant_cost,
        variant_price: row.variant_price,
        quantity: row.quantity,
        line_cost: line_cost(row.quantity, row.variant_cost)?,
        source: row.source,
        suggested_qty: row.suggested_qty,
        note: row.note,
        supplier,
        supplier_sku: row.supplier_sku,
    })
}

fn list_event(
    actor: Option<String>,
    action: &str,
    list_id: i32,
    summary: String,
    changes: Option<JsonValue>,
    metadata: JsonValue,
) -> NewAuditEvent {
    NewAuditEvent {
        correlation_id: Some(Uuid::new_v4()),
        category: AuditCategory::Business,
        module: AuditModule::Suppliers,
        action: action.to_string(),
        outcome: AuditOutcome::Success,
        actor,
        entity_type: Some("purchase_list".to_string()),
        entity_id: Some(list_id.to_string()),
        summary,
        changes,
        metadata: Some(metadata),
        error_message: None,
    }
}

async fn lock_list(conn: &mut PgConnection, id: i32) -> Result<PurchaseList, AppError> {
    sqlx::query_as::<_, PurchaseList>(&format!(
        "SELECT {LIST_COLS} FROM purchase_lists WHERE id = $1 FOR UPDATE"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await
    .map_err(AppError::from)?
    .ok_or_else(|| AppError::not_found("Lista de compras no encontrada"))
}

fn require_draft(list: &PurchaseList) -> Result<(), AppError> {
    if list.status == "draft" {
        Ok(())
    } else {
        Err(AppError::validation(
            "Solo se puede modificar una lista en borrador. Reábrela para editarla",
        ))
    }
}

async fn touch(conn: &mut PgConnection, id: i32) -> Result<(), AppError> {
    sqlx::query("UPDATE purchase_lists SET updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(conn)
        .await
        .map_err(AppError::from)?;
    Ok(())
}

async fn ensure_capacity(
    conn: &mut PgConnection,
    list_id: i32,
    incoming: usize,
) -> Result<(), AppError> {
    let existing: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM purchase_list_items WHERE list_id = $1")
            .bind(list_id)
            .fetch_one(conn)
            .await
            .map_err(AppError::from)?;
    if existing + incoming as i64 > MAX_ITEMS_PER_LIST {
        return Err(AppError::validation(&format!(
            "Una lista admite hasta {MAX_ITEMS_PER_LIST} productos"
        )));
    }
    Ok(())
}

async fn require_list(pool: &PgPool, id: i32) -> Result<(), AppError> {
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM purchase_lists WHERE id = $1)")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(AppError::from)?;
    if exists {
        Ok(())
    } else {
        Err(AppError::not_found("Lista de compras no encontrada"))
    }
}

pub async fn create_purchase_list(
    pool: &PgPool,
    dto: CreatePurchaseListDto,
) -> Result<PurchaseList, AppError> {
    let title = normalize_title(&dto.title)?;
    let notes = optional_text(dto.notes, "Las notas", MAX_NOTES)?;
    let actor = declared_actor(&dto.created_by);
    let created_by = actor.clone().unwrap_or_else(|| "system".to_string());
    let mut tx = pool.begin().await.map_err(AppError::from)?;

    let list = sqlx::query_as::<_, PurchaseList>(&format!(
        "INSERT INTO purchase_lists (title, notes, created_by)
         VALUES ($1, $2, $3) RETURNING {LIST_COLS}"
    ))
    .bind(&title)
    .bind(&notes)
    .bind(&created_by)
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::from)?;

    audit_repo::insert_event(
        &mut tx,
        list_event(
            actor,
            "purchase_list_create",
            list.id,
            format!("Lista de compras #{} creada: {}", list.id, list.title),
            None,
            json!({ "title": list.title }),
        ),
    )
    .await
    .map_err(AppError::from)?;

    tx.commit().await.map_err(AppError::from)?;
    Ok(list)
}

pub async fn update_purchase_list(
    pool: &PgPool,
    id: i32,
    dto: UpdatePurchaseListDto,
) -> Result<PurchaseList, AppError> {
    let title = normalize_title(&dto.title)?;
    let notes = optional_text(dto.notes, "Las notas", MAX_NOTES)?;
    let actor = declared_actor(&dto.updated_by);
    let mut tx = pool.begin().await.map_err(AppError::from)?;

    let before = lock_list(&mut tx, id).await?;
    require_draft(&before)?;

    let after = sqlx::query_as::<_, PurchaseList>(&format!(
        "UPDATE purchase_lists SET title = $2, notes = $3 WHERE id = $1 RETURNING {LIST_COLS}"
    ))
    .bind(id)
    .bind(&title)
    .bind(&notes)
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::from)?;

    let changes = changed_fields(&before, &after, &["id", "updated_at"]);
    if changes.is_empty() {
        return Ok(before);
    }

    audit_repo::insert_event(
        &mut tx,
        list_event(
            actor,
            "purchase_list_update",
            after.id,
            format!("Lista de compras #{} editada", after.id),
            Some(JsonValue::Object(changes)),
            json!({ "title": after.title }),
        ),
    )
    .await
    .map_err(AppError::from)?;

    tx.commit().await.map_err(AppError::from)?;
    Ok(after)
}

pub async fn set_purchase_list_status(
    pool: &PgPool,
    id: i32,
    dto: SetListStatusDto,
) -> Result<PurchaseList, AppError> {
    let target = dto.status.trim().to_lowercase();
    if target == "received" {
        return Err(AppError::validation(
            "«Recibida» se registra al recibir la compra, no desde aquí",
        ));
    }
    if !["draft", "ordered", "cancelled"].contains(&target.as_str()) {
        return Err(AppError::validation("El estado no es válido"));
    }
    let actor = declared_actor(&dto.updated_by);
    let mut tx = pool.begin().await.map_err(AppError::from)?;

    let before = lock_list(&mut tx, id).await?;
    if before.status == target {
        return Ok(before);
    }
    if !can_transition(&before.status, &target) {
        return Err(AppError::validation(&format!(
            "No se puede pasar la lista de «{}» a «{}»",
            status_label(&before.status),
            status_label(&target)
        )));
    }
    if target == "ordered" {
        let items: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM purchase_list_items WHERE list_id = $1")
                .bind(id)
                .fetch_one(&mut *tx)
                .await
                .map_err(AppError::from)?;
        if items == 0 {
            return Err(AppError::validation(
                "La lista está vacía: agrega productos antes de marcarla como pedida",
            ));
        }
    }

    let after = sqlx::query_as::<_, PurchaseList>(&format!(
        "UPDATE purchase_lists
         SET status = $2::TEXT,
             ordered_at = CASE $2::TEXT WHEN 'ordered' THEN NOW() WHEN 'draft' THEN NULL ELSE ordered_at END,
             cancelled_at = CASE $2::TEXT WHEN 'cancelled' THEN NOW() ELSE cancelled_at END
         WHERE id = $1
         RETURNING {LIST_COLS}"
    ))
    .bind(id)
    .bind(&target)
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::from)?;

    let verb = match target.as_str() {
        "ordered" => "marcada como pedida",
        "draft" => "reabierta",
        _ => "cancelada",
    };
    audit_repo::insert_event(
        &mut tx,
        list_event(
            actor,
            "purchase_list_status",
            after.id,
            format!("Lista de compras #{} {}", after.id, verb),
            Some(json!({ "status": { "from": before.status, "to": after.status } })),
            json!({ "title": after.title }),
        ),
    )
    .await
    .map_err(AppError::from)?;

    tx.commit().await.map_err(AppError::from)?;
    Ok(after)
}

pub async fn get_purchase_list(pool: &PgPool, id: i32) -> Result<PurchaseListDetail, AppError> {
    let list = sqlx::query_as::<_, PurchaseList>(&format!(
        "SELECT {LIST_COLS} FROM purchase_lists WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from)?
    .ok_or_else(|| AppError::not_found("Lista de compras no encontrada"))?;

    let rows = sqlx::query_as::<_, ItemRow>(&format!(
        "{ITEM_SELECT} WHERE i.list_id = $1 ORDER BY p.name ASC, i.variant_id ASC"
    ))
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)?;
    let items = rows
        .into_iter()
        .map(to_item)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PurchaseListDetail { list, items })
}

fn push_list_filters(
    query: &mut QueryBuilder<'_, Postgres>,
    open_only: bool,
    pattern: &Option<String>,
) {
    query.push(" WHERE TRUE");
    if open_only {
        query.push(" AND l.status IN ('draft', 'ordered')");
    }
    if let Some(pattern) = pattern {
        query.push(" AND l.title ILIKE ").push_bind(pattern.clone());
    }
}

pub async fn list_purchase_lists(
    pool: &PgPool,
    filter: PurchaseListFilterDto,
) -> Result<PaginatedResponse<PurchaseListSummary>, AppError> {
    let scope = filter.scope.as_deref().unwrap_or("open");
    if !["open", "all"].contains(&scope) {
        return Err(AppError::validation("El filtro de estado no es válido"));
    }
    let open_only = scope == "open";
    let search = optional_text(filter.search, "La búsqueda", 100)?;
    let pattern = search.as_deref().map(bulk::like_pattern);
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(50).clamp(1, 100);
    let offset = (page - 1).saturating_mul(page_size);

    let mut rows_query = QueryBuilder::<Postgres>::new(
        "SELECT l.id, l.title, l.status, l.created_at, l.updated_at,
                COUNT(i.id)::BIGINT AS item_count,
                COUNT(DISTINCT i.supplier_id)::BIGINT AS supplier_count,
                COALESCE(SUM(i.quantity::NUMERIC * v.cost::NUMERIC)
                    FILTER (WHERE v.cost > 0), 0)::BIGINT AS estimated_cost,
                (COUNT(i.id) FILTER (WHERE v.cost <= 0))::BIGINT AS items_without_cost
         FROM purchase_lists l
         LEFT JOIN purchase_list_items i ON i.list_id = l.id
         LEFT JOIN product_variants v ON v.id = i.variant_id",
    );
    push_list_filters(&mut rows_query, open_only, &pattern);
    rows_query
        .push(
            " GROUP BY l.id
              ORDER BY CASE l.status WHEN 'draft' THEN 0 WHEN 'ordered' THEN 1 ELSE 2 END,
                       l.updated_at DESC, l.id DESC
              LIMIT ",
        )
        .push_bind(page_size)
        .push(" OFFSET ")
        .push_bind(offset);
    let data = rows_query
        .build_query_as::<PurchaseListSummary>()
        .fetch_all(pool)
        .await
        .map_err(AppError::from)?;

    let mut count_query = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM purchase_lists l");
    push_list_filters(&mut count_query, open_only, &pattern);
    let total = count_query
        .build_query_scalar::<i64>()
        .fetch_one(pool)
        .await
        .map_err(AppError::from)?;

    Ok(PaginatedResponse::new(data, total, page, page_size))
}

pub async fn add_purchase_items(
    pool: &PgPool,
    list_id: i32,
    dto: AddItemsDto,
) -> Result<AddItemsResult, AppError> {
    let ids = bulk::normalize_ids(dto.variant_ids)?;
    let actor = declared_actor(&dto.updated_by);
    let created_by = actor.clone().unwrap_or_else(|| "system".to_string());
    let mut tx = pool.begin().await.map_err(AppError::from)?;

    let list = lock_list(&mut tx, list_id).await?;
    require_draft(&list)?;

    let valid: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM product_variants v
         JOIN products p ON p.id = v.product_id
         WHERE v.id = ANY($1) AND v.is_active AND p.is_active",
    )
    .bind(&ids[..])
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::from)?;
    if valid as usize != ids.len() {
        return Err(AppError::validation(
            "Algún producto seleccionado ya no existe o está inactivo. Vuelve a buscar y selecciona de nuevo",
        ));
    }
    ensure_capacity(&mut tx, list_id, ids.len()).await?;

    let sql = format!(
        "WITH {assigned}, wanted AS (SELECT UNNEST($2::INT[]) AS variant_id)
         INSERT INTO purchase_list_items
             (list_id, variant_id, supplier_id, quantity, source, created_by)
         SELECT $1::INT, w.variant_id, a.supplier_id, 1, 'manual', $3::TEXT
         FROM wanted w
         LEFT JOIN assigned_suppliers a ON a.variant_id = w.variant_id
         ON CONFLICT (list_id, variant_id) DO NOTHING
         RETURNING variant_id",
        assigned = restock::ASSIGNED_SUPPLIERS_CTE
    );
    let added: Vec<i32> = sqlx::query_scalar(&sql)
        .bind(list_id)
        .bind(&ids[..])
        .bind(&created_by)
        .fetch_all(&mut *tx)
        .await
        .map_err(AppError::from)?;

    finish_add(tx, actor, &list, "manual", ids.len(), added, json!({})).await
}

pub async fn add_recommended_to_list(
    pool: &PgPool,
    list_id: i32,
    dto: AddRecommendedDto,
) -> Result<AddItemsResult, AppError> {
    let actor = declared_actor(&dto.updated_by);
    let created_by = actor.clone().unwrap_or_else(|| "system".to_string());

    let overview = restock::get_restock_overview(
        pool,
        RestockFilterDto {
            supplier_id: dto.supplier_id,
            include_unassigned: Some(true),
            only_urgent: dto.only_urgent,
            window_days: dto.window_days,
            cover_days: dto.cover_days,
            ..Default::default()
        },
    )
    .await?;

    let wanted: Option<HashSet<i32>> = dto.variant_ids.map(|ids| ids.into_iter().collect());
    let (mut variant_ids, mut supplier_ids, mut quantities) = (Vec::new(), Vec::new(), Vec::new());
    for group in &overview.groups {
        let supplier_id = group
            .supplier
            .as_ref()
            .map(|supplier| supplier.id)
            .unwrap_or(0);
        for item in &group.items {
            if wanted
                .as_ref()
                .is_some_and(|set| !set.contains(&item.variant_id))
            {
                continue;
            }
            variant_ids.push(item.variant_id);
            supplier_ids.push(supplier_id);
            quantities.push(clamp_quantity(item.suggested_qty));
        }
    }
    let requested = variant_ids.len();

    let mut tx = pool.begin().await.map_err(AppError::from)?;
    let list = lock_list(&mut tx, list_id).await?;
    require_draft(&list)?;
    if requested == 0 {
        return Ok(AddItemsResult {
            requested: 0,
            added: 0,
            already_in_list: 0,
        });
    }
    let new_items: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM UNNEST($2::INT[]) AS candidate(variant_id)
         WHERE NOT EXISTS (
             SELECT 1 FROM purchase_list_items i
             WHERE i.list_id = $1 AND i.variant_id = candidate.variant_id
         )",
    )
    .bind(list_id)
    .bind(&variant_ids[..])
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::from)?;
    ensure_capacity(&mut tx, list_id, new_items as usize).await?;

    let added: Vec<i32> = sqlx::query_scalar(
        "INSERT INTO purchase_list_items
             (list_id, variant_id, supplier_id, quantity, source, suggested_qty, created_by)
         SELECT $1::INT, x.variant_id, NULLIF(x.supplier_id, 0), x.quantity,
                'recommended', x.quantity, $5::TEXT
         FROM UNNEST($2::INT[], $3::INT[], $4::INT[])
              AS x(variant_id, supplier_id, quantity)
         ON CONFLICT (list_id, variant_id) DO NOTHING
         RETURNING variant_id",
    )
    .bind(list_id)
    .bind(&variant_ids[..])
    .bind(&supplier_ids[..])
    .bind(&quantities[..])
    .bind(&created_by)
    .fetch_all(&mut *tx)
    .await
    .map_err(AppError::from)?;

    finish_add(
        tx,
        actor,
        &list,
        "recommended",
        requested,
        added,
        json!({ "supplier_id": dto.supplier_id, "only_urgent": dto.only_urgent.unwrap_or(false) }),
    )
    .await
}

async fn finish_add(
    mut tx: sqlx::Transaction<'_, Postgres>,
    actor: Option<String>,
    list: &PurchaseList,
    source: &str,
    requested: usize,
    added: Vec<i32>,
    extra: JsonValue,
) -> Result<AddItemsResult, AppError> {
    let result = AddItemsResult {
        requested,
        added: added.len(),
        already_in_list: requested - added.len(),
    };
    if added.is_empty() {
        return Ok(result);
    }

    touch(&mut tx, list.id).await?;
    let shown: Vec<i32> = added.iter().copied().take(AUDIT_IDS_LIMIT).collect();
    let mut metadata = json!({
        "source": source,
        "added": result.added,
        "already_in_list": result.already_in_list,
        "variant_ids": shown,
        "variant_ids_truncated": added.len() > AUDIT_IDS_LIMIT,
    });
    if let (Some(base), Some(more)) = (metadata.as_object_mut(), extra.as_object()) {
        base.extend(more.clone());
    }
    audit_repo::insert_event(
        &mut tx,
        list_event(
            actor,
            "purchase_list_add_items",
            list.id,
            format!(
                "Lista de compras #{}: {} productos agregados",
                list.id, result.added
            ),
            None,
            metadata,
        ),
    )
    .await
    .map_err(AppError::from)?;

    tx.commit().await.map_err(AppError::from)?;
    Ok(result)
}

pub async fn update_purchase_item(
    pool: &PgPool,
    item_id: i32,
    dto: UpdateItemDto,
) -> Result<PurchaseItem, AppError> {
    let quantity = validate_quantity(dto.quantity)?;
    let note = optional_text(dto.note, "La nota", MAX_ITEM_NOTE)?;
    let mut tx = pool.begin().await.map_err(AppError::from)?;

    let (list_id, current_supplier): (i32, Option<i32>) =
        sqlx::query_as("SELECT list_id, supplier_id FROM purchase_list_items WHERE id = $1")
            .bind(item_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(AppError::from)?
            .ok_or_else(|| AppError::not_found("Línea no encontrada"))?;
    let list = lock_list(&mut tx, list_id).await?;
    require_draft(&list)?;

    if let Some(supplier_id) = dto.supplier_id.filter(|id| Some(*id) != current_supplier) {
        let active: Option<bool> =
            sqlx::query_scalar("SELECT is_active FROM suppliers WHERE id = $1")
                .bind(supplier_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(AppError::from)?;
        match active {
            Some(true) => {}
            Some(false) => return Err(AppError::validation("El proveedor está desactivado")),
            None => return Err(AppError::not_found("Proveedor no encontrado")),
        }
    }

    let rows = sqlx::query(
        "UPDATE purchase_list_items SET quantity = $2, supplier_id = $3, note = $4 WHERE id = $1",
    )
    .bind(item_id)
    .bind(quantity)
    .bind(dto.supplier_id)
    .bind(&note)
    .execute(&mut *tx)
    .await
    .map_err(AppError::from)?
    .rows_affected();
    if rows == 0 {
        return Err(AppError::not_found("Línea no encontrada"));
    }
    touch(&mut tx, list_id).await?;
    tx.commit().await.map_err(AppError::from)?;

    let row = sqlx::query_as::<_, ItemRow>(&format!("{ITEM_SELECT} WHERE i.id = $1"))
        .bind(item_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("Línea no encontrada"))?;
    to_item(row)
}

pub async fn remove_purchase_item(pool: &PgPool, item_id: i32) -> Result<(), AppError> {
    let mut tx = pool.begin().await.map_err(AppError::from)?;
    let list_id: i32 = sqlx::query_scalar("SELECT list_id FROM purchase_list_items WHERE id = $1")
        .bind(item_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("Línea no encontrada"))?;
    let list = lock_list(&mut tx, list_id).await?;
    require_draft(&list)?;

    let rows = sqlx::query("DELETE FROM purchase_list_items WHERE id = $1")
        .bind(item_id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::from)?
        .rows_affected();
    if rows == 0 {
        return Err(AppError::not_found("Línea no encontrada"));
    }
    touch(&mut tx, list_id).await?;
    tx.commit().await.map_err(AppError::from)
}

fn push_purchasable(
    query: &mut QueryBuilder<'_, Postgres>,
    list_id: i32,
    filter: &bulk::ValidLinkable,
    exclude_in_list: bool,
) {
    query.push(
        " FROM product_variants v
          JOIN products p ON p.id = v.product_id AND p.is_active
          JOIN categories c ON c.id = p.category_id
          WHERE v.is_active",
    );
    if let Some(category_id) = filter.category_id {
        query.push(" AND p.category_id = ").push_bind(category_id);
    }
    if let Some(text) = &filter.search {
        let pattern = bulk::like_pattern(text);
        query
            .push(" AND (p.name ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR p.brand ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR v.sku ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR v.barcode ILIKE ")
            .push_bind(pattern)
            .push(")");
    }
    if exclude_in_list {
        query
            .push(
                " AND NOT EXISTS (
                     SELECT 1 FROM purchase_list_items x
                     WHERE x.variant_id = v.id AND x.list_id = ",
            )
            .push_bind(list_id)
            .push(")");
    }
}

pub async fn search_purchasable_variants(
    pool: &PgPool,
    list_id: i32,
    filter: LinkableFilterDto,
) -> Result<PurchasablePage, AppError> {
    let filter = bulk::validate_linkable_filter(filter)?;
    require_list(pool, list_id).await?;
    let offset = (filter.page - 1).saturating_mul(filter.page_size);

    let mut rows_query = QueryBuilder::<Postgres>::new(
        "SELECT v.id AS variant_id, p.id AS product_id, p.name AS product_name, p.brand,
                v.attributes, v.sku, v.barcode, c.name AS category_name,
                v.stock, v.stock_min, v.cost AS variant_cost,
                EXISTS (SELECT 1 FROM purchase_list_items il
                        WHERE il.variant_id = v.id AND il.list_id = ",
    );
    rows_query.push_bind(list_id);
    rows_query.push(") AS in_list");
    push_purchasable(&mut rows_query, list_id, &filter, false);
    rows_query
        .push(" ORDER BY p.name ASC, v.id ASC LIMIT ")
        .push_bind(filter.page_size)
        .push(" OFFSET ")
        .push_bind(offset);
    let data = rows_query
        .build_query_as::<PurchasableVariant>()
        .fetch_all(pool)
        .await
        .map_err(AppError::from)?;

    let mut count_query = QueryBuilder::<Postgres>::new("SELECT COUNT(*)");
    push_purchasable(&mut count_query, list_id, &filter, false);
    let total = count_query
        .build_query_scalar::<i64>()
        .fetch_one(pool)
        .await
        .map_err(AppError::from)?;

    Ok(PurchasablePage {
        data,
        total,
        page: filter.page,
        page_size: filter.page_size,
        total_pages: (total + filter.page_size - 1) / filter.page_size,
    })
}

pub async fn list_purchasable_variant_ids(
    pool: &PgPool,
    list_id: i32,
    filter: LinkableFilterDto,
) -> Result<PurchasableIds, AppError> {
    let filter = bulk::validate_linkable_filter(filter)?;
    require_list(pool, list_id).await?;

    let mut query = QueryBuilder::<Postgres>::new("SELECT v.id");
    push_purchasable(&mut query, list_id, &filter, true);
    query
        .push(" ORDER BY p.name ASC, v.id ASC LIMIT ")
        .push_bind(MAX_ITEMS_PER_LIST + 1);
    let mut ids = query
        .build_query_scalar::<i32>()
        .fetch_all(pool)
        .await
        .map_err(AppError::from)?;
    let truncated = ids.len() as i64 > MAX_ITEMS_PER_LIST;
    ids.truncate(MAX_ITEMS_PER_LIST as usize);
    Ok(PurchasableIds { ids, truncated })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_is_required_trimmed_and_bounded() {
        assert!(normalize_title("   ").is_err());
        assert_eq!(normalize_title("  Ida a Bogotá ").unwrap(), "Ida a Bogotá");
        assert!(normalize_title(&"x".repeat(MAX_TITLE + 1)).is_err());
        assert!(normalize_title(&"ñ".repeat(MAX_TITLE)).is_ok());
    }

    #[test]
    fn quantity_must_be_between_one_and_the_cap() {
        assert!(validate_quantity(1).is_ok());
        assert!(validate_quantity(MAX_QTY).is_ok());
        assert!(validate_quantity(0).is_err());
        assert!(validate_quantity(-3).is_err());
        assert!(validate_quantity(MAX_QTY + 1).is_err());
    }

    #[test]
    fn suggested_quantities_are_clamped_into_range() {
        assert_eq!(clamp_quantity(0), 1);
        assert_eq!(clamp_quantity(-5), 1);
        assert_eq!(clamp_quantity(19_889), 19_889);
        assert_eq!(clamp_quantity(i64::MAX), MAX_QTY);
    }

    #[test]
    fn only_the_allowed_status_transitions_are_valid() {
        assert!(can_transition("draft", "ordered"));
        assert!(can_transition("ordered", "draft"));
        assert!(can_transition("draft", "cancelled"));
        assert!(can_transition("ordered", "cancelled"));
        assert!(!can_transition("cancelled", "draft"));
        assert!(!can_transition("received", "draft"));
        assert!(!can_transition("draft", "received"));
        assert!(!can_transition("draft", "draft"));
    }

    #[test]
    fn status_labels_are_in_spanish() {
        assert_eq!(status_label("draft"), "borrador");
        assert_eq!(status_label("ordered"), "pedida");
        assert_eq!(status_label("cancelled"), "cancelada");
        assert_eq!(status_label("???"), "desconocido");
    }

    #[test]
    fn stock_status_follows_the_inventory_rule() {
        assert_eq!(stock_status(0, 3), "out_of_stock");
        assert_eq!(stock_status(-2, 3), "out_of_stock");
        assert_eq!(stock_status(3, 3), "low");
        assert_eq!(stock_status(4, 3), "ok");
    }

    #[test]
    fn line_cost_uses_variant_cost_and_never_overflows_silently() {
        assert_eq!(line_cost(5, 200_000).unwrap(), Some(1_000_000));
        assert_eq!(line_cost(5, 0).unwrap(), None);
        assert_eq!(line_cost(5, -1).unwrap(), None);
        assert!(line_cost(MAX_QTY, i64::MAX).is_err());
    }

    #[test]
    fn item_notes_are_trimmed_and_bounded() {
        assert_eq!(
            optional_text(Some("  ".into()), "La nota", 500).unwrap(),
            None
        );
        assert_eq!(
            optional_text(Some(" pedir caja ".into()), "La nota", 500)
                .unwrap()
                .as_deref(),
            Some("pedir caja")
        );
        assert!(optional_text(Some("n".repeat(501)), "La nota", 500).is_err());
    }
}
