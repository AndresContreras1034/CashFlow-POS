use sqlx::PgConnection;

use super::dto::{StocktakeCountLineDto, StocktakeReviewLineDto, StocktakeSummaryDto};
use super::models::{Stocktake, StocktakeStatus};

pub async fn query_stocktakes(
    conn: &mut PgConnection,
    id: Option<i32>,
) -> Result<Vec<Stocktake>, sqlx::Error> {
    sqlx::query_as!(
        Stocktake,
        r#"SELECT
               s.id,
               s.category_id,
               c.name AS "category_name?",
               s.status AS "status: StocktakeStatus",
               s.notes,
               s.created_by,
               s.created_at,
               s.applied_at,
               s.cancelled_at,
               (SELECT COUNT(*) FROM stocktake_lines l
                 WHERE l.stocktake_id = s.id) AS "total_lines!",
               (SELECT COUNT(*) FROM stocktake_lines l
                 WHERE l.stocktake_id = s.id AND l.counted_stock IS NOT NULL) AS "counted_lines!"
           FROM stocktakes s
           LEFT JOIN categories c ON c.id = s.category_id
           WHERE ($1::INT IS NULL OR s.id = $1)
           ORDER BY s.id DESC
           LIMIT 50"#,
        id
    )
    .fetch_all(&mut *conn)
    .await
}

pub async fn get_open_stocktake_id(conn: &mut PgConnection) -> Result<Option<i32>, sqlx::Error> {
    sqlx::query_scalar!("SELECT id FROM stocktakes WHERE status = 'counting'")
        .fetch_optional(&mut *conn)
        .await
}

pub async fn insert_stocktake(
    conn: &mut PgConnection,
    category_id: Option<i32>,
    notes: Option<String>,
    created_by: &str,
) -> Result<i32, sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO stocktakes (category_id, notes, created_by)
         VALUES ($1, $2, $3) RETURNING id",
        category_id,
        notes,
        created_by
    )
    .fetch_one(&mut *conn)
    .await
}

pub async fn insert_lines(
    conn: &mut PgConnection,
    stocktake_id: i32,
    category_id: Option<i32>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "INSERT INTO stocktake_lines (stocktake_id, variant_id, expected_stock)
         SELECT $1, v.id, v.stock
         FROM product_variants v
         JOIN products p ON p.id = v.product_id
         WHERE v.is_active = TRUE
           AND p.is_active = TRUE
           AND ($2::INT IS NULL OR p.category_id = $2)",
        stocktake_id,
        category_id
    )
    .execute(&mut *conn)
    .await?;
    Ok(result.rows_affected())
}

pub async fn lock_stocktake_status(
    conn: &mut PgConnection,
    id: i32,
) -> Result<Option<StocktakeStatus>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT status AS "status!: StocktakeStatus"
           FROM stocktakes WHERE id = $1 FOR UPDATE"#,
        id
    )
    .fetch_optional(&mut *conn)
    .await
}

pub async fn mark_applied(conn: &mut PgConnection, id: i32) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE stocktakes SET status = 'applied', applied_at = NOW() WHERE id = $1",
        id
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn mark_cancelled(conn: &mut PgConnection, id: i32) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "UPDATE stocktakes SET status = 'cancelled', cancelled_at = NOW()
         WHERE id = $1 AND status = 'counting'",
        id
    )
    .execute(&mut *conn)
    .await?;
    Ok(result.rows_affected())
}

pub async fn get_count_lines(
    conn: &mut PgConnection,
    stocktake_id: i32,
    search: Option<String>,
    only_pending: bool,
    limit: i64,
    offset: i64,
) -> Result<(Vec<StocktakeCountLineDto>, i64), sqlx::Error> {
    let rows = sqlx::query_as!(
        StocktakeCountLineDto,
        r#"SELECT
               v.id AS variant_id,
               p.name AS product_name,
               v.attributes,
               v.sku,
               v.barcode,
               l.counted_stock
           FROM stocktake_lines l
           JOIN product_variants v ON v.id = l.variant_id
           JOIN products p ON p.id = v.product_id
           WHERE l.stocktake_id = $1
             AND ($2::TEXT IS NULL
                  OR p.name ILIKE $2 OR p.brand ILIKE $2
                  OR v.sku ILIKE $2 OR v.barcode ILIKE $2)
             AND ($3::BOOL = FALSE OR l.counted_stock IS NULL)
           ORDER BY p.name, v.id
           LIMIT $4 OFFSET $5"#,
        stocktake_id,
        search,
        only_pending,
        limit,
        offset
    )
    .fetch_all(&mut *conn)
    .await?;

    let total = sqlx::query_scalar!(
        r#"SELECT COUNT(*)
           FROM stocktake_lines l
           JOIN product_variants v ON v.id = l.variant_id
           JOIN products p ON p.id = v.product_id
           WHERE l.stocktake_id = $1
             AND ($2::TEXT IS NULL
                  OR p.name ILIKE $2 OR p.brand ILIKE $2
                  OR v.sku ILIKE $2 OR v.barcode ILIKE $2)
             AND ($3::BOOL = FALSE OR l.counted_stock IS NULL)"#,
        stocktake_id,
        search,
        only_pending
    )
    .fetch_one(&mut *conn)
    .await?
    .unwrap_or(0);

    Ok((rows, total))
}

