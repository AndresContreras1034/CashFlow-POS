use std::collections::HashSet;

use chrono::Local;
use rust_xlsxwriter::{Color, Format, Workbook, Worksheet, XlsxError};
use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::errors::app_error::AppError;
use crate::modules::inventory::dto::{ExportSummaryDto, ExportVariantRow, InventoryValueDto};
use crate::modules::inventory::repository;

/// Columnas A-L: deben conservar el orden que espera `import.rs::parse_workbook`.
pub const IMPORT_HEADERS: [&str; 12] = [
    "Producto",
    "Marca",
    "Categoría",
    "Descripción",
    "SKU",
    "Código de barras",
    "Precio",
    "Costo",
    "Stock",
    "Stock mínimo",
    "Atributos",
    "Permitir stock negativo",
];

const INFO_HEADERS: [&str; 5] = [
    "Estado del stock",
    "Margen",
    "Valor a costo",
    "Valor potencial de venta",
    "ID variante",
];

const VARIANT_WIDTHS: [f64; 17] = [
    30.0, 16.0, 18.0, 30.0, 16.0, 20.0, 12.0, 12.0, 10.0, 14.0, 34.0, 22.0, 16.0, 12.0, 16.0, 22.0,
    12.0,
];

fn xl(error: XlsxError) -> AppError {
    AppError::internal(&format!("No se pudo generar el Excel: {}", error))
}

fn pesos(centavos: i64) -> f64 {
    centavos as f64 / 100.0
}

pub fn format_attributes(value: &JsonValue) -> String {
    match value {
        JsonValue::Object(map) => map
            .iter()
            .map(|(key, value)| {
                let value = match value {
                    JsonValue::String(text) => text.clone(),
                    other => other.to_string(),
                };
                format!("{}: {}", key, value)
            })
            .collect::<Vec<_>>()
            .join("; "),
        _ => String::new(),
    }
}

fn stock_label(stock: i32, stock_min: i32) -> &'static str {
    if stock <= 0 {
        "Agotado"
    } else if stock <= stock_min {
        "Bajo"
    } else {
        "Normal"
    }
}

struct Formats {
    header: Format,
    money: Format,
    percent: Format,
}

fn formats() -> Formats {
    Formats {
        header: Format::new()
            .set_bold()
            .set_background_color(Color::RGB(0xD9E1F2)),
        money: Format::new().set_num_format("#,##0"),
        percent: Format::new().set_num_format("0.0%"),
    }
}

fn write_headers(
    worksheet: &mut Worksheet,
    headers: &[&str],
    format: &Format,
) -> Result<(), XlsxError> {
    for (column, header) in headers.iter().enumerate() {
        worksheet.write_string_with_format(0, column as u16, *header, format)?;
    }
    Ok(())
}

fn write_optional(
    worksheet: &mut Worksheet,
    row: u32,
    column: u16,
    value: &Option<String>,
) -> Result<(), XlsxError> {
    if let Some(value) = value {
        worksheet.write_string(row, column, value)?;
    }
    Ok(())
}

