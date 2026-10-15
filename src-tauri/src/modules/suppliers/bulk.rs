use std::collections::HashSet;

use serde::Serialize;
use serde_json::{json, Value as JsonValue};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::errors::app_error::AppError;
use crate::modules::audit::{
    actor::declared_actor,
    dto::NewAuditEvent,
    models::{AuditCategory, AuditModule, AuditOutcome},
    repository as audit_repo,
};
use crate::modules::suppliers::dto::{BulkLinkDto, LinkableFilterDto};

pub const MAX_BULK_LINKS: usize = 500;
pub const MAX_PICKER_PAGE_SIZE: i64 = 100;
const DEFAULT_PICKER_PAGE_SIZE: i64 = 50;
const MIN_SEARCH_CHARS: usize = 2;
const MAX_SEARCH_CHARS: usize = 100;
const AUDIT_IDS_LIMIT: usize = 100;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct LinkableVariant {
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
    pub linked_here: bool,
    pub supplier_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkablePage {
    pub data: Vec<LinkableVariant>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkableIds {
    pub ids: Vec<i32>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BulkLinkResult {
    pub requested: usize,
    pub linked: usize,
    pub already_linked: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct ValidLinkable {
    pub(crate) search: Option<String>,
    pub(crate) category_id: Option<i32>,
    pub(crate) only_unassigned: bool,
    pub(crate) page: i64,
    pub(crate) page_size: i64,
}

pub fn normalize_ids(ids: Vec<i32>) -> Result<Vec<i32>, AppError> {
    let mut seen = HashSet::new();
    let unique: Vec<i32> = ids
        .into_iter()
        .filter(|id| *id > 0 && seen.insert(*id))
        .collect();
    if unique.is_empty() {
        return Err(AppError::validation("Selecciona al menos un producto"));
    }
    if unique.len() > MAX_BULK_LINKS {
        return Err(AppError::validation(&format!(
            "Puedes enlazar hasta {MAX_BULK_LINKS} productos por vez"
        )));
    }
    Ok(unique)
}

pub(crate) fn validate_linkable_filter(
    filter: LinkableFilterDto,
) -> Result<ValidLinkable, AppError> {
    let search = filter
        .search
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty());
    if let Some(text) = &search {
        let length = text.chars().count();
        if length < MIN_SEARCH_CHARS {
            return Err(AppError::validation(
                "La búsqueda necesita al menos 2 caracteres",
            ));
        }
        if length > MAX_SEARCH_CHARS {
            return Err(AppError::validation(&format!(
                "La búsqueda no puede superar {MAX_SEARCH_CHARS} caracteres"
            )));
        }
    }
    let only_unassigned = filter.only_unassigned.unwrap_or(false);
    if search.is_none() && filter.category_id.is_none() && !only_unassigned {
        return Err(AppError::validation(
            "Escribe al menos 2 letras o elige una categoría",
        ));
    }
    Ok(ValidLinkable {
        search,
        category_id: filter.category_id,
        only_unassigned,
        page: filter.page.unwrap_or(1).max(1),
        page_size: filter
            .page_size
            .unwrap_or(DEFAULT_PICKER_PAGE_SIZE)
            .clamp(1, MAX_PICKER_PAGE_SIZE),
    })
}

pub(crate) fn like_pattern(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}

fn push_from_where(
    qb: &mut QueryBuilder<'_, Postgres>,
    supplier_id: i32,
    filter: &ValidLinkable,
    exclude_linked: bool,
) {
    qb.push(
        " FROM product_variants v
          JOIN products p ON p.id = v.product_id AND p.is_active
          JOIN categories c ON c.id = p.category_id
          WHERE v.is_active",
    );
    if let Some(category_id) = filter.category_id {
        qb.push(" AND p.category_id = ").push_bind(category_id);
    }
    if let Some(search) = &filter.search {
        let pattern = like_pattern(search);
        qb.push(" AND (p.name ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR p.brand ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR v.sku ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR v.barcode ILIKE ")
            .push_bind(pattern)
            .push(")");
    }
    if filter.only_unassigned {
        qb.push(
            " AND NOT EXISTS (
                  SELECT 1 FROM supplier_variants su
                  JOIN suppliers sx ON sx.id = su.supplier_id AND sx.is_active
                  WHERE su.variant_id = v.id)",
        );
    }
    if exclude_linked {
        qb.push(
            " AND NOT EXISTS (
                  SELECT 1 FROM supplier_variants sh
                  WHERE sh.variant_id = v.id AND sh.supplier_id = ",
        )
        .push_bind(supplier_id)
        .push(")");
    }
}

async fn require_supplier(pool: &PgPool, supplier_id: i32) -> Result<(), AppError> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM suppliers WHERE id = $1)")
        .bind(supplier_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::from)?;
    if exists {
        Ok(())
    } else {
        Err(AppError::not_found("Proveedor no encontrado"))
    }
}

