use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::app_error::AppError;
use crate::modules::audit::models::AuditEvent;
use crate::modules::debts::{
    catalog::{self, DebtOptions},
    dto::{
        CreateDebtDto, DebtFilterDto, DebtSortBy, DueDebtsFilterDto, PaginatedResponse,
        RegisterPaymentDto, SortDir, UpdateDebtDto, VoidDebtDto, VoidPaymentDto,
    },
    models::{DebtCategoryStat, DebtDetail, DebtOverview, DebtSummary, DebtUrgency},
    repository::{self, DebtError},
};
use crate::types::money::round_to_currency;

// ============================================================
// LÍMITES (alineados con las columnas de la migración 014)
// ============================================================

const MAX_CREDITOR_CHARS: usize = 150;
const MAX_CONCEPT_CHARS: usize = 200;
const MAX_SUBCATEGORY_CHARS: usize = 60;
const MAX_REFERENCE_CHARS: usize = 100;
const MAX_NOTES_CHARS: usize = 2000;
const MAX_REASON_CHARS: usize = 500;
const MAX_SEARCH_CHARS: usize = 100;

/// Tope de un importe, en centésimas (1e14 = 1 billón de unidades). Deja
/// margen para que sumar muchas deudas nunca desborde un i64.
const MAX_AMOUNT: i64 = 100_000_000_000_000;

// ============================================================
// NORMALIZACIÓN Y VALIDACIÓN (funciones puras)
// ============================================================

fn required_text(value: &str, label: &str, max: usize) -> Result<String, AppError> {
    let text = value.trim();
    if text.is_empty() {
        return Err(AppError::validation(&format!("{label} es obligatorio")));
    }
    if text.chars().count() > max {
        return Err(AppError::validation(&format!(
            "{label} no puede superar {max} caracteres"
        )));
    }
    Ok(text.to_string())
}

/// Texto opcional: sin espacios sobrantes y `None` si queda vacío.
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

/// El importe debe ser positivo, no pasar del tope y respetar la precisión
/// de la moneda. Un importe con más decimales se RECHAZA: nunca se redondea.
fn validate_amount(amount: i64, decimals: u8) -> Result<(), AppError> {
    if amount <= 0 {
        return Err(AppError::validation("El importe debe ser mayor a cero"));
    }
    if amount > MAX_AMOUNT {
        return Err(AppError::validation(
            "El importe supera el máximo permitido",
        ));
    }
    if round_to_currency(amount, decimals) != amount {
        return Err(AppError::validation(&if decimals == 0 {
            "El importe debe ser un valor entero, sin decimales".to_string()
        } else {
            format!("El importe no puede tener más de {decimals} decimales")
        }));
    }
    Ok(())
}

fn validate_key(key: Uuid) -> Result<(), AppError> {
    if key.is_nil() {
        return Err(AppError::validation(
            "La clave de la solicitud no es válida",
        ));
    }
    Ok(())
}

fn normalize_create(dto: CreateDebtDto, decimals: u8) -> Result<CreateDebtDto, AppError> {
    validate_key(dto.idempotency_key)?;
    validate_amount(dto.original_amount, decimals)?;
    Ok(CreateDebtDto {
        creditor_name: required_text(&dto.creditor_name, "El acreedor", MAX_CREDITOR_CHARS)?,
        concept: required_text(&dto.concept, "El concepto", MAX_CONCEPT_CHARS)?,
        subcategory: optional_text(dto.subcategory, "La subcategoría", MAX_SUBCATEGORY_CHARS)?,
        document_ref: optional_text(dto.document_ref, "La referencia", MAX_REFERENCE_CHARS)?,
        notes: optional_text(dto.notes, "Las observaciones", MAX_NOTES_CHARS)?,
        ..dto
    })
}

fn normalize_update(dto: UpdateDebtDto, decimals: u8) -> Result<UpdateDebtDto, AppError> {
    validate_amount(dto.original_amount, decimals)?;
    Ok(UpdateDebtDto {
        creditor_name: required_text(&dto.creditor_name, "El acreedor", MAX_CREDITOR_CHARS)?,
        concept: required_text(&dto.concept, "El concepto", MAX_CONCEPT_CHARS)?,
        subcategory: optional_text(dto.subcategory, "La subcategoría", MAX_SUBCATEGORY_CHARS)?,
        document_ref: optional_text(dto.document_ref, "La referencia", MAX_REFERENCE_CHARS)?,
        notes: optional_text(dto.notes, "Las observaciones", MAX_NOTES_CHARS)?,
        ..dto
    })
}

