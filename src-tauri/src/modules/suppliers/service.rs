use sqlx::PgPool;

use crate::errors::app_error::AppError;
use crate::modules::suppliers::{
    dto::{
        CreateSupplierDto, LinkSupplierVariantDto, PaginatedResponse, SetSupplierActiveDto,
        SetSupplierCategoriesDto, SupplierFields, SupplierFilterDto, SupplierVariantFilterDto,
        UnlinkSupplierVariantDto, UpdateSupplierDto, UpdateSupplierVariantDto,
    },
    links::{self, LinkError},
    models::{
        Supplier, SupplierCategory, SupplierOverview, SupplierVariantLink, SupplierVariantView,
        VariantSupplier,
    },
    repository::{self, SupplierError},
};

const MAX_NAME: usize = 150;
const MAX_CONTACT: usize = 150;
const MAX_DOC_NUMBER: usize = 30;
const MAX_PHONE: usize = 50;
const MAX_EMAIL: usize = 150;
const MAX_CITY: usize = 100;
const MAX_VISIT_DAYS: usize = 100;
const MAX_TEXT: usize = 2000;
const MAX_SEARCH: usize = 100;
const MAX_SUPPLIER_SKU: usize = 100;
const MAX_DAYS: i32 = 365;

pub(crate) fn deduplicate_categories(ids: Vec<i32>) -> Vec<i32> {
    let mut ids = ids;
    ids.sort_unstable();
    ids.dedup();
    ids
}

pub(crate) fn category_diff(before: &[i32], after: &[i32]) -> (Vec<i32>, Vec<i32>) {
    let before: std::collections::BTreeSet<_> = before.iter().copied().collect();
    let after: std::collections::BTreeSet<_> = after.iter().copied().collect();
    (
        after.difference(&before).copied().collect(),
        before.difference(&after).copied().collect(),
    )
}

pub const DOCUMENT_TYPES: [&str; 4] = ["NIT", "CC", "CE", "OTRO"];
pub const PRIORITIES: [&str; 3] = ["normal", "preferred", "urgent"];

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

/// Teléfono / WhatsApp: solo dígitos y separadores comunes, de 7 a 15 dígitos.
fn optional_phone(value: Option<String>, label: &str) -> Result<Option<String>, AppError> {
    let Some(text) = optional_text(value, label, MAX_PHONE)? else {
        return Ok(None);
    };
    let allowed = text
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, ' ' | '+' | '-' | '(' | ')' | '.'));
    let digits = text.chars().filter(char::is_ascii_digit).count();
    if !allowed || !(7..=15).contains(&digits) {
        return Err(AppError::validation(&format!(
            "{label} no es válido: usa solo dígitos, espacios y + - ( ) . (de 7 a 15 dígitos)"
        )));
    }
    Ok(Some(text))
}

fn optional_days(value: Option<i32>, label: &str) -> Result<Option<i32>, AppError> {
    match value {
        Some(days) if !(0..=MAX_DAYS).contains(&days) => Err(AppError::validation(&format!(
            "{label} debe estar entre 0 y {MAX_DAYS} días"
        ))),
        other => Ok(other),
    }
}

