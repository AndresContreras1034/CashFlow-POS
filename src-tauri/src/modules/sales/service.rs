use sqlx::PgPool;

use crate::errors::app_error::AppError;
use crate::modules::sales::{
    dto::{CreateSaleDto, PaginatedResponse, SaleFilterDto},
    models::{Sale, SaleDetail},
    repository::{self, CreateSaleError},
};

pub async fn create_sale(pool: &PgPool, dto: CreateSaleDto) -> Result<SaleDetail, AppError> {
    if dto.items.is_empty() {
        return Err(AppError::validation("La venta debe tener al menos un ítem"));
    }
    if dto.payments.is_empty() {
        return Err(AppError::validation("La venta debe tener al menos un pago"));
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
            CreateSaleError::NoOpenCashSession => {
                AppError::validation("Hay un pago en efectivo pero no hay un turno de caja abierto")
            }
            CreateSaleError::PaymentMismatch => {
                AppError::validation("El total de los pagos no coincide con el total de la venta")
            }
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

pub async fn list_sales(
    pool: &PgPool,
    filter: SaleFilterDto,
) -> Result<PaginatedResponse<Sale>, AppError> {
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(20).clamp(1, 100);

    let (data, total) = repository::list_sales(pool, &filter)
        .await
        .map_err(AppError::from)?;
    Ok(PaginatedResponse::new(data, total, page, page_size))
}
