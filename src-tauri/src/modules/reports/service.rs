use sqlx::PgPool;

use crate::{
    errors::app_error::AppError,
    modules::{
        reports::{
            dto::{DashboardKpis, DateRange, Delta, ReportFilter},
            range::{change_bps, previous_range},
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

    let settings = settings_service::get_settings(pool).await?;
    if !settings_repository::timezone_exists(pool, &settings.timezone)
        .await
        .map_err(AppError::from)?
    {
        return Err(AppError::validation(
            "La zona horaria configurada no es válida",
        ));
    }

    let range = DateRange {
        from: filter.date_from,
        to: filter.date_to,
    };

    let (current, previous_range, previous) = if filter.compare_previous {
        let previous_range = previous_range(&range);
        let (current, previous) = tokio::try_join!(
            repository::fetch_sales_kpis(
                pool,
                &range,
                &settings.timezone,
                settings.currency_decimals as u8
            ),
            repository::fetch_sales_kpis(
                pool,
                &previous_range,
                &settings.timezone,
                settings.currency_decimals as u8
            )
        )
        .map_err(AppError::from)?;
        (current, Some(previous_range), Some(previous))
    } else {
        let current = repository::fetch_sales_kpis(
            pool,
            &range,
            &settings.timezone,
            settings.currency_decimals as u8,
        )
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