fn normalize(fields: SupplierFields) -> Result<SupplierFields, AppError> {
    let name = fields.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::validation("El nombre es obligatorio"));
    }
    if name.chars().count() > MAX_NAME {
        return Err(AppError::validation(&format!(
            "El nombre no puede superar {MAX_NAME} caracteres"
        )));
    }

    let document_type = optional_text(fields.document_type, "El tipo de documento", 10)?
        .map(|text| text.to_uppercase());
    let document_number = optional_text(fields.document_number, "El documento", MAX_DOC_NUMBER)?;
    match (&document_type, &document_number) {
        (Some(_), None) => {
            return Err(AppError::validation(
                "Indica el número de documento o quita el tipo",
            ))
        }
        (None, Some(_)) => return Err(AppError::validation("Elige el tipo de documento")),
        (Some(kind), Some(_)) if !DOCUMENT_TYPES.contains(&kind.as_str()) => {
            return Err(AppError::validation("El tipo de documento no es válido"))
        }
        _ => {}
    }

    let email = optional_text(fields.email, "El email", MAX_EMAIL)?;
    if let Some(value) = &email {
        let valid = value.contains('@')
            && !value.starts_with('@')
            && !value.ends_with('@')
            && !value.contains(char::is_whitespace);
        if !valid {
            return Err(AppError::validation("El email no es válido"));
        }
    }

    let priority = optional_text(fields.priority, "La prioridad", 10)?
        .map(|text| text.to_lowercase())
        .unwrap_or_else(|| "normal".to_string());
    if !PRIORITIES.contains(&priority.as_str()) {
        return Err(AppError::validation("La prioridad no es válida"));
    }

    Ok(SupplierFields {
        name,
        contact_name: optional_text(fields.contact_name, "El contacto", MAX_CONTACT)?,
        document_type,
        document_number,
        phone: optional_phone(fields.phone, "El teléfono")?,
        whatsapp: optional_phone(fields.whatsapp, "El WhatsApp")?,
        email,
        address: optional_text(fields.address, "La dirección", MAX_TEXT)?,
        city: optional_text(fields.city, "La ciudad", MAX_CITY)?,
        payment_terms_days: optional_days(fields.payment_terms_days, "El plazo de pago")?,
        lead_time_days: optional_days(fields.lead_time_days, "El tiempo de entrega")?,
        visit_days: optional_text(fields.visit_days, "Los días de visita", MAX_VISIT_DAYS)?,
        priority: Some(priority),
        notes: optional_text(fields.notes, "Las notas", MAX_TEXT)?,
    })
}

fn map_error(error: SupplierError) -> AppError {
    match error {
        SupplierError::Db(error) => AppError::from(error),
        SupplierError::NotFound => AppError::not_found("Proveedor no encontrado"),
        SupplierError::DuplicateName => {
            AppError::validation("Ya existe un proveedor con ese nombre (puede estar desactivado)")
        }
        SupplierError::DuplicateDocument => {
            AppError::validation("Ya existe un proveedor con ese documento")
        }
    }
}

fn map_link_error(error: LinkError) -> AppError {
    match error {
        LinkError::Db(error) => AppError::from(error),
        LinkError::NotFound => AppError::not_found("Proveedor o variante no encontrado"),
        LinkError::InactiveSupplier => {
            AppError::validation("No se pueden vincular variantes a un proveedor inactivo")
        }
        LinkError::DuplicateLink => {
            AppError::validation("La variante ya está vinculada a este proveedor")
        }
        LinkError::PreferredConflict => AppError::validation(
            "No se pudo asignar el proveedor preferido para esta variante; inténtalo de nuevo",
        ),
        LinkError::InvalidCategory => {
            AppError::validation("Una o más categorías no existen o están inactivas")
        }
    }
}

fn normalize_variant_texts(
    supplier_sku: Option<String>,
    notes: Option<String>,
) -> Result<(Option<String>, Option<String>), AppError> {
    Ok((
        optional_text(supplier_sku, "El SKU del proveedor", MAX_SUPPLIER_SKU)?,
        optional_text(notes, "Las notas", MAX_TEXT)?,
    ))
}

pub async fn set_supplier_categories(
    pool: &PgPool,
    supplier_id: i32,
    dto: SetSupplierCategoriesDto,
) -> Result<(), AppError> {
    links::set_supplier_categories(pool, supplier_id, dto)
        .await
        .map_err(map_link_error)
}

pub async fn list_supplier_categories(
    pool: &PgPool,
    supplier_id: i32,
) -> Result<Vec<SupplierCategory>, AppError> {
    links::list_supplier_categories(pool, supplier_id)
        .await
        .map_err(map_link_error)
}

pub async fn link_supplier_variant(
    pool: &PgPool,
    supplier_id: i32,
    mut dto: LinkSupplierVariantDto,
) -> Result<SupplierVariantLink, AppError> {
    (dto.supplier_sku, dto.notes) =
        normalize_variant_texts(dto.supplier_sku.take(), dto.notes.take())?;
    links::link_supplier_variant(pool, supplier_id, dto)
        .await
        .map_err(map_link_error)
}

