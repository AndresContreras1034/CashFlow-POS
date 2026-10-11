use sqlx::PgPool;

use crate::{
    errors::app_error::AppError,
    modules::{
        reports::{
            dto::{
                DashboardKpis, DateRange, Delta, Granularity, HeatmapCell, PaymentMethodStat,
                ReportFilter, TimeSeriesPoint,
            },
            range::{change_bps, fill_buckets, previous_range},
            repository,
        },
        settings::{repository as settings_repository, service as settings_service},
    },
};

pub async fn get_dashboard_kpis(
    pool: &PgPool,
    filter: ReportFilter,
) -> Result<DashboardKpis, AppError> {
    filter.validate()?;

    let (timezone, currency_decimals) = resolve_timezone_and_decimals(pool).await?;

    let range = DateRange {
        from: filter.date_from,
        to: filter.date_to,
    };

    let (current, previous_range, previous) = if filter.compare_previous {
        let previous_range = previous_range(&range);
        let (current, previous) = tokio::try_join!(
            repository::fetch_sales_kpis(pool, &range, &timezone, currency_decimals),
            repository::fetch_sales_kpis(pool, &previous_range, &timezone, currency_decimals)
        )
        .map_err(AppError::from)?;
        (current, Some(previous_range), Some(previous))
    } else {
        let current = repository::fetch_sales_kpis(pool, &range, &timezone, currency_decimals)
            .await
            .map_err(AppError::from)?;
        (current, None, None)
    };

    let delta = |current: i64, previous: Option<i64>| Delta {
        current,
        previous,
        change_bps: previous.and_then(|previous| change_bps(current, previous)),
    };

    Ok(DashboardKpis {
        range,
        previous_range,
        gross_sales_delta: delta(
            current.gross_sales,
            previous.as_ref().map(|kpis| kpis.gross_sales),
        ),
        gross_profit_delta: delta(
            current.gross_profit,
            previous.as_ref().map(|kpis| kpis.gross_profit),
        ),
        tickets_delta: delta(current.tickets, previous.as_ref().map(|kpis| kpis.tickets)),
        avg_ticket_delta: delta(
            current.avg_ticket,
            previous.as_ref().map(|kpis| kpis.avg_ticket),
        ),
        current,
        previous,
    })
}

pub async fn get_sales_series(
    pool: &PgPool,
    filter: ReportFilter,
) -> Result<Vec<TimeSeriesPoint>, AppError> {
    filter.validate()?;
    let (timezone, _) = resolve_timezone_and_decimals(pool).await?;
    let range = DateRange {
        from: filter.date_from,
        to: filter.date_to,
    };
    let granularity = match filter.granularity {
        Some(Granularity::Day) | None => ("day", Granularity::Day),
        Some(Granularity::Week) => ("week", Granularity::Week),
        Some(Granularity::Month) => ("month", Granularity::Month),
    };
    let points = repository::fetch_sales_series(pool, &range, &timezone, granularity.0)
        .await
        .map_err(AppError::from)?;

    fill_buckets(&range, granularity.1, points)
}

pub async fn get_sales_by_payment_method(
    pool: &PgPool,
    filter: ReportFilter,
) -> Result<Vec<PaymentMethodStat>, AppError> {
    filter.validate()?;
    let (timezone, _) = resolve_timezone_and_decimals(pool).await?;
    repository::fetch_sales_by_payment_method(
        pool,
        &DateRange {
            from: filter.date_from,
            to: filter.date_to,
        },
        &timezone,
    )
    .await
    .map_err(AppError::from)
}

pub async fn get_sales_heatmap(
    pool: &PgPool,
    filter: ReportFilter,
) -> Result<Vec<HeatmapCell>, AppError> {
    filter.validate()?;
    let (timezone, _) = resolve_timezone_and_decimals(pool).await?;
    repository::fetch_sales_heatmap(
        pool,
        &DateRange {
            from: filter.date_from,
            to: filter.date_to,
        },
        &timezone,
    )
    .await
    .map_err(AppError::from)
}

async fn resolve_timezone_and_decimals(pool: &PgPool) -> Result<(String, u8), AppError> {
    let settings = settings_service::get_settings(pool).await?;
    if !settings_repository::timezone_exists(pool, &settings.timezone)
        .await
        .map_err(AppError::from)?
    {
        return Err(AppError::validation(
            "La zona horaria configurada no es válida",
        ));
    }

    Ok((settings.timezone, settings.currency_decimals as u8))
}