pub async fn search_linkable_variants(
    pool: &PgPool,
    supplier_id: i32,
    filter: LinkableFilterDto,
) -> Result<LinkablePage, AppError> {
    let filter = validate_linkable_filter(filter)?;
    require_supplier(pool, supplier_id).await?;
    let offset = (filter.page - 1).saturating_mul(filter.page_size);

    let mut rows_query = QueryBuilder::<Postgres>::new(
        "SELECT v.id AS variant_id, p.id AS product_id, p.name AS product_name, p.brand,
                v.attributes, v.sku, v.barcode, c.name AS category_name,
                v.stock, v.stock_min, v.cost AS variant_cost,
                EXISTS (SELECT 1 FROM supplier_variants sl
                        WHERE sl.variant_id = v.id AND sl.supplier_id = ",
    );
    rows_query.push_bind(supplier_id).push(
        ") AS linked_here,
               (SELECT COUNT(*) FROM supplier_variants sa
                JOIN suppliers sy ON sy.id = sa.supplier_id AND sy.is_active
                WHERE sa.variant_id = v.id)::BIGINT AS supplier_count",
    );
    push_from_where(&mut rows_query, supplier_id, &filter, false);
    rows_query
        .push(" ORDER BY p.name ASC, v.id ASC LIMIT ")
        .push_bind(filter.page_size)
        .push(" OFFSET ")
        .push_bind(offset);
    let data = rows_query
        .build_query_as::<LinkableVariant>()
        .fetch_all(pool)
        .await
        .map_err(AppError::from)?;

    let mut count_query = QueryBuilder::<Postgres>::new("SELECT COUNT(*)");
    push_from_where(&mut count_query, supplier_id, &filter, false);
    let total = count_query
        .build_query_scalar::<i64>()
        .fetch_one(pool)
        .await
        .map_err(AppError::from)?;

    Ok(LinkablePage {
        data,
        total,
        page: filter.page,
        page_size: filter.page_size,
        total_pages: (total + filter.page_size - 1) / filter.page_size,
    })
}

pub async fn list_linkable_variant_ids(
    pool: &PgPool,
    supplier_id: i32,
    filter: LinkableFilterDto,
) -> Result<LinkableIds, AppError> {
    let filter = validate_linkable_filter(filter)?;
    require_supplier(pool, supplier_id).await?;

    let mut query = QueryBuilder::<Postgres>::new("SELECT v.id");
    push_from_where(&mut query, supplier_id, &filter, true);
    query
        .push(" ORDER BY p.name ASC, v.id ASC LIMIT ")
        .push_bind((MAX_BULK_LINKS + 1) as i64);
    let mut ids = query
        .build_query_scalar::<i32>()
        .fetch_all(pool)
        .await
        .map_err(AppError::from)?;

    let truncated = ids.len() > MAX_BULK_LINKS;
    ids.truncate(MAX_BULK_LINKS);
    Ok(LinkableIds { ids, truncated })
}

pub async fn link_supplier_variants(
    pool: &PgPool,
    supplier_id: i32,
    dto: BulkLinkDto,
) -> Result<BulkLinkResult, AppError> {
    let ids = normalize_ids(dto.variant_ids)?;
    let actor = declared_actor(&dto.updated_by);
    let created_by = actor.clone().unwrap_or_else(|| "system".to_string());
    let mut tx = pool.begin().await.map_err(AppError::from)?;

    let supplier: Option<(String, bool)> =
        sqlx::query_as("SELECT name, is_active FROM suppliers WHERE id = $1 FOR UPDATE")
            .bind(supplier_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(AppError::from)?;
    let (name, is_active) =
        supplier.ok_or_else(|| AppError::not_found("Proveedor no encontrado"))?;
    if !is_active {
        return Err(AppError::validation(
            "El proveedor está desactivado: reactívalo para enlazar productos",
        ));
    }

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

    let linked: Vec<i32> = sqlx::query_scalar(
        "INSERT INTO supplier_variants (supplier_id, variant_id, created_by)
         SELECT $1::INT, v, $3::TEXT FROM unnest($2::INT[]) AS v
         ON CONFLICT (supplier_id, variant_id) DO NOTHING
         RETURNING variant_id",
    )
    .bind(supplier_id)
    .bind(&ids[..])
    .bind(&created_by)
    .fetch_all(&mut *tx)
    .await
    .map_err(AppError::from)?;

    let result = BulkLinkResult {
        requested: ids.len(),
        linked: linked.len(),
        already_linked: ids.len() - linked.len(),
    };
    if linked.is_empty() {
        return Ok(result);
    }

    let shown: Vec<i32> = linked.iter().copied().take(AUDIT_IDS_LIMIT).collect();
    audit_repo::insert_event(
        &mut tx,
        NewAuditEvent {
            correlation_id: Some(Uuid::new_v4()),
            category: AuditCategory::Business,
            module: AuditModule::Suppliers,
            action: "link_variants".to_string(),
            outcome: AuditOutcome::Success,
            actor,
            entity_type: Some("supplier".to_string()),
            entity_id: Some(supplier_id.to_string()),
            summary: format!(
                "Proveedor #{supplier_id} ({name}): {} productos enlazados",
                result.linked
            ),
            changes: None,
            metadata: Some(json!({
                "linked_count": result.linked,
                "already_linked": result.already_linked,
                "variant_ids": shown,
                "variant_ids_truncated": linked.len() > AUDIT_IDS_LIMIT,
            })),
            error_message: None,
        },
    )
    .await
    .map_err(AppError::from)?;

    tx.commit().await.map_err(AppError::from)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_deduplicated_and_non_positive_ones_dropped() {
        assert_eq!(
            normalize_ids(vec![3, 1, 3, 0, -2, 1, 2]).unwrap(),
            vec![3, 1, 2]
        );
    }

    #[test]
    fn empty_or_oversized_batches_are_rejected() {
        assert!(normalize_ids(vec![]).is_err());
        assert!(normalize_ids(vec![0, -1]).is_err());
        assert!(normalize_ids((1..=MAX_BULK_LINKS as i32).collect()).is_ok());
        assert!(normalize_ids((1..=MAX_BULK_LINKS as i32 + 1).collect()).is_err());
    }

    #[test]
    fn filter_needs_two_letters_or_another_criterion() {
        assert!(validate_linkable_filter(LinkableFilterDto::default()).is_err());

        let one_letter = LinkableFilterDto {
            search: Some("a".into()),
            ..Default::default()
        };
        assert!(validate_linkable_filter(one_letter).is_err());

        let blank = LinkableFilterDto {
            search: Some("   ".into()),
            ..Default::default()
        };
        assert!(validate_linkable_filter(blank).is_err());

        let two = LinkableFilterDto {
            search: Some(" sh ".into()),
            ..Default::default()
        };
        assert_eq!(
            validate_linkable_filter(two).unwrap().search.as_deref(),
            Some("sh")
        );

        let by_category = LinkableFilterDto {
            category_id: Some(3),
            ..Default::default()
        };
        assert!(validate_linkable_filter(by_category).is_ok());

        let unassigned = LinkableFilterDto {
            only_unassigned: Some(true),
            ..Default::default()
        };
        assert!(validate_linkable_filter(unassigned).is_ok());

        let long = LinkableFilterDto {
            search: Some("x".repeat(101)),
            ..Default::default()
        };
        assert!(validate_linkable_filter(long).is_err());
    }

    #[test]
    fn paging_is_bounded() {
        let filter = LinkableFilterDto {
            category_id: Some(1),
            page: Some(-5),
            page_size: Some(10_000),
            ..Default::default()
        };
        let valid = validate_linkable_filter(filter).unwrap();
        assert_eq!((valid.page, valid.page_size), (1, MAX_PICKER_PAGE_SIZE));
    }

    #[test]
    fn like_wildcards_are_escaped() {
        assert_eq!(like_pattern("50%_off"), "%50\\%\\_off%");
        assert_eq!(like_pattern("a\\b"), "%a\\\\b%");
    }
}