pub async fn update_supplier_variant(
    pool: &PgPool,
    link_id: i32,
    mut dto: UpdateSupplierVariantDto,
) -> Result<SupplierVariantLink, AppError> {
    (dto.supplier_sku, dto.notes) =
        normalize_variant_texts(dto.supplier_sku.take(), dto.notes.take())?;
    links::update_supplier_variant(pool, link_id, dto)
        .await
        .map_err(map_link_error)
}

pub async fn unlink_supplier_variant(
    pool: &PgPool,
    link_id: i32,
    dto: UnlinkSupplierVariantDto,
) -> Result<(), AppError> {
    links::unlink_supplier_variant(pool, link_id, dto)
        .await
        .map_err(map_link_error)
}

pub async fn list_supplier_variants(
    pool: &PgPool,
    supplier_id: i32,
    mut filter: SupplierVariantFilterDto,
) -> Result<PaginatedResponse<SupplierVariantView>, AppError> {
    filter.search = optional_text(filter.search.take(), "La búsqueda", MAX_SEARCH)?;
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(20).clamp(1, 100);
    filter.page = Some(page);
    filter.page_size = Some(page_size);
    let (data, total) = links::list_supplier_variants(pool, supplier_id, &filter)
        .await
        .map_err(map_link_error)?;
    Ok(PaginatedResponse::new(data, total, page, page_size))
}

pub async fn list_variant_suppliers(
    pool: &PgPool,
    variant_id: i32,
) -> Result<Vec<VariantSupplier>, AppError> {
    links::list_variant_suppliers(pool, variant_id)
        .await
        .map_err(map_link_error)
}

pub async fn create_supplier(pool: &PgPool, dto: CreateSupplierDto) -> Result<Supplier, AppError> {
    let dto = CreateSupplierDto {
        fields: normalize(dto.fields)?,
        created_by: dto.created_by,
    };
    repository::create_supplier(pool, dto)
        .await
        .map_err(map_error)
}

pub async fn update_supplier(
    pool: &PgPool,
    id: i32,
    dto: UpdateSupplierDto,
) -> Result<Supplier, AppError> {
    let dto = UpdateSupplierDto {
        fields: normalize(dto.fields)?,
        updated_by: dto.updated_by,
    };
    repository::update_supplier(pool, id, dto)
        .await
        .map_err(map_error)
}

pub async fn set_supplier_active(
    pool: &PgPool,
    id: i32,
    dto: SetSupplierActiveDto,
) -> Result<Supplier, AppError> {
    repository::set_supplier_active(pool, id, dto)
        .await
        .map_err(map_error)
}

pub async fn get_supplier(pool: &PgPool, id: i32) -> Result<SupplierOverview, AppError> {
    repository::get_supplier(pool, id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("Proveedor no encontrado"))
}