fn normalize_payment(
    dto: RegisterPaymentDto,
    decimals: u8,
) -> Result<RegisterPaymentDto, AppError> {
    validate_key(dto.idempotency_key)?;
    validate_amount(dto.amount, decimals)?;
    Ok(RegisterPaymentDto {
        reference: optional_text(dto.reference, "La referencia", MAX_REFERENCE_CHARS)?,
        notes: optional_text(dto.notes, "Las observaciones", MAX_NOTES_CHARS)?,
        ..dto
    })
}

fn check_range(
    label: &str,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
) -> Result<(), AppError> {
    if let (Some(from), Some(to)) = (from, to) {
        if from > to {
            return Err(AppError::validation(&format!(
                "El rango de {label}: la fecha inicial no puede ser posterior a la final"
            )));
        }
    }
    Ok(())
}

/// Deja el filtro listo para el repository: texto limpio, rangos válidos,
/// urgencias sin repetir y paginación acotada.
fn normalize_filter(mut filter: DebtFilterDto) -> Result<DebtFilterDto, AppError> {
    filter.search = optional_text(filter.search.take(), "La búsqueda", MAX_SEARCH_CHARS)?;

    check_range("emisión", filter.issued_from, filter.issued_to)?;
    check_range("vencimiento", filter.due_from, filter.due_to)?;
    check_range("pago", filter.paid_from, filter.paid_to)?;

    filter.urgencies = filter
        .urgencies
        .take()
        .map(|list| {
            let mut unique: Vec<DebtUrgency> = Vec::with_capacity(list.len());
            for urgency in list {
                if !unique.contains(&urgency) {
                    unique.push(urgency);
                }
            }
            unique
        })
        .filter(|list| !list.is_empty());

    filter.page = Some(filter.page.unwrap_or(1).max(1));
    filter.page_size = Some(filter.page_size.unwrap_or(20).clamp(1, 100));
    Ok(filter)
}

// ============================================================
// ERRORES
// ============================================================

/// Traduce los errores de negocio del repository a mensajes para el usuario.
fn map_error(error: DebtError) -> AppError {
    match error {
        DebtError::Db(error) => AppError::from(error),
        DebtError::NotFound => AppError::not_found("Deuda no encontrada"),
        DebtError::PaymentNotFound => AppError::not_found("Pago no encontrado"),
        DebtError::DebtVoided => {
            AppError::validation("La deuda está anulada y no admite cambios ni pagos")
        }
        DebtError::DebtAlreadyVoided => AppError::validation("La deuda ya está anulada"),
        DebtError::ReasonRequired => AppError::validation("El motivo es obligatorio"),
        DebtError::InvalidAmount => AppError::validation("El importe debe ser mayor a cero"),
        DebtError::IssuedInFuture => {
            AppError::validation("La fecha de emisión no puede ser futura")
        }
        DebtError::DueBeforeIssue => {
            AppError::validation("El vencimiento no puede ser anterior a la emisión")
        }
        DebtError::HasLivePayments => AppError::validation(
            "No se puede anular una deuda con pagos vigentes. Anula primero cada pago",
        ),
        DebtError::AmountChangeWithPayments => AppError::validation(
            "No se puede cambiar el importe de una deuda con pagos vigentes",
        ),
        DebtError::IssuedAfterFirstPayment => AppError::validation(
            "La fecha de emisión no puede ser posterior al primer pago vigente",
        ),
        DebtError::PaymentInFuture => {
            AppError::validation("La fecha de pago no puede ser futura")
        }
        DebtError::PaymentBeforeIssue => AppError::validation(
            "La fecha de pago no puede ser anterior a la emisión de la deuda",
        ),
        DebtError::Overpayment { .. } => {
            AppError::validation("El pago supera el saldo pendiente de la deuda")
        }
        DebtError::PaymentAlreadyVoided => AppError::validation("El pago ya está anulado"),
        DebtError::IdempotencyConflict => AppError::validation(
            "Esta solicitud ya se envió con datos distintos. Recarga el formulario e inténtalo de nuevo",
        ),
    }
}

