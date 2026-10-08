use serde_json::{json, Value as JsonValue};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::errors::app_error::AppError;
use crate::modules::audit::{
    actor::declared_actor,
    dto::NewAuditEvent,
    models::{AuditCategory, AuditModule, AuditOutcome},
    repository as audit_repo,
};
use crate::modules::inventory::{
    dto::{PaginatedResponse, StockAdjustmentDto},
    models::MovementReason,
    repository as inventory_repo,
};

use super::dto::{
    CreateStocktakeDto, StocktakeApplyResultDto, StocktakeCountLineDto, StocktakeLineFilterDto,
    StocktakeReviewDto,
};
use super::models::{Stocktake, StocktakeStatus};
use super::repository;

async fn load(conn: &mut PgConnection, id: i32) -> Result<Stocktake, AppError> {
    repository::query_stocktakes(conn, Some(id))
        .await
        .map_err(AppError::from)?
        .into_iter()
        .next()
        .ok_or_else(|| AppError::not_found("Toma de inventario no encontrada"))
}

fn stocktake_event(
    action: &str,
    stocktake_id: i32,
    actor: Option<String>,
    summary: String,
    changes: Option<JsonValue>,
    metadata: JsonValue,
) -> NewAuditEvent {
    NewAuditEvent {
        correlation_id: Some(Uuid::new_v4()),
        category: AuditCategory::Business,
        module: AuditModule::Stocktake,
        action: action.to_string(),
        outcome: AuditOutcome::Success,
        actor,
        entity_type: Some("stocktake".to_string()),
        entity_id: Some(stocktake_id.to_string()),
        summary,
        changes,
        metadata: Some(metadata),
        error_message: None,
    }
}

pub async fn start_stocktake(
    pool: &PgPool,
    dto: CreateStocktakeDto,
) -> Result<Stocktake, AppError> {
    if let Some(category_id) = dto.category_id {
        inventory_repo::get_category_by_id(pool, category_id)
            .await
            .map_err(AppError::from)?
            .ok_or_else(|| AppError::validation("La categoría especificada no existe"))?;
    }

    let notes = dto
        .notes
        .as_deref()
        .map(str::trim)
        .filter(|notes| !notes.is_empty())
        .map(str::to_string);
    let created_by = dto
        .created_by
        .as_deref()
        .map(str::trim)
        .filter(|created_by| !created_by.is_empty())
        .unwrap_or("system");
    let actor = declared_actor(&dto.created_by);
    let notes_for_event = notes.clone();

    let mut tx = pool.begin().await.map_err(AppError::from)?;
    if repository::get_open_stocktake_id(&mut tx)
        .await
        .map_err(AppError::from)?
        .is_some()
    {
        return Err(AppError::validation(
            "Ya hay una toma de inventario en curso. Termínala o cancélala antes de iniciar otra",
        ));
    }

    let id = repository::insert_stocktake(&mut tx, dto.category_id, notes, created_by)
        .await
        .map_err(|error| {
            if error
                .as_database_error()
                .and_then(|database_error| database_error.constraint())
                == Some("one_open_stocktake")
            {
                AppError::validation("Ya hay una toma de inventario en curso")
            } else {
                AppError::from(error)
            }
        })?;
    let line_count = repository::insert_lines(&mut tx, id, dto.category_id)
        .await
        .map_err(AppError::from)?;
    if line_count == 0 {
        return Err(AppError::validation(
            "No hay variantes activas para contar en ese alcance",
        ));
    }

    audit_repo::insert_event(
        &mut *tx,
        stocktake_event(
            "start",
            id,
            actor,
            format!("Toma de inventario #{} iniciada", id),
            None,
            json!({
                "category_id": dto.category_id,
                "line_count": line_count,
                "notes": notes_for_event,
            }),
        ),
    )
    .await
    .map_err(AppError::from)?;

    tx.commit().await.map_err(AppError::from)?;
    let mut conn = pool.acquire().await.map_err(AppError::from)?;
    load(&mut conn, id).await
}

