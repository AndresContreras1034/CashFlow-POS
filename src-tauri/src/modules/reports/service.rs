use sqlx::PgPool;

use crate::{
    errors::app_error::AppError,
    modules::{
        reports::{
            debt_report,
            dto::{
                AgingBucket, CategorySales, CustomerSalesSplit, DashboardKpis, DateRange, Delta,
                Granularity, HeatmapCell, InventoryTurnover, InventoryValuation, PayablesSummary,
                PaymentMethodStat, RankBy, ReportFilter, StaleProduct, StockAlert, TimeSeriesPoint,
                TopCreditor, TopCustomer, TopProduct, UpcomingDue,
            },
            range::{
                change_bps, fill_buckets, granularity_for, previous_range, previous_year_range,
                MAX_RANGE_DAYS,
            },
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

    let comparison_range = if filter.compare_year_over_year {
        previous_year_range(&range)
            .filter(|candidate| (candidate.to - candidate.from).num_days() + 1 <= MAX_RANGE_DAYS)
    } else {
        Some(previous_range(&range))
    };
    let (current, previous_range, previous) = if filter.compare_previous {
        if let Some(previous_range) = comparison_range {
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
        }
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
    let range_days = (range.to - range.from).num_days() + 1;
    let granularity = match granularity_for(range_days) {
        Granularity::Day => ("day", Granularity::Day),
        Granularity::Week => ("day", Granularity::Day),
        Granularity::Month => ("month", Granularity::Month),
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

pub async fn get_top_products(
    pool: &PgPool,
    filter: ReportFilter,
) -> Result<Vec<TopProduct>, AppError> {
    filter.validate()?;
    let (timezone, _) = resolve_timezone_and_decimals(pool).await?;
    repository::fetch_top_products(
        pool,
        &DateRange {
            from: filter.date_from,
            to: filter.date_to,
        },
        &timezone,
        filter.category_id,
        filter.limit.unwrap_or(10),
        filter.rank_by.unwrap_or(RankBy::Revenue),
    )
    .await
    .map_err(AppError::from)
}

pub async fn get_sales_by_category(
    pool: &PgPool,
    filter: ReportFilter,
) -> Result<Vec<CategorySales>, AppError> {
    filter.validate()?;
    let (timezone, _) = resolve_timezone_and_decimals(pool).await?;
    repository::fetch_sales_by_category(
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

pub async fn get_top_customers(
    pool: &PgPool,
    filter: ReportFilter,
) -> Result<Vec<TopCustomer>, AppError> {
    filter.validate()?;
    let (timezone, _) = resolve_timezone_and_decimals(pool).await?;
    repository::fetch_top_customers(
        pool,
        &DateRange {
            from: filter.date_from,
            to: filter.date_to,
        },
        &timezone,
        filter.limit.unwrap_or(10).clamp(1, 100),
    )
    .await
    .map_err(AppError::from)
}

pub async fn get_customer_sales_split(
    pool: &PgPool,
    filter: ReportFilter,
) -> Result<Vec<CustomerSalesSplit>, AppError> {
    filter.validate()?;
    let (timezone, _) = resolve_timezone_and_decimals(pool).await?;
    repository::fetch_customer_sales_split(
        pool,
        &DateRange {
            from: filter.date_from,
            to: filter.date_to,
        },
        &timezone,
        filter.limit.unwrap_or(10).clamp(1, 100),
    )
    .await
    .map_err(AppError::from)
}

pub async fn get_stale_products(
    pool: &PgPool,
    filter: ReportFilter,
) -> Result<Vec<StaleProduct>, AppError> {
    filter.validate()?;
    let (timezone, _) = resolve_timezone_and_decimals(pool).await?;
    repository::fetch_stale_products(
        pool,
        &DateRange {
            from: filter.date_from,
            to: filter.date_to,
        },
        &timezone,
        filter.category_id,
        filter.limit.unwrap_or(10),
    )
    .await
    .map_err(AppError::from)
}

pub async fn get_inventory_valuation(
    pool: &PgPool,
    filter: ReportFilter,
) -> Result<InventoryValuation, AppError> {
    filter.validate()?;
    repository::fetch_inventory_valuation(pool, filter.category_id)
        .await
        .map_err(AppError::from)
}

pub async fn get_inventory_turnover(
    pool: &PgPool,
    filter: ReportFilter,
) -> Result<InventoryTurnover, AppError> {
    filter.validate()?;
    let (timezone, _) = resolve_timezone_and_decimals(pool).await?;
    let range = DateRange {
        from: filter.date_from,
        to: filter.date_to,
    };
    let range_days = (range.to - range.from).num_days() + 1;
    repository::fetch_inventory_turnover(pool, &range, &timezone, range_days, filter.category_id)
        .await
        .map_err(AppError::from)
}

pub async fn get_stock_alerts(
    pool: &PgPool,
    filter: ReportFilter,
) -> Result<Vec<StockAlert>, AppError> {
    filter.validate()?;
    let (timezone, _) = resolve_timezone_and_decimals(pool).await?;
    let range = DateRange {
        from: filter.date_from,
        to: filter.date_to,
    };
    let range_days = (range.to - range.from).num_days() + 1;
    repository::fetch_stock_alerts(
        pool,
        &range,
        &timezone,
        range_days,
        filter.category_id,
        filter.limit.unwrap_or(25),
    )
    .await
    .map_err(AppError::from)
}

pub async fn get_payables_summary(
    pool: &PgPool,
    filter: ReportFilter,
) -> Result<PayablesSummary, AppError> {
    filter.validate()?;
    debt_report::fetch_payables_summary(
        pool,
        &DateRange {
            from: filter.date_from,
            to: filter.date_to,
        },
    )
    .await
    .map_err(AppError::from)
}

pub async fn get_payables_aging(
    pool: &PgPool,
    filter: ReportFilter,
) -> Result<Vec<AgingBucket>, AppError> {
    filter.validate()?;
    debt_report::fetch_payables_aging(pool)
        .await
        .map_err(AppError::from)
}

pub async fn get_top_creditors(
    pool: &PgPool,
    filter: ReportFilter,
) -> Result<Vec<TopCreditor>, AppError> {
    filter.validate()?;
    debt_report::fetch_top_creditors(pool, filter.limit.unwrap_or(10).clamp(1, 100))
        .await
        .map_err(AppError::from)
}

/// Category filters do not apply to debts; the filter is validated for API consistency.
pub async fn get_upcoming_dues(
    pool: &PgPool,
    filter: ReportFilter,
) -> Result<Vec<UpcomingDue>, AppError> {
    filter.validate()?;
    debt_report::fetch_upcoming_dues(pool, filter.limit.unwrap_or(25).clamp(1, 100))
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