fn variants_sheet(
    name: &str,
    rows: &[&ExportVariantRow],
    formats: &Formats,
) -> Result<Worksheet, XlsxError> {
    let mut worksheet = Worksheet::new();
    worksheet.set_name(name)?;

    let headers: Vec<&str> = IMPORT_HEADERS
        .iter()
        .chain(INFO_HEADERS.iter())
        .copied()
        .collect();
    write_headers(&mut worksheet, &headers, &formats.header)?;

    for (index, variant) in rows.iter().enumerate() {
        let row = (index + 1) as u32;

        worksheet.write_string(row, 0, &variant.product_name)?;
        write_optional(&mut worksheet, row, 1, &variant.brand)?;
        worksheet.write_string(row, 2, &variant.category_name)?;
        write_optional(&mut worksheet, row, 3, &variant.description)?;
        write_optional(&mut worksheet, row, 4, &variant.sku)?;
        write_optional(&mut worksheet, row, 5, &variant.barcode)?;
        worksheet.write_number_with_format(row, 6, pesos(variant.price), &formats.money)?;
        worksheet.write_number_with_format(row, 7, pesos(variant.cost), &formats.money)?;
        worksheet.write_number(row, 8, variant.stock as f64)?;
        worksheet.write_number(row, 9, variant.stock_min as f64)?;
        worksheet.write_string(row, 10, format_attributes(&variant.attributes))?;
        worksheet.write_string(row, 11, if variant.allow_negative { "Sí" } else { "No" })?;

        worksheet.write_string(row, 12, stock_label(variant.stock, variant.stock_min))?;
        if variant.price <= 0 {
            worksheet.write_string(row, 13, "Sin precio")?;
        } else if variant.cost <= 0 {
            worksheet.write_string(row, 13, "Sin costo")?;
        } else {
            let margin = (variant.price - variant.cost) as f64 / variant.price as f64;
            worksheet.write_number_with_format(row, 13, margin, &formats.percent)?;
        }

        if variant.cost > 0 {
            let stock = variant.stock.max(0) as f64;
            worksheet.write_number_with_format(
                row,
                14,
                stock * pesos(variant.cost),
                &formats.money,
            )?;
            worksheet.write_number_with_format(
                row,
                15,
                stock * pesos(variant.price),
                &formats.money,
            )?;
        }

        worksheet.write_number(row, 16, variant.variant_id as f64)?;
    }

    worksheet.set_freeze_panes(1, 0)?;
    if !rows.is_empty() {
        worksheet.autofilter(0, 0, rows.len() as u32, (headers.len() - 1) as u16)?;
    }
    for (column, width) in VARIANT_WIDTHS.iter().enumerate() {
        worksheet.set_column_width(column as u16, *width)?;
    }
    Ok(worksheet)
}

fn low_stock_sheet(rows: &[&ExportVariantRow], formats: &Formats) -> Result<Worksheet, XlsxError> {
    let mut worksheet = Worksheet::new();
    worksheet.set_name("STOCK BAJO")?;
    let headers = [
        "Producto",
        "Atributos",
        "SKU",
        "Código de barras",
        "Estado",
        "Stock",
        "Stock mínimo",
        "Faltante hasta el mínimo",
    ];
    write_headers(&mut worksheet, &headers, &formats.header)?;

    for (index, variant) in rows.iter().enumerate() {
        let row = (index + 1) as u32;
        worksheet.write_string(row, 0, &variant.product_name)?;
        worksheet.write_string(row, 1, format_attributes(&variant.attributes))?;
        write_optional(&mut worksheet, row, 2, &variant.sku)?;
        write_optional(&mut worksheet, row, 3, &variant.barcode)?;
        worksheet.write_string(row, 4, stock_label(variant.stock, variant.stock_min))?;
        worksheet.write_number(row, 5, variant.stock as f64)?;
        worksheet.write_number(row, 6, variant.stock_min as f64)?;
        worksheet.write_number(row, 7, (variant.stock_min - variant.stock).max(0) as f64)?;
    }

    worksheet.set_freeze_panes(1, 0)?;
    if !rows.is_empty() {
        worksheet.autofilter(0, 0, rows.len() as u32, (headers.len() - 1) as u16)?;
    }
    for (column, width) in [30.0, 34.0, 16.0, 20.0, 12.0, 10.0, 14.0, 24.0]
        .iter()
        .enumerate()
    {
        worksheet.set_column_width(column as u16, *width)?;
    }
    Ok(worksheet)
}

fn write_text(
    worksheet: &mut Worksheet,
    row: u32,
    label: &str,
    value: &str,
) -> Result<(), XlsxError> {
    worksheet.write_string(row, 0, label)?;
    worksheet.write_string(row, 1, value)?;
    Ok(())
}

fn write_count(
    worksheet: &mut Worksheet,
    row: u32,
    label: &str,
    value: usize,
) -> Result<(), XlsxError> {
    worksheet.write_string(row, 0, label)?;
    worksheet.write_number(row, 1, value as f64)?;
    Ok(())
}

fn write_money(
    worksheet: &mut Worksheet,
    row: u32,
    label: &str,
    centavos: i64,
    format: &Format,
) -> Result<(), XlsxError> {
    worksheet.write_string(row, 0, label)?;
    worksheet.write_number_with_format(row, 1, pesos(centavos), format)?;
    Ok(())
}