pub async fn find_count_lines_by_code(
    conn: &mut PgConnection,
    stocktake_id: i32,
    code: &str,
) -> Result<Vec<StocktakeCountLineDto>, sqlx::Error> {
    sqlx::query_as!(
        StocktakeCountLineDto,
        r#"SELECT
               v.id AS variant_id,
               p.name AS product_name,
               v.attributes,
               v.sku,
               v.barcode,
               l.counted_stock
           FROM stocktake_lines l
           JOIN product_variants v ON v.id = l.variant_id
           JOIN products p ON p.id = v.product_id
           WHERE l.stocktake_id = $1
             AND (LOWER(v.barcode) = LOWER($2) OR LOWER(v.sku) = LOWER($2))
           ORDER BY v.id"#,
        stocktake_id,
        code
    )
    .fetch_all(&mut *conn)
    .await
}

pub async fn set_count(
    conn: &mut PgConnection,
    stocktake_id: i32,
    variant_id: i32,
    counted: Option<i32>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "UPDATE stocktake_lines
         SET counted_stock = $3,
             counted_at = CASE WHEN $3::INT IS NULL THEN NULL ELSE NOW() END
         WHERE stocktake_id = $1 AND variant_id = $2",
        stocktake_id,
        variant_id,
        counted
    )
    .execute(&mut *conn)
    .await?;
    Ok(result.rows_affected())
}

pub async fn get_review_summary(
    conn: &mut PgConnection,
    stocktake_id: i32,
) -> Result<StocktakeSummaryDto, sqlx::Error> {
    sqlx::query_as!(
        StocktakeSummaryDto,
        r#"SELECT
               COUNT(*) FILTER (WHERE l.counted_stock IS NULL) AS "uncounted!",
               COUNT(*) FILTER (WHERE l.counted_stock IS NOT NULL
                                  AND l.counted_stock = v.stock) AS "matching!",
               COUNT(*) FILTER (WHERE l.counted_stock IS NOT NULL
                                  AND l.counted_stock <> v.stock) AS "with_difference!",
               COALESCE(SUM(l.counted_stock::BIGINT - v.stock::BIGINT)
                        FILTER (WHERE l.counted_stock > v.stock), 0)::BIGINT AS "surplus_units!",
               COALESCE(SUM(v.stock::BIGINT - l.counted_stock::BIGINT)
                        FILTER (WHERE l.counted_stock < v.stock), 0)::BIGINT AS "shortage_units!",
               COUNT(*) FILTER (WHERE v.stock <> l.expected_stock) AS "moved_during_count!"
           FROM stocktake_lines l
           JOIN product_variants v ON v.id = l.variant_id
           WHERE l.stocktake_id = $1"#,
        stocktake_id
    )
    .fetch_one(&mut *conn)
    .await
}

pub async fn get_review_lines(
    conn: &mut PgConnection,
    stocktake_id: i32,
) -> Result<Vec<StocktakeReviewLineDto>, sqlx::Error> {
    sqlx::query_as!(
        StocktakeReviewLineDto,
        r#"SELECT
               v.id AS variant_id,
               p.name AS product_name,
               v.attributes,
               v.sku,
               v.barcode,
               l.expected_stock,
               v.stock AS current_stock,
               l.counted_stock AS "counted_stock!",
               (l.counted_stock::BIGINT - v.stock::BIGINT) AS "difference!",
               (v.stock <> l.expected_stock) AS "moved_during_count!"
           FROM stocktake_lines l
           JOIN product_variants v ON v.id = l.variant_id
           JOIN products p ON p.id = v.product_id
           WHERE l.stocktake_id = $1
             AND l.counted_stock IS NOT NULL
             AND l.counted_stock <> v.stock
           ORDER BY p.name, v.id"#,
        stocktake_id
    )
    .fetch_all(&mut *conn)
    .await
}

pub async fn get_counted_lines(
    conn: &mut PgConnection,
    stocktake_id: i32,
) -> Result<Vec<(i32, i32)>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"SELECT variant_id, counted_stock AS "counted_stock!"
           FROM stocktake_lines
           WHERE stocktake_id = $1 AND counted_stock IS NOT NULL
           ORDER BY variant_id"#,
        stocktake_id
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| (row.variant_id, row.counted_stock))
        .collect())
}

pub async fn lock_variant_stock(
    conn: &mut PgConnection,
    variant_id: i32,
) -> Result<i32, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT stock FROM product_variants WHERE id = $1 FOR UPDATE",
        variant_id
    )
    .fetch_one(&mut *conn)
    .await
}