// ============================================================
// LECTURAS
// ============================================================

async fn detail(pool: &PgPool, id: i32) -> Result<DebtDetail, AppError> {
    let debt = repository::get_overview(pool, id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("Deuda no encontrada"))?;
    let payments = repository::list_payments(pool, id)
        .await
        .map_err(AppError::from)?;
    Ok(DebtDetail { debt, payments })
}

pub async fn get_debt(pool: &PgPool, id: i32) -> Result<DebtDetail, AppError> {
    detail(pool, id).await
}

pub async fn list_debts(
    pool: &PgPool,
    filter: DebtFilterDto,
) -> Result<PaginatedResponse<DebtOverview>, AppError> {
    let filter = normalize_filter(filter)?;
    let page = filter.page.unwrap_or(1);
    let page_size = filter.page_size.unwrap_or(20);

    let (data, total) = repository::list_debts(pool, &filter)
        .await
        .map_err(AppError::from)?;
    Ok(PaginatedResponse::new(data, total, page, page_size))
}

pub async fn get_summary(pool: &PgPool, filter: DebtFilterDto) -> Result<DebtSummary, AppError> {
    let filter = normalize_filter(filter)?;
    repository::summary(pool, &filter)
        .await
        .map_err(AppError::from)
}

pub async fn get_category_stats(
    pool: &PgPool,
    filter: DebtFilterDto,
) -> Result<Vec<DebtCategoryStat>, AppError> {
    let filter = normalize_filter(filter)?;
    repository::category_stats(pool, &filter)
        .await
        .map_err(AppError::from)
}

/// Deudas que requieren atención: próximas a vencer, que vencen hoy y
/// vencidas, de la más antigua a la más lejana. No define umbrales propios:
/// usa la urgencia que calcula la base de datos.
pub async fn list_due_debts(
    pool: &PgPool,
    filter: DueDebtsFilterDto,
) -> Result<PaginatedResponse<DebtOverview>, AppError> {
    list_debts(
        pool,
        DebtFilterDto {
            urgencies: Some(vec![
                DebtUrgency::Overdue,
                DebtUrgency::DueToday,
                DebtUrgency::Alert,
            ]),
            sort_by: Some(DebtSortBy::DueOn),
            sort_dir: Some(SortDir::Asc),
            page: filter.page,
            page_size: filter.page_size,
            ..Default::default()
        },
    )
    .await
}

pub fn get_options() -> DebtOptions {
    catalog::options()
}

/// Historial de modificaciones y anulaciones de la deuda y de sus pagos.
pub async fn list_history(pool: &PgPool, debt_id: i32) -> Result<Vec<AuditEvent>, AppError> {
    // 404 explícito para una deuda inexistente, en vez de una lista vacía.
    repository::get_overview(pool, debt_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("Deuda no encontrada"))?;
    repository::list_history(pool, debt_id)
        .await
        .map_err(AppError::from)
}

// ============================================================
// ESCRITURAS
// ============================================================

pub async fn create_debt(pool: &PgPool, dto: CreateDebtDto) -> Result<DebtDetail, AppError> {
    let decimals = repository::currency_decimals(pool)
        .await
        .map_err(AppError::from)?;
    let dto = normalize_create(dto, decimals)?;

    let debt = repository::create_debt(pool, dto)
        .await
        .map_err(map_error)?;
    detail(pool, debt.id).await
}

pub async fn update_debt(
    pool: &PgPool,
    id: i32,
    dto: UpdateDebtDto,
) -> Result<DebtDetail, AppError> {
    let decimals = repository::currency_decimals(pool)
        .await
        .map_err(AppError::from)?;
    let dto = normalize_update(dto, decimals)?;

    let debt = repository::update_debt(pool, id, dto)
        .await
        .map_err(map_error)?;
    detail(pool, debt.id).await
}

