use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, QueryBuilder};

use crate::errors::app_error::AppError;

pub const DEFAULT_WINDOW_DAYS: i32 = 30;
pub const MIN_WINDOW_DAYS: i32 = 7;
pub const MAX_WINDOW_DAYS: i32 = 180;

pub const DEFAULT_COVER_DAYS: i32 = 14;
pub const MIN_COVER_DAYS: i32 = 1;
pub const MAX_COVER_DAYS: i32 = 90;

pub const DEFAULT_LEAD_TIME_THRESHOLD: i32 = 3;

/// CTE compartida: desempata el proveedor asignado a cada variante.
/// Orden de desempate: preferido > compra más reciente > menor costo > menor id.
/// alternative_suppliers expone la cantidad de OTROS proveedores activos enlazados.
pub const ASSIGNED_SUPPLIERS_CTE: &str = r#"
assigned_suppliers AS (
    SELECT DISTINCT ON (sv.variant_id)
        sv.variant_id,
        sv.supplier_id,
        sv.supplier_sku,
        sv.last_cost,
        sv.is_preferred,
        sv.last_purchase_at,
        (COUNT(*) OVER (PARTITION BY sv.variant_id) - 1)::BIGINT AS alternative_suppliers
    FROM supplier_variants sv
    JOIN suppliers s ON s.id = sv.supplier_id
    WHERE s.is_active = TRUE
    ORDER BY sv.variant_id,
             sv.is_preferred DESC,
             sv.last_purchase_at DESC NULLS LAST,
             sv.last_cost ASC NULLS LAST,
             sv.supplier_id ASC
)
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestockUrgency {
    OutOfStock,
    Critical,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SupplierContact {
    pub id: i32,
    pub name: String,
    pub contact_name: Option<String>,
    pub phone: Option<String>,
    pub whatsapp: Option<String>,
    pub email: Option<String>,
    pub priority: String,
    pub lead_time_days: Option<i32>,
    pub payment_terms_days: Option<i32>,
    pub visit_days: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestockItem {
    pub variant_id: i32,
    pub product_id: i32,
    pub product_name: String,
    pub brand: Option<String>,
    pub attributes: serde_json::Value,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub category_name: String,
    pub stock: i32,
    pub stock_min: i32,
    pub stock_status: String,
    pub units_sold_window: i64,
    pub days_of_cover: Option<i64>,
    pub urgency: RestockUrgency,
    pub suggested_qty: i64,
    pub unit_cost: Option<i64>,
    pub line_cost: Option<i64>,
    pub supplier_sku: Option<String>,
    pub alternative_suppliers: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestockGroup {
    pub supplier: Option<SupplierContact>,
    pub items: Vec<RestockItem>,
    pub item_count: i64,
    pub out_count: i64,
    pub critical_count: i64,
    pub suggested_units: i64,
    pub estimated_cost: i64,
    pub items_without_cost: i64,
    pub top_urgency: Option<RestockUrgency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestockSummary {
    pub suppliers_to_contact: i64,
    pub items_needing_restock: i64,
    pub out_of_stock_count: i64,
    pub critical_count: i64,
    pub unassigned_count: i64,
    pub suggested_units: i64,
    pub estimated_cost: i64,
    pub items_without_cost: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestockOverview {
    pub summary: RestockSummary,
    pub groups: Vec<RestockGroup>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RestockFilterDto {
    pub supplier_id: Option<i32>,
    pub include_unassigned: Option<bool>,
    pub only_urgent: Option<bool>,
    pub category_id: Option<i32>,
    pub search: Option<String>,
    pub window_days: Option<i32>,
    pub cover_days: Option<i32>,
}

#[derive(Debug, FromRow)]
struct RawRestockRow {
    variant_id: i32,
    product_id: i32,
    product_name: String,
    brand: Option<String>,
    attributes: serde_json::Value,
    sku: Option<String>,
    barcode: Option<String>,
    category_name: String,
    stock: i32,
    stock_min: i32,
    stock_status: String,
    variant_cost: i64,

    supplier_id: Option<i32>,
    supplier_name: Option<String>,
    supplier_contact_name: Option<String>,
    supplier_phone: Option<String>,
    supplier_whatsapp: Option<String>,
    supplier_email: Option<String>,
    supplier_priority: Option<String>,
    supplier_lead_time_days: Option<i32>,
    supplier_payment_terms_days: Option<i32>,
    supplier_visit_days: Option<String>,

    supplier_sku: Option<String>,
    link_last_cost: Option<i64>,
    alternative_suppliers: i64,

    units_sold_window: i64,
}

// ============================================================
// FUNCIONES PURAS
// ============================================================

/// Cantidad sugerida sin f64, con aritmética entera y división techo.
/// horizonte = lead_time_days (0 si None) + cover_days.
/// objetivo = max(stock_min * 2, ceil(vendidas * horizonte / window_days)).
/// sugerida = max(objetivo - stock, 1).
pub fn calc_suggested_qty(
    stock: i32,
    stock_min: i32,
    units_sold: i64,
    lead_time_days: Option<i32>,
    cover_days: i32,
    window_days: i32,
) -> i64 {
    let lead_time = lead_time_days.unwrap_or(0).max(0) as i64;
    let cover = cover_days.max(0) as i64;
    let horizon_days = lead_time.saturating_add(cover);

    let window = (window_days as i64).max(1);
    let sold = units_sold.max(0);

    let product = sold.saturating_mul(horizon_days);
    let demand_target = if product == 0 {
        0
    } else {
        product.saturating_add(window - 1) / window
    };

    let min_target = (stock_min as i64).saturating_mul(2);
    let target = min_target.max(demand_target);

    let stock_i64 = stock as i64;
    let diff = target.saturating_sub(stock_i64);
    diff.max(1)
}

/// Cobertura en días:
/// - floor(stock * window_days / vendidas) si vendidas > 0 y stock > 0.
/// - None si vendidas == 0.
/// - Some(0) si stock <= 0 con ventas.
pub fn calc_days_of_cover(stock: i32, window_days: i32, units_sold: i64) -> Option<i64> {
    if units_sold <= 0 {
        return None;
    }
    if stock <= 0 {
        return Some(0);
    }
    let window = (window_days as i64).max(1);
    let stock_i64 = stock as i64;
    Some(stock_i64.saturating_mul(window) / units_sold)
}

/// Urgencia por ítem:
/// - stock <= 0 -> out_of_stock.
/// - stock > 0, hay ventas y days_of_cover <= lead_time_days.unwrap_or(3) -> critical.
/// - en otro caso -> low.
pub fn calc_restock_urgency(
    stock: i32,
    units_sold: i64,
    days_of_cover: Option<i64>,
    lead_time_days: Option<i32>,
) -> RestockUrgency {
    if stock <= 0 {
        return RestockUrgency::OutOfStock;
    }
    if units_sold > 0 {
        let threshold = lead_time_days.unwrap_or(DEFAULT_LEAD_TIME_THRESHOLD) as i64;
        if let Some(cover) = days_of_cover {
            if cover <= threshold {
                return RestockUrgency::Critical;
            }
        }
    }
    RestockUrgency::Low
}

/// Costo unitario: last_cost del enlace si > 0; si no, product_variants.cost si > 0; si no, None.
pub fn choose_unit_cost(link_last_cost: Option<i64>, variant_cost: i64) -> Option<i64> {
    if let Some(cost) = link_last_cost {
        if cost > 0 {
            return Some(cost);
        }
    }
    if variant_cost > 0 {
        return Some(variant_cost);
    }
    None
}

/// Costo estimado de la línea: sugerida * costo con checked_mul.
pub fn calc_line_cost(suggested_qty: i64, unit_cost: Option<i64>) -> Result<Option<i64>, AppError> {
    match unit_cost {
        Some(cost) => {
            let line = suggested_qty.checked_mul(cost).ok_or_else(|| {
                AppError::validation("Desbordamiento al calcular el costo estimado de la línea")
            })?;
            Ok(Some(line))
        }
        None => Ok(None),
    }
}

/// Orden de ítems: urgency (out, critical, low), luego days_of_cover ascendente (None al final),
/// luego nombre de producto.
pub fn sort_items(items: &mut [RestockItem]) {
    items.sort_by(|a, b| {
        a.urgency
            .cmp(&b.urgency)
            .then_with(|| match (a.days_of_cover, b.days_of_cover) {
                (Some(da), Some(db)) => da.cmp(&db),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            })
            .then_with(|| {
                a.product_name
                    .to_lowercase()
                    .cmp(&b.product_name.to_lowercase())
            })
    });
}

fn priority_rank(priority: &str) -> i32 {
    match priority {
        "urgent" => 3,
        "preferred" => 2,
        "normal" => 1,
        _ => 0,
    }
}

/// Orden de grupos:
/// Primero los que tienen ítems out_of_stock/critical (más ítems urgentes primero),
/// luego prioridad ('urgent' > 'preferred' > 'normal'), luego nombre;
/// el grupo sin proveedor SIEMPRE al final.
pub fn sort_groups(groups: &mut [RestockGroup]) {
    groups.sort_by(|a, b| match (&a.supplier, &b.supplier) {
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
        (Some(sa), Some(sb)) => {
            let urgent_a = a.out_count + a.critical_count;
            let urgent_b = b.out_count + b.critical_count;
            urgent_b
                .cmp(&urgent_a)
                .then_with(|| priority_rank(&sb.priority).cmp(&priority_rank(&sa.priority)))
                .then_with(|| sa.name.to_lowercase().cmp(&sb.name.to_lowercase()))
        }
    });
}

/// Validación de rangos del filtro.
pub fn validate_restock_filter(filter: &RestockFilterDto) -> Result<(i32, i32), AppError> {
    let window_days = filter.window_days.unwrap_or(DEFAULT_WINDOW_DAYS);
    if !(MIN_WINDOW_DAYS..=MAX_WINDOW_DAYS).contains(&window_days) {
        return Err(AppError::validation(&format!(
            "La ventana de ventas debe estar entre {MIN_WINDOW_DAYS} y {MAX_WINDOW_DAYS} días"
        )));
    }

    let cover_days = filter.cover_days.unwrap_or(DEFAULT_COVER_DAYS);
    if !(MIN_COVER_DAYS..=MAX_COVER_DAYS).contains(&cover_days) {
        return Err(AppError::validation(&format!(
            "Los días de cobertura deben estar entre {MIN_COVER_DAYS} y {MAX_COVER_DAYS} días"
        )));
    }

    Ok((window_days, cover_days))
}

// ============================================================
// CONSULTA Y ARMADO DEL REPORTE
// ============================================================

pub async fn get_restock_overview(
    pool: &PgPool,
    filter: RestockFilterDto,
) -> Result<RestockOverview, AppError> {
    let (window_days, cover_days) = validate_restock_filter(&filter)?;

    if let Some(supplier_id) = filter.supplier_id {
        let exists = sqlx::query_scalar::<_, i32>("SELECT id FROM suppliers WHERE id = $1")
            .bind(supplier_id)
            .fetch_optional(pool)
            .await?;
        if exists.is_none() {
            return Err(AppError::not_found("Proveedor no encontrado"));
        }
    }

    let mut qb = QueryBuilder::new("WITH ");
    qb.push(ASSIGNED_SUPPLIERS_CTE);
    qb.push(
        ", sales_window AS (\
            SELECT si.variant_id, COALESCE(SUM(si.quantity), 0)::BIGINT AS units_sold \
            FROM sale_items si \
            JOIN sales s ON s.id = si.sale_id \
            WHERE s.status = 'completed' AND s.created_at >= NOW() - (",
    );
    qb.push_bind(window_days);
    qb.push(" * INTERVAL '1 day') GROUP BY si.variant_id) ");

    qb.push(
        r#"SELECT
            v.id AS variant_id,
            v.product_id,
            p.name AS product_name,
            p.brand,
            v.attributes,
            v.sku,
            v.barcode,
            c.name AS category_name,
            v.stock,
            v.stock_min,
            CASE WHEN v.stock <= 0 THEN 'out_of_stock' ELSE 'low' END AS stock_status,
            v.cost AS variant_cost,

            s.id AS supplier_id,
            s.name AS supplier_name,
            s.contact_name AS supplier_contact_name,
            s.phone AS supplier_phone,
            s.whatsapp AS supplier_whatsapp,
            s.email AS supplier_email,
            s.priority AS supplier_priority,
            s.lead_time_days AS supplier_lead_time_days,
            s.payment_terms_days AS supplier_payment_terms_days,
            s.visit_days AS supplier_visit_days,

            a.supplier_sku,
            a.last_cost AS link_last_cost,
            COALESCE(a.alternative_suppliers, 0)::BIGINT AS alternative_suppliers,

            COALESCE(sw.units_sold, 0)::BIGINT AS units_sold_window
        FROM product_variants v
        JOIN products p ON p.id = v.product_id
        JOIN categories c ON c.id = p.category_id
        LEFT JOIN assigned_suppliers a ON a.variant_id = v.id
        LEFT JOIN suppliers s ON s.id = a.supplier_id AND s.is_active = TRUE
        LEFT JOIN sales_window sw ON sw.variant_id = v.id
        WHERE v.is_active = TRUE
          AND p.is_active = TRUE
          AND v.stock <= v.stock_min"#,
    );

    if let Some(category_id) = filter.category_id {
        qb.push(" AND p.category_id = ").push_bind(category_id);
    }

    if let Some(supplier_id) = filter.supplier_id {
        qb.push(" AND s.id = ").push_bind(supplier_id);
    } else if filter.include_unassigned == Some(false) {
        qb.push(" AND s.id IS NOT NULL");
    }

    if let Some(search) = filter.search.as_deref().filter(|s| !s.trim().is_empty()) {
        let escaped = search
            .trim()
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = format!("%{escaped}%");
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

    let rows = qb.build_query_as::<RawRestockRow>().fetch_all(pool).await?;

    let only_urgent = filter.only_urgent.unwrap_or(false);

    // Agrupamiento por supplier_id (None = sin proveedor)
    let mut groups_map: BTreeMap<Option<i32>, (Option<SupplierContact>, Vec<RestockItem>)> =
        BTreeMap::new();

    for row in rows {
        let days_of_cover = calc_days_of_cover(row.stock, window_days, row.units_sold_window);
        let urgency = calc_restock_urgency(
            row.stock,
            row.units_sold_window,
            days_of_cover,
            row.supplier_lead_time_days,
        );

        if only_urgent && urgency == RestockUrgency::Low {
            continue;
        }

        let suggested_qty = calc_suggested_qty(
            row.stock,
            row.stock_min,
            row.units_sold_window,
            row.supplier_lead_time_days,
            cover_days,
            window_days,
        );

        let unit_cost = choose_unit_cost(row.link_last_cost, row.variant_cost);
        let line_cost = calc_line_cost(suggested_qty, unit_cost)?;

        let item = RestockItem {
            variant_id: row.variant_id,
            product_id: row.product_id,
            product_name: row.product_name,
            brand: row.brand,
            attributes: row.attributes,
            sku: row.sku,
            barcode: row.barcode,
            category_name: row.category_name,
            stock: row.stock,
            stock_min: row.stock_min,
            stock_status: row.stock_status,
            units_sold_window: row.units_sold_window,
            days_of_cover,
            urgency,
            suggested_qty,
            unit_cost,
            line_cost,
            supplier_sku: row.supplier_sku,
            alternative_suppliers: row.alternative_suppliers,
        };

        let entry = groups_map.entry(row.supplier_id).or_insert_with(|| {
            let contact = row.supplier_id.map(|id| SupplierContact {
                id,
                name: row.supplier_name.clone().unwrap_or_default(),
                contact_name: row.supplier_contact_name.clone(),
                phone: row.supplier_phone.clone(),
                whatsapp: row.supplier_whatsapp.clone(),
                email: row.supplier_email.clone(),
                priority: row
                    .supplier_priority
                    .clone()
                    .unwrap_or_else(|| "normal".to_string()),
                lead_time_days: row.supplier_lead_time_days,
                payment_terms_days: row.supplier_payment_terms_days,
                visit_days: row.supplier_visit_days.clone(),
            });
            (contact, Vec::new())
        });

        entry.1.push(item);
    }

    let mut groups: Vec<RestockGroup> = Vec::with_capacity(groups_map.len());
    let mut summary = RestockSummary {
        suppliers_to_contact: 0,
        items_needing_restock: 0,
        out_of_stock_count: 0,
        critical_count: 0,
        unassigned_count: 0,
        suggested_units: 0,
        estimated_cost: 0,
        items_without_cost: 0,
    };

    for (_sup_id, (supplier, mut items)) in groups_map {
        sort_items(&mut items);

        let mut out_count = 0i64;
        let mut critical_count = 0i64;
        let mut suggested_units = 0i64;
        let mut estimated_cost = 0i64;
        let mut items_without_cost = 0i64;
        let mut top_urgency: Option<RestockUrgency> = None;

        for it in &items {
            match it.urgency {
                RestockUrgency::OutOfStock => {
                    out_count += 1;
                    top_urgency = Some(RestockUrgency::OutOfStock);
                }
                RestockUrgency::Critical => {
                    critical_count += 1;
                    if top_urgency != Some(RestockUrgency::OutOfStock) {
                        top_urgency = Some(RestockUrgency::Critical);
                    }
                }
                RestockUrgency::Low => {
                    if top_urgency.is_none() {
                        top_urgency = Some(RestockUrgency::Low);
                    }
                }
            }
            suggested_units = suggested_units.saturating_add(it.suggested_qty);
            if let Some(cost) = it.line_cost {
                estimated_cost = estimated_cost.saturating_add(cost);
            } else {
                items_without_cost += 1;
            }
        }

        let item_count = items.len() as i64;
        if supplier.is_some() && item_count > 0 {
            summary.suppliers_to_contact += 1;
        }
        if supplier.is_none() {
            summary.unassigned_count += item_count;
        }

        summary.items_needing_restock += item_count;
        summary.out_of_stock_count += out_count;
        summary.critical_count += critical_count;
        summary.suggested_units = summary.suggested_units.saturating_add(suggested_units);
        summary.estimated_cost = summary.estimated_cost.saturating_add(estimated_cost);
        summary.items_without_cost += items_without_cost;

        groups.push(RestockGroup {
            supplier,
            items,
            item_count,
            out_count,
            critical_count,
            suggested_units,
            estimated_cost,
            items_without_cost,
            top_urgency,
        });
    }

    sort_groups(&mut groups);

    Ok(RestockOverview { summary, groups })
}

/// Ficha de reposición de un proveedor específico: reutiliza la misma consulta y lógica.
pub async fn get_supplier_restock(
    pool: &PgPool,
    supplier_id: i32,
    window_days: Option<i32>,
    cover_days: Option<i32>,
) -> Result<RestockGroup, AppError> {
    let filter = RestockFilterDto {
        supplier_id: Some(supplier_id),
        include_unassigned: Some(false),
        only_urgent: None,
        category_id: None,
        search: None,
        window_days,
        cover_days,
    };

    let overview = get_restock_overview(pool, filter).await?;

    if let Some(group) = overview
        .groups
        .into_iter()
        .find(|g| g.supplier.as_ref().map(|s| s.id) == Some(supplier_id))
    {
        return Ok(group);
    }

    // Si el proveedor existe pero no tiene ítems por reponer, devolver el grupo vacío
    let contact = sqlx::query_as::<_, SupplierContact>(
        "SELECT id, name, contact_name, phone, whatsapp, email, priority,
                lead_time_days, payment_terms_days, visit_days
         FROM suppliers WHERE id = $1",
    )
    .bind(supplier_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::not_found("Proveedor no encontrado"))?;

    Ok(RestockGroup {
        supplier: Some(contact),
        items: Vec::new(),
        item_count: 0,
        out_count: 0,
        critical_count: 0,
        suggested_units: 0,
        estimated_cost: 0,
        items_without_cost: 0,
        top_urgency: None,
    })
}

// ============================================================
// TESTS PUROS (OBLIGATORIOS)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_suggested_qty_without_sales() {
        // Sin ventas: objetivo = stock_min * 2.
        // stock_min = 5, stock = 2 -> objetivo = 10 -> sugerida = 10 - 2 = 8.
        let qty = calc_suggested_qty(2, 5, 0, Some(5), 14, 30);
        assert_eq!(qty, 8);

        // Sin ventas, stock negativo:
        // stock_min = 5, stock = -3 -> objetivo = 10 -> sugerida = 10 - (-3) = 13.
        let qty = calc_suggested_qty(-3, 5, 0, None, 14, 30);
        assert_eq!(qty, 13);

        // Sin ventas, stock suficiente (mayor o igual al objetivo):
        // stock_min = 2, stock = 10 -> objetivo = 4 -> sugerida = max(4 - 10, 1) = 1.
        let qty = calc_suggested_qty(10, 2, 0, None, 14, 30);
        assert_eq!(qty, 1);
    }

    #[test]
    fn test_calc_suggested_qty_with_sales_and_lead_time() {
        // horizonte = lead_time (7) + cover (14) = 21 días.
        // vendidas = 30 unidades en 30 días.
        // ceil(30 * 21 / 30) = 21.
        // stock_min = 4 -> stock_min * 2 = 8.
        // objetivo = max(8, 21) = 21.
        // stock = 3 -> sugerida = 21 - 3 = 18.
        let qty = calc_suggested_qty(3, 4, 30, Some(7), 14, 30);
        assert_eq!(qty, 18);
    }

    #[test]
    fn test_calc_suggested_qty_ceiling_division_exact_and_inexact() {
        // Inexacta: vendidas = 10, horizonte = 14, ventana = 30.
        // 10 * 14 = 140. 140 / 30 = 4.666... -> ceil = 5.
        // stock_min = 1 -> min_target = 2.
        // objetivo = max(2, 5) = 5.
        // stock = 1 -> sugerida = 5 - 1 = 4.
        let qty = calc_suggested_qty(1, 1, 10, None, 14, 30);
        assert_eq!(qty, 4);

        // Exacta: vendidas = 15, horizonte = 14, ventana = 30.
        // 15 * 14 = 210. 210 / 30 = 7.0 -> ceil = 7.
        // stock_min = 1 -> min_target = 2.
        // objetivo = 7. stock = 0 -> sugerida = 7 - 0 = 7.
        let qty = calc_suggested_qty(0, 1, 15, None, 14, 30);
        assert_eq!(qty, 7);
    }

    #[test]
    fn test_calc_suggested_qty_overflow_protection() {
        // Valores enormes no provocan pánico
        let qty = calc_suggested_qty(i32::MIN, i32::MAX, i64::MAX, Some(365), 90, 7);
        assert!(qty > 0);
    }

    #[test]
    fn test_calc_days_of_cover() {
        // Sin ventas -> None
        assert_eq!(calc_days_of_cover(10, 30, 0), None);

        // Con ventas pero stock <= 0 -> Some(0)
        assert_eq!(calc_days_of_cover(0, 30, 10), Some(0));
        assert_eq!(calc_days_of_cover(-5, 30, 10), Some(0));

        // stock = 5, window = 30, vendidas = 10 -> floor(150 / 10) = 15
        assert_eq!(calc_days_of_cover(5, 30, 10), Some(15));

        // floor inexacto: stock = 5, window = 30, vendidas = 7 -> floor(150 / 7) = 21
        assert_eq!(calc_days_of_cover(5, 30, 7), Some(21));
    }

    #[test]
    fn test_calc_restock_urgency() {
        // stock <= 0 -> out_of_stock
        assert_eq!(
            calc_restock_urgency(0, 10, Some(0), Some(5)),
            RestockUrgency::OutOfStock
        );
        assert_eq!(
            calc_restock_urgency(-1, 0, None, None),
            RestockUrgency::OutOfStock
        );

        // stock > 0, hay ventas y cover <= lead_time -> critical
        // lead_time = Some(5), cover = 4 -> critical
        assert_eq!(
            calc_restock_urgency(2, 10, Some(4), Some(5)),
            RestockUrgency::Critical
        );
        // lead_time = None (umbral default 3), cover = 2 -> critical
        assert_eq!(
            calc_restock_urgency(2, 10, Some(2), None),
            RestockUrgency::Critical
        );
        // lead_time = None, cover = 3 -> critical (cover <= 3)
        assert_eq!(
            calc_restock_urgency(2, 10, Some(3), None),
            RestockUrgency::Critical
        );

        // stock > 0, hay ventas pero cover > lead_time -> low
        assert_eq!(
            calc_restock_urgency(5, 10, Some(4), None),
            RestockUrgency::Low
        );
        assert_eq!(
            calc_restock_urgency(5, 10, Some(8), Some(5)),
            RestockUrgency::Low
        );

        // stock > 0 pero sin ventas -> low
        assert_eq!(
            calc_restock_urgency(2, 0, None, Some(5)),
            RestockUrgency::Low
        );
    }

    #[test]
    fn test_choose_unit_cost() {
        // Si last_cost del enlace > 0, se usa ese
        assert_eq!(choose_unit_cost(Some(1200), 1000), Some(1200));

        // Si last_cost es None o <= 0, se usa variant_cost si > 0
        assert_eq!(choose_unit_cost(None, 1000), Some(1000));
        assert_eq!(choose_unit_cost(Some(0), 1000), Some(1000));
        assert_eq!(choose_unit_cost(Some(-50), 1000), Some(1000));

        // Si ninguno es > 0, None
        assert_eq!(choose_unit_cost(None, 0), None);
        assert_eq!(choose_unit_cost(Some(0), 0), None);
    }

    #[test]
    fn test_calc_line_cost() {
        assert_eq!(calc_line_cost(5, Some(2000)).unwrap(), Some(10000));
        assert_eq!(calc_line_cost(5, None).unwrap(), None);

        // Desbordamiento
        assert!(calc_line_cost(i64::MAX, Some(2)).is_err());
    }

    #[test]
    fn test_sort_items() {
        let mut items = vec![
            RestockItem {
                variant_id: 1,
                product_id: 1,
                product_name: "Zeta".to_string(),
                brand: None,
                attributes: serde_json::json!({}),
                sku: None,
                barcode: None,
                category_name: "Cat".to_string(),
                stock: 2,
                stock_min: 5,
                stock_status: "low".to_string(),
                units_sold_window: 10,
                days_of_cover: Some(15),
                urgency: RestockUrgency::Low,
                suggested_qty: 8,
                unit_cost: None,
                line_cost: None,
                supplier_sku: None,
                alternative_suppliers: 0,
            },
            RestockItem {
                variant_id: 2,
                product_id: 2,
                product_name: "Beta".to_string(),
                brand: None,
                attributes: serde_json::json!({}),
                sku: None,
                barcode: None,
                category_name: "Cat".to_string(),
                stock: 0,
                stock_min: 5,
                stock_status: "out_of_stock".to_string(),
                units_sold_window: 10,
                days_of_cover: Some(0),
                urgency: RestockUrgency::OutOfStock,
                suggested_qty: 10,
                unit_cost: None,
                line_cost: None,
                supplier_sku: None,
                alternative_suppliers: 0,
            },
            RestockItem {
                variant_id: 3,
                product_id: 3,
                product_name: "Alfa".to_string(),
                brand: None,
                attributes: serde_json::json!({}),
                sku: None,
                barcode: None,
                category_name: "Cat".to_string(),
                stock: 1,
                stock_min: 5,
                stock_status: "low".to_string(),
                units_sold_window: 10,
                days_of_cover: Some(2),
                urgency: RestockUrgency::Critical,
                suggested_qty: 9,
                unit_cost: None,
                line_cost: None,
                supplier_sku: None,
                alternative_suppliers: 0,
            },
            RestockItem {
                variant_id: 4,
                product_id: 4,
                product_name: "Gama".to_string(),
                brand: None,
                attributes: serde_json::json!({}),
                sku: None,
                barcode: None,
                category_name: "Cat".to_string(),
                stock: 1,
                stock_min: 5,
                stock_status: "low".to_string(),
                units_sold_window: 0,
                days_of_cover: None,
                urgency: RestockUrgency::Low,
                suggested_qty: 9,
                unit_cost: None,
                line_cost: None,
                supplier_sku: None,
                alternative_suppliers: 0,
            },
        ];

        sort_items(&mut items);

        // 1º OutOfStock ("Beta")
        assert_eq!(items[0].product_name, "Beta");
        // 2º Critical ("Alfa")
        assert_eq!(items[1].product_name, "Alfa");
        // 3º Low con cover = 15 ("Zeta") antes de Low con cover = None ("Gama")
        assert_eq!(items[2].product_name, "Zeta");
        assert_eq!(items[3].product_name, "Gama");
    }

    #[test]
    fn test_sort_groups() {
        let make_group =
            |name: &str, priority: &str, out: i64, crit: i64, unassigned: bool| RestockGroup {
                supplier: if unassigned {
                    None
                } else {
                    Some(SupplierContact {
                        id: 1,
                        name: name.to_string(),
                        contact_name: None,
                        phone: None,
                        whatsapp: None,
                        email: None,
                        priority: priority.to_string(),
                        lead_time_days: None,
                        payment_terms_days: None,
                        visit_days: None,
                    })
                },
                items: Vec::new(),
                item_count: out + crit,
                out_count: out,
                critical_count: crit,
                suggested_units: 0,
                estimated_cost: 0,
                items_without_cost: 0,
                top_urgency: None,
            };

        let mut groups = vec![
            make_group("Sin Proveedor", "normal", 10, 10, true), // Debe quedar al final
            make_group("Prov B", "normal", 1, 0, false),         // urgente = 1
            make_group("Prov A", "urgent", 2, 1, false),         // urgente = 3
            make_group("Prov C", "preferred", 1, 0, false), // urgente = 1, prioridad preferida > normal
        ];

        sort_groups(&mut groups);

        // 1º Prov A (más urgentes: 3)
        assert_eq!(groups[0].supplier.as_ref().unwrap().name, "Prov A");
        // 2º Prov C (urgentes: 1, prioridad 'preferred')
        assert_eq!(groups[1].supplier.as_ref().unwrap().name, "Prov C");
        // 3º Prov B (urgentes: 1, prioridad 'normal')
        assert_eq!(groups[2].supplier.as_ref().unwrap().name, "Prov B");
        // 4º Sin Proveedor (siempre al final)
        assert!(groups[3].supplier.is_none());
    }

    #[test]
    fn test_validate_restock_filter_ranges() {
        // Válido por defecto
        let (w, c) = validate_restock_filter(&RestockFilterDto::default()).unwrap();
        assert_eq!(w, DEFAULT_WINDOW_DAYS);
        assert_eq!(c, DEFAULT_COVER_DAYS);

        // Límites válidos
        assert!(validate_restock_filter(&RestockFilterDto {
            window_days: Some(7),
            cover_days: Some(1),
            ..Default::default()
        })
        .is_ok());

        assert!(validate_restock_filter(&RestockFilterDto {
            window_days: Some(180),
            cover_days: Some(90),
            ..Default::default()
        })
        .is_ok());

        // Fuera de rango window_days
        assert!(validate_restock_filter(&RestockFilterDto {
            window_days: Some(6),
            ..Default::default()
        })
        .is_err());
        assert!(validate_restock_filter(&RestockFilterDto {
            window_days: Some(181),
            ..Default::default()
        })
        .is_err());

        // Fuera de rango cover_days
        assert!(validate_restock_filter(&RestockFilterDto {
            cover_days: Some(0),
            ..Default::default()
        })
        .is_err());
        assert!(validate_restock_filter(&RestockFilterDto {
            cover_days: Some(91),
            ..Default::default()
        })
        .is_err());
    }
}