pub async fn get_current_stocktake(pool: &PgPool) -> Result<Option<Stocktake>, AppError> {
    let mut conn = pool.acquire().await.map_err(AppError::from)?;
    match repository::get_open_stocktake_id(&mut conn)
        .await
        .map_err(AppError::from)?
    {
        Some(id) => Ok(Some(load(&mut conn, id).await?)),
        None => Ok(None),
    }
}

pub async fn list_stocktakes(pool: &PgPool) -> Result<Vec<Stocktake>, AppError> {
    let mut conn = pool.acquire().await.map_err(AppError::from)?;
    repository::query_stocktakes(&mut conn, None)
        .await
        .map_err(AppError::from)
}

pub async fn list_count_lines(
    pool: &PgPool,
    stocktake_id: i32,
    filter: StocktakeLineFilterDto,
) -> Result<PaginatedResponse<StocktakeCountLineDto>, AppError> {
    let mut conn = pool.acquire().await.map_err(AppError::from)?;
    load(&mut conn, stocktake_id).await?;

    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(50).clamp(1, 200);
    let search = filter
        .search
        .as_deref()
        .map(str::trim)
        .filter(|search| !search.is_empty())
        .map(|search| format!("%{}%", search));
    let (data, total) = repository::get_count_lines(
        &mut conn,
        stocktake_id,
        search,
        filter.only_pending.unwrap_or(false),
        page_size,
        (page - 1) * page_size,
    )
    .await
    .map_err(AppError::from)?;

    Ok(PaginatedResponse::new(data, total, page, page_size))
}

pub async fn find_count_lines_by_code(
    pool: &PgPool,
    stocktake_id: i32,
    code: &str,
) -> Result<Vec<super::dto::StocktakeCountLineDto>, AppError> {
    let mut conn = pool.acquire().await.map_err(AppError::from)?;
    let stocktake = load(&mut conn, stocktake_id).await?;
    if stocktake.status != StocktakeStatus::Counting {
        return Err(AppError::validation(
            "Esta toma de inventario ya no admite conteos",
        ));
    }
    repository::find_count_lines_by_code(&mut conn, stocktake_id, code)
        .await
        .map_err(AppError::from)
}

pub async fn set_count(
    pool: &PgPool,
    stocktake_id: i32,
    variant_id: i32,
    counted_stock: Option<i32>,
) -> Result<(), AppError> {
    if counted_stock.is_some_and(|count| count < 0) {
        return Err(AppError::validation("El conteo no puede ser negativo"));
    }

    let mut tx = pool.begin().await.map_err(AppError::from)?;
    let status = repository::lock_stocktake_status(&mut tx, stocktake_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("Toma de inventario no encontrada"))?;
    if status != StocktakeStatus::Counting {
        return Err(AppError::validation(
            "Esta toma de inventario ya no admite conteos",
        ));
    }

    let rows = repository::set_count(&mut tx, stocktake_id, variant_id, counted_stock)
        .await
        .map_err(AppError::from)?;
    if rows == 0 {
        return Err(AppError::validation(
            "Esa variante no forma parte de esta toma de inventario",
        ));
    }
    tx.commit().await.map_err(AppError::from)
}

pub async fn get_review(pool: &PgPool, stocktake_id: i32) -> Result<StocktakeReviewDto, AppError> {
    let mut conn = pool.acquire().await.map_err(AppError::from)?;
    let stocktake = load(&mut conn, stocktake_id).await?;
    let summary = repository::get_review_summary(&mut conn, stocktake_id)
        .await
        .map_err(AppError::from)?;
    let lines = repository::get_review_lines(&mut conn, stocktake_id)
        .await
        .map_err(AppError::from)?;
    Ok(StocktakeReviewDto {
        stocktake,
        summary,
        lines,
    })
}