fn summary_sheet(
    value: &InventoryValueDto,
    active_variants: usize,
    inactive_variants: usize,
    active_products: usize,
    formats: &Formats,
) -> Result<Worksheet, XlsxError> {
    let mut worksheet = Worksheet::new();
    worksheet.set_name("RESUMEN")?;
    worksheet.write_string_with_format(0, 0, "Concepto", &formats.header)?;
    worksheet.write_string_with_format(0, 1, "Valor", &formats.header)?;

    let exported_at = Local::now().format("%Y-%m-%d %H:%M").to_string();
    write_text(&mut worksheet, 1, "Fecha de exportación", &exported_at)?;
    write_money(
        &mut worksheet,
        2,
        "Valor del inventario (a costo)",
        value.value_at_cost,
        &formats.money,
    )?;
    write_money(
        &mut worksheet,
        3,
        "Valor potencial de venta",
        value.potential_sale_value,
        &formats.money,
    )?;
    write_count(
        &mut worksheet,
        4,
        "Variantes valoradas",
        value.variants_valued as usize,
    )?;
    write_count(
        &mut worksheet,
        5,
        "Variantes sin costo (excluidas del valor)",
        value.variants_without_cost as usize,
    )?;
    write_count(
        &mut worksheet,
        6,
        "Variantes con stock negativo (aportan 0)",
        value.negative_stock_variants as usize,
    )?;
    write_count(&mut worksheet, 7, "Variantes activas", active_variants)?;
    write_count(&mut worksheet, 8, "Variantes inactivas", inactive_variants)?;
    write_count(&mut worksheet, 9, "Productos activos", active_products)?;
    worksheet.set_column_width(0, 44)?;
    worksheet.set_column_width(1, 22)?;
    Ok(worksheet)
}

fn build_workbook(
    rows: &[ExportVariantRow],
    value: &InventoryValueDto,
    path: &str,
) -> Result<ExportSummaryDto, AppError> {
    let formats = formats();
    let active: Vec<&ExportVariantRow> = rows.iter().filter(|row| row.is_active).collect();
    let inactive: Vec<&ExportVariantRow> = rows.iter().filter(|row| !row.is_active).collect();

    let mut low_stock: Vec<&ExportVariantRow> = active
        .iter()
        .copied()
        .filter(|row| row.stock <= row.stock_min)
        .collect();
    low_stock.sort_by(|left, right| {
        left.stock
            .cmp(&right.stock)
            .then_with(|| left.product_name.cmp(&right.product_name))
    });

    let active_products = active
        .iter()
        .map(|row| row.product_id)
        .collect::<HashSet<_>>()
        .len();
    let mut workbook = Workbook::new();
    workbook.push_worksheet(variants_sheet("PRODUCTOS", &active, &formats).map_err(xl)?);
    workbook.push_worksheet(variants_sheet("INACTIVOS", &inactive, &formats).map_err(xl)?);
    workbook.push_worksheet(low_stock_sheet(&low_stock, &formats).map_err(xl)?);
    workbook.push_worksheet(
        summary_sheet(
            value,
            active.len(),
            inactive.len(),
            active_products,
            &formats,
        )
        .map_err(xl)?,
    );
    workbook.save(path).map_err(xl)?;

    Ok(ExportSummaryDto {
        active_rows: active.len(),
        inactive_rows: inactive.len(),
        low_stock_rows: low_stock.len(),
    })
}

pub async fn export_inventory(
    pool: &PgPool,
    file_path: &str,
) -> Result<ExportSummaryDto, AppError> {
    let rows = repository::get_export_rows(pool)
        .await
        .map_err(AppError::from)?;
    let value = repository::get_inventory_value(pool)
        .await
        .map_err(AppError::from)?;

    let mut path = file_path.to_string();
    if !path.to_lowercase().ends_with(".xlsx") {
        path.push_str(".xlsx");
    }

    tokio::task::spawn_blocking(move || build_workbook(&rows, &value, &path))
        .await
        .map_err(|error| AppError::internal(&format!("Falló la exportación: {}", error)))?
}