pub async fn void_debt(pool: &PgPool, id: i32, dto: VoidDebtDto) -> Result<DebtDetail, AppError> {
    let dto = VoidDebtDto {
        reason: required_text(&dto.reason, "El motivo", MAX_REASON_CHARS)?,
        ..dto
    };

    let debt = repository::void_debt(pool, id, dto)
        .await
        .map_err(map_error)?;
    detail(pool, debt.id).await
}

pub async fn register_payment(
    pool: &PgPool,
    debt_id: i32,
    dto: RegisterPaymentDto,
) -> Result<DebtDetail, AppError> {
    let decimals = repository::currency_decimals(pool)
        .await
        .map_err(AppError::from)?;
    let dto = normalize_payment(dto, decimals)?;

    repository::register_payment(pool, debt_id, dto)
        .await
        .map_err(map_error)?;
    detail(pool, debt_id).await
}

pub async fn void_payment(
    pool: &PgPool,
    payment_id: i32,
    dto: VoidPaymentDto,
) -> Result<DebtDetail, AppError> {
    let dto = VoidPaymentDto {
        reason: required_text(&dto.reason, "El motivo", MAX_REASON_CHARS)?,
        ..dto
    };

    let payment = repository::void_payment(pool, payment_id, dto)
        .await
        .map_err(map_error)?;
    detail(pool, payment.debt_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::debts::models::{DebtCategory, DebtNature, DebtPaymentMethod};

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, d).unwrap()
    }

    fn create_dto() -> CreateDebtDto {
        CreateDebtDto {
            idempotency_key: Uuid::new_v4(),
            creditor_name: "  Ferretería Central  ".to_string(),
            category: DebtCategory::Maintenance,
            subcategory: Some("   ".to_string()),
            nature: DebtNature::Operating,
            concept: " Techo ".to_string(),
            original_amount: 100_000,
            issued_on: day(1),
            due_on: None,
            document_ref: Some(" FAC-1 ".to_string()),
            notes: Some("".to_string()),
            created_by: None,
        }
    }

    fn message(error: AppError) -> String {
        error.to_string()
    }

    #[test]
    fn texts_are_trimmed_and_blank_optionals_become_none() {
        let dto = normalize_create(create_dto(), 0).unwrap();
        assert_eq!(dto.creditor_name, "Ferretería Central");
        assert_eq!(dto.concept, "Techo");
        assert_eq!(dto.subcategory, None);
        assert_eq!(dto.notes, None);
        assert_eq!(dto.document_ref.as_deref(), Some("FAC-1"));
    }

    #[test]
    fn required_texts_and_lengths_are_enforced() {
        let mut dto = create_dto();
        dto.creditor_name = "   ".to_string();
        assert_eq!(
            message(normalize_create(dto, 0).unwrap_err()),
            "El acreedor es obligatorio"
        );

        let mut dto = create_dto();
        dto.concept = "x".repeat(201);
        assert!(message(normalize_create(dto, 0).unwrap_err()).contains("200"));

        let mut dto = create_dto();
        dto.concept = "ñ".repeat(200); // 200 caracteres, 400 bytes: válido
        assert!(normalize_create(dto, 0).is_ok());

        let mut dto = create_dto();
        dto.subcategory = Some("s".repeat(61));
        assert!(normalize_create(dto, 0).is_err());

        let mut dto = create_dto();
        dto.notes = Some("n".repeat(2001));
        assert!(normalize_create(dto, 0).is_err());

        let mut dto = create_dto();
        dto.idempotency_key = Uuid::nil();
        assert!(normalize_create(dto, 0).is_err());
    }

    #[test]
    fn amounts_are_validated_against_currency_precision_without_rounding() {
        // COP (0 decimales): solo pesos enteros (múltiplos de 100 centésimas).
        assert!(validate_amount(150_000, 0).is_ok());
        assert_eq!(
            message(validate_amount(150_050, 0).unwrap_err()),
            "El importe debe ser un valor entero, sin decimales"
        );
        assert!(validate_amount(150_001, 0).is_err());
        // USD (2 decimales): cualquier centésima es válida.
        assert!(validate_amount(150_050, 2).is_ok());
        assert!(validate_amount(1, 2).is_ok());
    }

    #[test]
    fn amounts_must_be_positive_and_bounded() {
        assert!(validate_amount(0, 2).is_err());
        assert!(validate_amount(-100, 0).is_err());
        assert!(validate_amount(MAX_AMOUNT, 0).is_ok());
        assert!(validate_amount(MAX_AMOUNT + 100, 0).is_err());
        assert!(validate_amount(i64::MAX, 2).is_err());
    }

    #[test]
    fn payment_normalization_checks_amount_and_texts() {
        let dto = RegisterPaymentDto {
            idempotency_key: Uuid::new_v4(),
            amount: 100_050,
            paid_on: day(2),
            method: DebtPaymentMethod::Cash,
            reference: Some("  ".to_string()),
            notes: Some(" ok ".to_string()),
            created_by: None,
        };
        assert!(normalize_payment(dto.clone(), 0).is_err());
        let normalized = normalize_payment(dto, 2).unwrap();
        assert_eq!(normalized.reference, None);
        assert_eq!(normalized.notes.as_deref(), Some("ok"));
    }

    #[test]
    fn filter_ranges_are_validated_independently() {
        let ok = DebtFilterDto {
            issued_from: Some(day(1)),
            issued_to: Some(day(1)),
            due_from: Some(day(5)),
            ..Default::default()
        };
        assert!(normalize_filter(ok).is_ok());

        for bad in [
            DebtFilterDto {
                issued_from: Some(day(9)),
                issued_to: Some(day(2)),
                ..Default::default()
            },
            DebtFilterDto {
                due_from: Some(day(9)),
                due_to: Some(day(2)),
                ..Default::default()
            },
            DebtFilterDto {
                paid_from: Some(day(9)),
                paid_to: Some(day(2)),
                ..Default::default()
            },
        ] {
            assert!(normalize_filter(bad).is_err());
        }
    }

    #[test]
    fn filter_is_cleaned_and_pagination_is_bounded() {
        let filter = normalize_filter(DebtFilterDto {
            search: Some("  caja  ".to_string()),
            urgencies: Some(vec![
                DebtUrgency::Alert,
                DebtUrgency::Alert,
                DebtUrgency::Overdue,
            ]),
            page: Some(-4),
            page_size: Some(10_000),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(filter.search.as_deref(), Some("caja"));
        assert_eq!(
            filter.urgencies,
            Some(vec![DebtUrgency::Alert, DebtUrgency::Overdue])
        );
        assert_eq!((filter.page, filter.page_size), (Some(1), Some(100)));

        let empty = normalize_filter(DebtFilterDto {
            search: Some("   ".to_string()),
            urgencies: Some(vec![]),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(empty.search, None);
        assert_eq!(empty.urgencies, None);
        assert_eq!((empty.page, empty.page_size), (Some(1), Some(20)));

        assert!(normalize_filter(DebtFilterDto {
            search: Some("x".repeat(101)),
            ..Default::default()
        })
        .is_err());
    }

    #[test]
    fn business_errors_map_to_validation_and_not_found_not_to_technical_errors() {
        // `finish` solo audita Database/Internal: estos NO deben serlo.
        let business = [
            DebtError::DebtVoided,
            DebtError::DebtAlreadyVoided,
            DebtError::ReasonRequired,
            DebtError::InvalidAmount,
            DebtError::IssuedInFuture,
            DebtError::DueBeforeIssue,
            DebtError::HasLivePayments,
            DebtError::AmountChangeWithPayments,
            DebtError::IssuedAfterFirstPayment,
            DebtError::PaymentInFuture,
            DebtError::PaymentBeforeIssue,
            DebtError::Overpayment {
                balance: 1,
                requested: 2,
            },
            DebtError::PaymentAlreadyVoided,
            DebtError::IdempotencyConflict,
        ];
        for error in business {
            assert!(matches!(map_error(error), AppError::Validation(_)));
        }
        assert!(matches!(
            map_error(DebtError::NotFound),
            AppError::NotFound(_)
        ));
        assert!(matches!(
            map_error(DebtError::PaymentNotFound),
            AppError::NotFound(_)
        ));
        assert!(matches!(
            map_error(DebtError::Db(sqlx::Error::PoolTimedOut)),
            AppError::Database(_)
        ));
    }
}