pub async fn apply_stocktake(
    pool: &PgPool,
    stocktake_id: i32,
    created_by: Option<String>,
) -> Result<StocktakeApplyResultDto, AppError> {
    let actor = declared_actor(&created_by);
    let mut tx = pool.begin().await.map_err(AppError::from)?;
    let status = repository::lock_stocktake_status(&mut tx, stocktake_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("Toma de inventario no encontrada"))?;
    if status != StocktakeStatus::Counting {
        return Err(AppError::validation(
            "Esta toma de inventario ya fue aplicada o cancelada",
        ));
    }

    let stocktake = load(&mut tx, stocktake_id).await?;
    let uncounted = (stocktake.total_lines - stocktake.counted_lines).max(0) as usize;
    let lines = repository::get_counted_lines(&mut tx, stocktake_id)
        .await
        .map_err(AppError::from)?;
    let notes = format!("Toma de inventario #{}", stocktake_id);
    let created_by = created_by
        .as_deref()
        .map(str::trim)
        .filter(|created_by| !created_by.is_empty())
        .unwrap_or("system");
    let mut adjusted = 0;
    let mut unchanged = 0;
    let mut surplus_units: i64 = 0;
    let mut shortage_units: i64 = 0;

    for (variant_id, counted_stock) in lines {
        let current_stock = repository::lock_variant_stock(&mut tx, variant_id)
            .await
            .map_err(AppError::from)?;
        if current_stock == counted_stock {
            unchanged += 1;
            continue;
        }

        inventory_repo::apply_stock_adjustment(
            &mut tx,
            StockAdjustmentDto {
                variant_id,
                actual_stock: counted_stock,
                reason: Some(MovementReason::CountCorrection),
                notes: Some(notes.clone()),
                created_by: Some(created_by.to_string()),
            },
        )
        .await
        .map_err(AppError::from)?;
        let difference = counted_stock as i64 - current_stock as i64;
        if difference > 0 {
            surplus_units += difference;
        } else {
            shortage_units -= difference;
        }
        adjusted += 1;
    }

    repository::mark_applied(&mut tx, stocktake_id)
        .await
        .map_err(AppError::from)?;

    audit_repo::insert_event(
        &mut *tx,
        stocktake_event(
            "apply",
            stocktake_id,
            actor,
            format!(
                "Toma de inventario #{} aplicada: {} ajustes",
                stocktake_id, adjusted
            ),
            Some(json!({ "status": { "from": "counting", "to": "applied" } })),
            json!({
                "category_id": stocktake.category_id,
                "category_name": stocktake.category_name,
                "total_lines": stocktake.total_lines,
                "adjusted": adjusted,
                "unchanged": unchanged,
                "uncounted": uncounted,
                "surplus_units": surplus_units,
                "shortage_units": shortage_units,
            }),
        ),
    )
    .await
    .map_err(AppError::from)?;

    tx.commit().await.map_err(AppError::from)?;

    Ok(StocktakeApplyResultDto {
        adjusted,
        unchanged,
        uncounted,
    })
}

pub async fn cancel_stocktake(pool: &PgPool, stocktake_id: i32) -> Result<(), AppError> {
    let mut tx = pool.begin().await.map_err(AppError::from)?;
    let rows = repository::mark_cancelled(&mut tx, stocktake_id)
        .await
        .map_err(AppError::from)?;
    if rows == 0 {
        return Err(AppError::validation(
            "La toma de inventario no existe o ya fue aplicada/cancelada",
        ));
    }

    let stocktake = load(&mut tx, stocktake_id).await?;

    audit_repo::insert_event(
        &mut *tx,
        stocktake_event(
            "cancel",
            stocktake_id,
            None,
            format!("Toma de inventario #{} cancelada", stocktake_id),
            Some(json!({ "status": { "from": "counting", "to": "cancelled" } })),
            json!({
                "category_id": stocktake.category_id,
                "category_name": stocktake.category_name,
                "total_lines": stocktake.total_lines,
                "counted_lines": stocktake.counted_lines,
            }),
        ),
    )
    .await
    .map_err(AppError::from)?;

    tx.commit().await.map_err(AppError::from)
}