const TEMPLATE_INSTRUCTIONS: [(&str, &str, &str); 12] = [
    (
        "Sí",
        "Nombre del producto. Si ya existe, la fila agrega una variante.",
        "Perfume Rosa",
    ),
    ("No", "Marca del producto.", "Aroma"),
    ("Sí", "Se crea automáticamente si no existe.", "Perfumes"),
    ("No", "Texto libre.", "Fragancia floral"),
    (
        "No",
        "Código interno único. Si ya existe, la fila se omite.",
        "ROS-100",
    ),
    (
        "No",
        "Debe ser único. Si ya existe, la fila se marca como error.",
        "7501234000011",
    ),
    ("Sí", "En pesos, sin separador de miles.", "28000"),
    ("No", "En pesos. Vacío equivale a sin costo.", "15000"),
    (
        "Sí",
        "Número entero, 0 o mayor. Mayor que 0 registra stock inicial.",
        "10",
    ),
    ("No", "Número entero. Vacío equivale a 0.", "3"),
    (
        "Recomendado",
        "«Clave: Valor», varios separados por «;». Evita repetir atributos en el producto.",
        "Volumen: 100ml; Color: Rosa",
    ),
    ("No", "Sí o No. Vacío equivale a No.", "No"),
];

const TEMPLATE_NOTES: [&str; 4] = [
    "No agregues filas encima del encabezado de PRODUCTOS ni cambies el orden de las columnas.",
    "Si alguna fila tiene errores, no se importa nada hasta corregirlos.",
    "Los productos existentes no se actualizan: solo se agregan variantes nuevas.",
    "Puedes borrar esta hoja; el importador solo lee PRODUCTOS.",
];

fn build_template(path: &str) -> Result<(), AppError> {
    let formats = formats();
    let text = Format::new().set_num_format("@");
    let mut workbook = Workbook::new();

    let mut products = Worksheet::new();
    products.set_name("PRODUCTOS").map_err(xl)?;
    write_headers(&mut products, &IMPORT_HEADERS, &formats.header).map_err(xl)?;
    products.set_column_format(4, &text).map_err(xl)?;
    products.set_column_format(5, &text).map_err(xl)?;
    products.set_freeze_panes(1, 0).map_err(xl)?;
    for (column, width) in VARIANT_WIDTHS.iter().take(IMPORT_HEADERS.len()).enumerate() {
        products
            .set_column_width(column as u16, *width)
            .map_err(xl)?;
    }
    workbook.push_worksheet(products);

    let mut instructions = Worksheet::new();
    instructions.set_name("INSTRUCCIONES").map_err(xl)?;
    for (column, header) in ["Columna", "¿Obligatoria?", "Qué escribir", "Ejemplo"]
        .iter()
        .enumerate()
    {
        instructions
            .write_string_with_format(0, column as u16, *header, &formats.header)
            .map_err(xl)?;
    }
    for (index, (header, (required, description, example))) in IMPORT_HEADERS
        .iter()
        .zip(TEMPLATE_INSTRUCTIONS.iter())
        .enumerate()
    {
        let row = (index + 1) as u32;
        instructions.write_string(row, 0, *header).map_err(xl)?;
        instructions.write_string(row, 1, *required).map_err(xl)?;
        instructions
            .write_string(row, 2, *description)
            .map_err(xl)?;
        instructions.write_string(row, 3, *example).map_err(xl)?;
    }

    let notes_start = (IMPORT_HEADERS.len() + 3) as u32;
    instructions
        .write_string_with_format(notes_start - 1, 0, "Notas", &formats.header)
        .map_err(xl)?;
    for (index, note) in TEMPLATE_NOTES.iter().enumerate() {
        instructions
            .write_string(notes_start + index as u32, 0, *note)
            .map_err(xl)?;
    }
    for (column, width) in [24.0, 16.0, 90.0, 30.0].iter().enumerate() {
        instructions
            .set_column_width(column as u16, *width)
            .map_err(xl)?;
    }
    workbook.push_worksheet(instructions);
    workbook.save(path).map_err(xl)?;
    Ok(())
}

pub async fn export_template(file_path: &str) -> Result<(), AppError> {
    let mut path = file_path.to_string();
    if !path.to_lowercase().ends_with(".xlsx") {
        path.push_str(".xlsx");
    }
    tokio::task::spawn_blocking(move || build_template(&path))
        .await
        .map_err(|error| AppError::internal(&format!("Falló la plantilla: {}", error)))?
}