pub async fn list_suppliers(
    pool: &PgPool,
    mut filter: SupplierFilterDto,
) -> Result<PaginatedResponse<SupplierOverview>, AppError> {
    filter.search = optional_text(filter.search.take(), "La búsqueda", MAX_SEARCH)?;
    filter.priority = optional_text(filter.priority.take(), "La prioridad", 10)?;
    if let Some(priority) = &filter.priority {
        if !PRIORITIES.contains(&priority.as_str()) {
            return Err(AppError::validation("La prioridad no es válida"));
        }
    }
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(20).clamp(1, 100);
    filter.page = Some(page);
    filter.page_size = Some(page_size);

    let (data, total) = repository::list_suppliers(pool, &filter)
        .await
        .map_err(AppError::from)?;
    Ok(PaginatedResponse::new(data, total, page, page_size))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> SupplierFields {
        SupplierFields {
            name: "  Distribuidora Norte  ".to_string(),
            contact_name: None,
            document_type: None,
            document_number: None,
            phone: None,
            whatsapp: None,
            email: None,
            address: None,
            city: None,
            payment_terms_days: None,
            lead_time_days: None,
            visit_days: None,
            priority: None,
            notes: None,
        }
    }

    #[test]
    fn name_is_required_and_trimmed() {
        let mut empty = base();
        empty.name = "   ".to_string();
        assert!(normalize(empty).is_err());
        assert_eq!(normalize(base()).unwrap().name, "Distribuidora Norte");
    }

    #[test]
    fn priority_defaults_to_normal_and_is_validated() {
        assert_eq!(
            normalize(base()).unwrap().priority.as_deref(),
            Some("normal")
        );

        let mut urgent = base();
        urgent.priority = Some(" URGENT ".to_string());
        assert_eq!(
            normalize(urgent).unwrap().priority.as_deref(),
            Some("urgent")
        );

        let mut bad = base();
        bad.priority = Some("alta".to_string());
        assert!(normalize(bad).is_err());
    }

    #[test]
    fn document_type_and_number_go_together() {
        let mut only_type = base();
        only_type.document_type = Some("NIT".to_string());
        assert!(normalize(only_type).is_err());

        let mut only_number = base();
        only_number.document_number = Some("900123".to_string());
        assert!(normalize(only_number).is_err());

        let mut bad_type = base();
        bad_type.document_type = Some("XX".to_string());
        bad_type.document_number = Some("1".to_string());
        assert!(normalize(bad_type).is_err());

        let mut ok = base();
        ok.document_type = Some("nit".to_string());
        ok.document_number = Some(" 900.123.456-7 ".to_string());
        let fields = normalize(ok).unwrap();
        assert_eq!(fields.document_type.as_deref(), Some("NIT"));
        assert_eq!(fields.document_number.as_deref(), Some("900.123.456-7"));
    }

    #[test]
    fn phones_accept_common_formats_and_reject_garbage() {
        for good in ["3012249460", "+57 301 224 9460", "(601) 234-5678"] {
            let mut fields = base();
            fields.phone = Some(good.to_string());
            fields.whatsapp = Some(good.to_string());
            assert!(normalize(fields).is_ok(), "{good}");
        }
        for bad in ["abc", "123", "301-ABC-9460", "1234567890123456"] {
            let mut fields = base();
            fields.phone = Some(bad.to_string());
            assert!(normalize(fields).is_err(), "{bad}");
        }
        let mut blank = base();
        blank.phone = Some("   ".to_string());
        assert_eq!(normalize(blank).unwrap().phone, None);
    }

    #[test]
    fn email_is_checked_and_blank_becomes_none() {
        let mut bad = base();
        bad.email = Some("sin-arroba".to_string());
        assert!(normalize(bad).is_err());

        let mut blank = base();
        blank.email = Some("  ".to_string());
        assert_eq!(normalize(blank).unwrap().email, None);

        let mut ok = base();
        ok.email = Some("ventas@norte.com".to_string());
        assert!(normalize(ok).is_ok());
    }

    #[test]
    fn day_counts_must_be_between_zero_and_a_year() {
        let mut ok = base();
        ok.payment_terms_days = Some(0);
        ok.lead_time_days = Some(365);
        assert!(normalize(ok).is_ok());

        let mut negative = base();
        negative.payment_terms_days = Some(-1);
        assert!(normalize(negative).is_err());

        let mut too_long = base();
        too_long.lead_time_days = Some(366);
        assert!(normalize(too_long).is_err());
    }

    #[test]
    fn category_ids_are_deduplicated_and_sorted() {
        assert_eq!(deduplicate_categories(vec![8, 2, 8, 3, 2]), vec![2, 3, 8]);
    }

    #[test]
    fn category_diff_reports_additions_and_removals() {
        assert_eq!(category_diff(&[1, 3, 5], &[2, 3, 5]), (vec![2], vec![1]));
        assert_eq!(category_diff(&[1, 2], &[1, 2]), (vec![], vec![]));
    }
}

pub use super::restock::{get_restock_overview, get_supplier_restock};
