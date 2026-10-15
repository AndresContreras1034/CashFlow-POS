use sqlx::PgPool;

use crate::errors::app_error::AppError;
use crate::modules::sales::{
    dto::{AssignSaleCustomerDto, CreateSaleDto, SaleFilterDto, SaleListResponse, VoidSaleDto},
    models::{SaleDetail, SaleStatus},
    repository::{self, AssignCustomerError, CreateSaleError, VoidSaleError},
};

pub async fn create_sale(pool: &PgPool, dto: CreateSaleDto) -> Result<SaleDetail, AppError> {
    if dto.items.is_empty() {
        return Err(AppError::validation("La venta debe tener al menos un ítem"));
    }
    if dto.items.iter().any(|i| i.quantity <= 0) {
        return Err(AppError::validation(
            "La cantidad de cada ítem debe ser mayor a cero",
        ));
    }
    if dto.payments.iter().any(|p| p.amount <= 0) {
        return Err(AppError::validation(
            "El monto de cada pago debe ser mayor a cero",
        ));
    }

    repository::create_sale(pool, dto)
        .await
        .map_err(|e| match e {
            CreateSaleError::Db(err) => AppError::from(err),
            CreateSaleError::NoOpenCashSession => AppError::validation(
                "No hay un turno de caja abierto. Abre la caja para registrar ventas.",
            ),
            CreateSaleError::CustomerNotFound => {
                AppError::validation("El cliente seleccionado no existe")
            }
            CreateSaleError::CustomerInactive => {
                AppError::validation("El cliente seleccionado está desactivado")
            }
            CreateSaleError::CourtesyReasonRequired => {
                AppError::validation("La venta de cortesía requiere un motivo")
            }
            CreateSaleError::CourtesyReasonTooLong => {
                AppError::validation("El motivo de cortesía no puede superar 200 caracteres")
            }
            CreateSaleError::CourtesyZeroGross => AppError::validation(
                "Una venta de cortesía debe tener un importe bruto mayor a cero",
            ),
            CreateSaleError::CourtesyHasPayments => {
                AppError::validation("Una venta de cortesía no puede incluir pagos")
            }
            CreateSaleError::NoPayments => {
                AppError::validation("La venta debe tener al menos un pago")
            }
            CreateSaleError::PaymentMismatch => {
                AppError::validation("El total de los pagos no coincide con el total de la venta")
            }
            CreateSaleError::CashReceivedWithoutCash => {
                AppError::validation("El efectivo recibido requiere una venta con pago en efectivo")
            }
            CreateSaleError::CashReceivedTooLow => {
                AppError::validation("El efectivo recibido no puede ser menor al pago en efectivo")
            }
            CreateSaleError::CashReceivedPrecision => AppError::validation(
                "El efectivo recibido no admite fracciones con la moneda configurada",
            ),
            CreateSaleError::InvalidDiscount => AppError::validation(
                "El descuento no puede ser negativo ni superar el subtotal de la venta",
            ),
            CreateSaleError::AmountOverflow => {
                AppError::validation("El importe de la venta supera el rango permitido")
            }
            CreateSaleError::VariantNotFound => {
                AppError::validation("La variante solicitada no existe")
            }
            CreateSaleError::VariantUnavailable => {
                AppError::validation("La variante no está disponible para la venta")
            }
            CreateSaleError::InsufficientStock {
                variant_id,
                available,
                requested,
            } => AppError::validation(&format!(
                "Stock insuficiente para la variante {}. Disponible: {}, solicitado: {}",
                variant_id, available, requested
            )),
        })
}

pub async fn get_sale(pool: &PgPool, id: i32) -> Result<SaleDetail, AppError> {
    repository::get_sale_by_id(pool, id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("Venta no encontrada"))
}

pub async fn assign_customer(
    pool: &PgPool,
    id: i32,
    dto: AssignSaleCustomerDto,
) -> Result<SaleDetail, AppError> {
    repository::assign_customer(pool, id, dto)
        .await
        .map_err(|error| match error {
            AssignCustomerError::Db(db_error) => AppError::from(db_error),
            AssignCustomerError::SaleNotFound => AppError::not_found("La venta no existe"),
            AssignCustomerError::CustomerNotFound => {
                AppError::validation("El cliente seleccionado no existe")
            }
            AssignCustomerError::CustomerInactive => {
                AppError::validation("El cliente seleccionado está desactivado")
            }
        })
}

pub async fn void_sale(
    pool: &PgPool,
    id: i32,
    mut dto: VoidSaleDto,
) -> Result<SaleDetail, AppError> {
    if !matches!(
        dto.target_status,
        SaleStatus::Cancelled | SaleStatus::Refunded
    ) {
        return Err(AppError::validation(
            "El estado destino debe ser anulación o devolución",
        ));
    }
    dto.reason = dto.reason.trim().to_string();
    let reason_length = dto.reason.chars().count();
    if !(1..=200).contains(&reason_length) {
        return Err(AppError::validation(
            "El motivo debe tener entre 1 y 200 caracteres",
        ));
    }

    repository::void_sale(pool, id, dto)
        .await
        .map_err(|error| match error {
            VoidSaleError::Db(db_error) => AppError::from(db_error),
            VoidSaleError::NotFound => AppError::not_found("La venta no existe"),
            VoidSaleError::NotCompleted(_) => {
                AppError::validation("La venta ya está anulada/devuelta")
            }
            VoidSaleError::CashSessionRequired => AppError::validation(
                "Para devolver efectivo hay que tener un turno de caja abierto",
            ),
        })
}

pub async fn list_sales(
    pool: &PgPool,
    filter: SaleFilterDto,
) -> Result<SaleListResponse, AppError> {
    if filter
        .payment_method
        .as_deref()
        .is_some_and(|method| !matches!(method, "cash" | "card" | "transfer"))
    {
        return Err(AppError::validation(
            "El método de pago debe ser cash, card o transfer",
        ));
    }
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(20).clamp(1, 100);

    let (data, total, total_amount) = repository::list_sales(pool, &filter)
        .await
        .map_err(AppError::from)?;
    Ok(SaleListResponse::new(
        data,
        total,
        page,
        page_size,
        total_amount,
    ))
}
