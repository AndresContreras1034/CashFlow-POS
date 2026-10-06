use std::collections::HashMap;

use calamine::{open_workbook, Data, DataType, Reader, Xlsx};
use serde_json::{Map, Value as JsonValue};
use sqlx::{PgConnection, PgPool};

use crate::errors::app_error::AppError;
use crate::modules::inventory::dto::{ImportRowResult, ImportSummaryDto};
use crate::modules::inventory::export::IMPORT_HEADERS;
use crate::modules::inventory::models::MovementType;

// ============================================================
// PARSEO DEL ARCHIVO
// ============================================================

struct ParsedRow {
    row_number: usize,
    product_name: String,
    brand: Option<String>,
    category_name: String,
    description: Option<String>,
    sku: Option<String>,
    barcode: Option<String>,
    price: i64,
    cost: i64,
    stock: i32,
    stock_min: i32,
    attributes: JsonValue,
    allow_negative: bool,
}

enum RowOutcome {
    Ok(ParsedRow),
    Error {
        row_number: usize,
        product_name: String,
        message: String,
    },
}

fn norm(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .map(|character| match character {
            'á' => 'a',
            'é' => 'e',
            'í' => 'i',
            'ó' => 'o',
            'ú' | 'ü' => 'u',
            other => other,
        })
        .collect()
}

fn validate_headers(header: &[Data]) -> Result<(), AppError> {
    for (index, expected) in IMPORT_HEADERS.iter().enumerate() {
        let found = header
            .get(index)
            .map(ToString::to_string)
            .unwrap_or_default();
        if norm(&found) != norm(expected) {
            return Err(AppError::validation(&format!(
                "Encabezado incorrecto en la columna {}: se esperaba «{}» y se encontró «{}». Descarga la plantilla para ver el formato.",
                index + 1,
                expected,
                found.trim()
            )));
        }
    }
    Ok(())
}

fn to_i32(value: f64) -> Option<i32> {
    if value.fract() == 0.0 && value >= i32::MIN as f64 && value <= i32::MAX as f64 {
        Some(value as i32)
    } else {
        None
    }
}

fn to_cents(value: f64) -> Option<i64> {
    let cents = (value * 100.0).round();
    if cents >= 0.0 && cents <= 9.0e15 {
        Some(cents as i64)
    } else {
        None
    }
}

fn parse_workbook(path: &str) -> Result<Vec<RowOutcome>, AppError> {
    let mut workbook: Xlsx<_> = open_workbook(path)
        .map_err(|e| AppError::validation(&format!("No se pudo abrir el archivo: {}", e)))?;

    let sheet_name = workbook
        .sheet_names()
        .iter()
        .find(|n| n.eq_ignore_ascii_case("PRODUCTOS"))
        .cloned()
        .or_else(|| workbook.sheet_names().first().cloned())
        .ok_or_else(|| AppError::validation("El archivo no tiene hojas"))?;

    let range = workbook.worksheet_range(&sheet_name).map_err(|e| {
        AppError::validation(&format!("No se pudo leer la hoja '{}': {}", sheet_name, e))
    })?;
    let mut source_rows = range.rows().enumerate();
    let (_, header) = source_rows
        .next()
        .ok_or_else(|| AppError::validation("La hoja está vacía"))?;
    validate_headers(header)?;

    let mut rows = Vec::new();
    for (index, row) in source_rows {
        let row_number = index + 1;
        if row.iter().all(|c| c.is_empty()) {
            continue;
        }
        rows.push(match parse_row(row, row_number) {
            Ok(parsed) => RowOutcome::Ok(parsed),
            Err((product_name, message)) => RowOutcome::Error {
                row_number,
                product_name,
                message,
            },
        });
    }

    Ok(rows)
}

fn parse_row(row: &[Data], row_number: usize) -> Result<ParsedRow, (String, String)> {
    let get_str = |index: usize| -> Option<String> {
        row.get(index).and_then(|cell| {
            let value = cell.to_string().trim().to_string();
            if value.is_empty() {
                None
            } else {
                Some(value)
            }
        })
    };
    let get_num = |index: usize| -> Result<Option<f64>, ()> {
        match row.get(index) {
            None => Ok(None),
            Some(cell) if cell.is_empty() => Ok(None),
            Some(cell) => {
                if let Some(value) = cell.get_float() {
                    return if value.is_finite() {
                        Ok(Some(value))
                    } else {
                        Err(())
                    };
                }
                let text = cell.to_string();
                let text = text.trim();
                if text.is_empty() {
                    Ok(None)
                } else if text.chars().all(|character| character.is_ascii_digit()) {
                    text.parse::<f64>().map(Some).map_err(|_| ())
                } else {
                    Err(())
                }
            }
        }
    };

    let product_name = get_str(0).ok_or_else(|| {
        (
            "(sin nombre)".to_string(),
            "Falta el nombre del producto".to_string(),
        )
    })?;
    let fail = |message: &str| (product_name.clone(), message.to_string());
    let category_name = get_str(2).ok_or_else(|| fail("Falta la categoría"))?;

    let price = match get_num(6) {
        Ok(Some(value)) if value >= 0.0 => {
            to_cents(value).ok_or_else(|| fail("Precio fuera de rango"))?
        }
        _ => return Err(fail("Precio inválido o faltante")),
    };
    let cost = match get_num(7) {
        Ok(None) => 0,
        Ok(Some(value)) if value >= 0.0 => {
            to_cents(value).ok_or_else(|| fail("Costo fuera de rango"))?
        }
        _ => return Err(fail("Costo inválido")),
    };
    let stock = match get_num(8) {
        Ok(Some(value)) if value >= 0.0 => {
            to_i32(value).ok_or_else(|| fail("El stock debe ser un número entero"))?
        }
        _ => return Err(fail("Stock inválido o faltante")),
    };
    let stock_min = match get_num(9) {
        Ok(None) => 0,
        Ok(Some(value)) if value >= 0.0 => {
            to_i32(value).ok_or_else(|| fail("El stock mínimo debe ser un número entero"))?
        }
        _ => return Err(fail("Stock mínimo inválido")),
    };

    let attributes_raw = get_str(10).unwrap_or_default();
    let attributes = parse_attributes(&attributes_raw);
    let declared_attributes = attributes_raw
        .split(|character| character == '\n' || character == ';')
        .filter(|pair| !pair.trim().is_empty())
        .count();
    let parsed_attributes = attributes.as_object().map(|map| map.len()).unwrap_or(0);
    if declared_attributes != parsed_attributes {
        return Err(fail(
            "Atributos mal formados: usa «Clave: Valor» separados por «;» o salto de línea",
        ));
    }

    let allow_negative = match get_str(11).map(|value| norm(&value)).as_deref() {
        None | Some("no") | Some("false") | Some("0") => false,
        Some("si") | Some("yes") | Some("true") | Some("1") => true,
        Some(_) => return Err(fail("«Permitir stock negativo» debe ser Sí o No")),
    };

    Ok(ParsedRow {
        row_number,
        product_name,
        brand: get_str(1),
        category_name,
        description: get_str(3),
        sku: get_str(4),
        barcode: get_str(5),
        price,
        cost,
        stock,
        stock_min,
        attributes,
        allow_negative,
    })
}

/// "Color: Dorado; Talla: M" o multilínea -> {"Color":"Dorado","Talla":"M"}
fn parse_attributes(raw: &str) -> JsonValue {
    let mut map = Map::new();
    for pair in raw.split(|c| c == '\n' || c == ';') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        if let Some((k, v)) = pair.split_once(':') {
            let (k, v) = (k.trim(), v.trim());
            if !k.is_empty() && !v.is_empty() {
                map.insert(k.to_string(), JsonValue::String(v.to_string()));
            }
        }
    }
    JsonValue::Object(map)
}

// ============================================================
// EJECUCIÓN (compartida por preview y execute)
// ============================================================

async fn run_import(
    pool: &PgPool,
    path: &str,
    dry_run: bool,
) -> Result<ImportSummaryDto, AppError> {
    let parsed = parse_workbook(path)?;

    let mut tx = pool.begin().await.map_err(AppError::from)?;

    let mut category_cache: HashMap<String, i32> = HashMap::new();
    let mut product_cache: HashMap<String, (i32, i32)> = HashMap::new(); // nombre_lower -> (id, category_id)

    let mut result_rows = Vec::new();
    let mut new_categories = 0;
    let mut new_products = 0;
    let mut new_variants = 0;
    let mut skipped = 0;
    let mut errors = 0;

    for outcome in parsed {
        let row = match outcome {
            RowOutcome::Error {
                row_number,
                product_name,
                message,
            } => {
                errors += 1;
                result_rows.push(ImportRowResult {
                    row_number,
                    product_name,
                    sku: None,
                    status: "error".into(),
                    message: Some(message),
                });
                continue;
            }
            RowOutcome::Ok(r) => r,
        };

        // -- SKU duplicado: se omite la fila --
        if let Some(ref sku) = row.sku {
            let exists = sqlx::query_scalar!(
                "SELECT id FROM product_variants WHERE LOWER(sku) = LOWER($1)",
                sku
            )
            .fetch_optional(&mut *tx)
            .await
            .map_err(AppError::from)?;

            if exists.is_some() {
                skipped += 1;
                result_rows.push(ImportRowResult {
                    row_number: row.row_number,
                    product_name: row.product_name.clone(),
                    sku: row.sku.clone(),
                    status: "skipped_duplicate_sku".into(),
                    message: Some(format!("El SKU '{}' ya existe", sku)),
                });
                continue;
            }
        }

        // -- Código de barras duplicado: requiere corregir los datos --
        if let Some(ref barcode) = row.barcode {
            let exists = sqlx::query_scalar!(
                "SELECT id FROM product_variants WHERE barcode = $1",
                barcode
            )
            .fetch_optional(&mut *tx)
            .await
            .map_err(AppError::from)?;

            if exists.is_some() {
                errors += 1;
                result_rows.push(ImportRowResult {
                    row_number: row.row_number,
                    product_name: row.product_name.clone(),
                    sku: row.sku.clone(),
                    status: "error".into(),
                    message: Some(format!("El código de barras '{}' ya existe", barcode)),
                });
                continue;
            }
        }

        // -- Categoría: buscar case-insensitive/trim, o crear --
        let category_key = row.category_name.trim().to_lowercase();
        let category_id = if let Some(&id) = category_cache.get(&category_key) {
            id
        } else if let Some(id) = sqlx::query_scalar!(
            "SELECT id FROM categories WHERE LOWER(TRIM(name)) = $1",
            category_key
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(AppError::from)?
        {
            category_cache.insert(category_key.clone(), id);
            id
        } else {
            let id = sqlx::query_scalar!(
                "INSERT INTO categories (name) VALUES ($1) RETURNING id",
                row.category_name.trim()
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(AppError::from)?;
            category_cache.insert(category_key.clone(), id);
            new_categories += 1;
            id
        };

        // -- Producto: buscar por nombre, o crear --
        let product_key = row.product_name.trim().to_lowercase();
        let (product_id, is_new_product) = if let Some(&(id, _)) = product_cache.get(&product_key) {
            (id, false)
        } else if let Some(id) = sqlx::query_scalar!(
            "SELECT id FROM products WHERE LOWER(TRIM(name)) = $1",
            product_key
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(AppError::from)?
        {
            product_cache.insert(product_key.clone(), (id, category_id));
            (id, false)
        } else {
            let id = sqlx::query_scalar!(
                "INSERT INTO products (category_id, name, description, brand)
                 VALUES ($1, $2, $3, $4) RETURNING id",
                category_id,
                row.product_name.trim(),
                row.description,
                row.brand
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(AppError::from)?;
            product_cache.insert(product_key.clone(), (id, category_id));
            new_products += 1;
            (id, true)
        };

        if !is_new_product {
            let duplicate = sqlx::query_scalar!(
                "SELECT id FROM product_variants WHERE product_id = $1 AND attributes = $2",
                product_id,
                row.attributes
            )
            .fetch_optional(&mut *tx)
            .await
            .map_err(AppError::from)?;

            if duplicate.is_some() {
                errors += 1;
                result_rows.push(ImportRowResult {
                    row_number: row.row_number,
                    product_name: row.product_name.clone(),
                    sku: row.sku.clone(),
                    status: "error".into(),
                    message: Some(
                        "Ya existe una variante de este producto con esos atributos".into(),
                    ),
                });
                continue;
            }
        }

        // -- Variante: crear con stock 0, luego aplicar entrada inicial --
        let variant_id = sqlx::query_scalar!(
            "INSERT INTO product_variants
                 (product_id, attributes, sku, barcode, price, cost, stock, stock_min, allow_negative)
             VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $8) RETURNING id",
            product_id, row.attributes, row.sku, row.barcode,
            row.price, row.cost, row.stock_min, row.allow_negative
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(AppError::from)?;

        if row.stock > 0 {
            apply_initial_stock(&mut tx, variant_id, row.stock, row.cost).await?;
        }

        new_variants += 1;
        result_rows.push(ImportRowResult {
            row_number: row.row_number,
            product_name: row.product_name.clone(),
            sku: row.sku.clone(),
            status: if is_new_product {
                "new_product".into()
            } else {
                "existing_product".into()
            },
            message: None,
        });
    }

    if dry_run || errors > 0 {
        tx.rollback().await.map_err(AppError::from)?;
    } else {
        tx.commit().await.map_err(AppError::from)?;
    }

    Ok(ImportSummaryDto {
        new_categories,
        new_products,
        new_variants,
        skipped,
        errors,
        rows: result_rows,
        can_execute: errors == 0,
    })
}

/// Misma lógica de 3 pasos que inventory::repository::apply_stock_entry,
/// pero contra la transacción compartida del importador (no abre la suya propia).
/// TODO: cuando se refactorice apply_stock_entry al patrón &mut PgConnection
/// (como ya se hizo con apply_stock_out), reemplazar esto por una llamada directa.
async fn apply_initial_stock(
    conn: &mut PgConnection,
    variant_id: i32,
    quantity: i32,
    unit_cost: i64,
) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE product_variants SET stock = $1 WHERE id = $2",
        quantity,
        variant_id
    )
    .execute(&mut *conn)
    .await
    .map_err(AppError::from)?;

    sqlx::query!(
        "INSERT INTO inventory_movements
             (variant_id, movement_type, quantity, stock_before, stock_after, unit_cost, notes, created_by)
         VALUES ($1, $2, $3, 0, $4, $5, $6, $7)",
        variant_id, MovementType::InitialStock as MovementType, quantity, quantity,
        unit_cost, "Importación masiva vía Excel", "system"
    )
    .execute(&mut *conn)
    .await
    .map_err(AppError::from)?;

    Ok(())
}

pub async fn preview_import(pool: &PgPool, path: &str) -> Result<ImportSummaryDto, AppError> {
    run_import(pool, path, true).await
}

pub async fn execute_import(pool: &PgPool, path: &str) -> Result<ImportSummaryDto, AppError> {
    run_import(pool, path, false).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_data_row() -> Vec<Data> {
        let mut row = vec![Data::Empty; IMPORT_HEADERS.len()];
        row[0] = Data::String("Perfume Rosa".into());
        row[2] = Data::String("Perfumes".into());
        row[6] = Data::Float(28_000.0);
        row[8] = Data::Int(2);
        row
    }

    #[test]
    fn integers_reject_decimals_and_overflow() {
        assert_eq!(to_i32(5.0), Some(5));
        assert_eq!(to_i32(2.7), None);
        assert_eq!(to_i32(3.0e10), None);
    }

    #[test]
    fn cents_conversion() {
        assert_eq!(to_cents(28_000.0), Some(2_800_000));
        assert_eq!(to_cents(-1.0), None);
    }

    #[test]
    fn header_normalization() {
        assert_eq!(norm(" Categoría "), "categoria");
        assert_eq!(norm("Código de barras"), "codigo de barras");

        let valid_headers = IMPORT_HEADERS
            .iter()
            .map(|header| Data::String((*header).to_string()))
            .collect::<Vec<_>>();
        assert!(validate_headers(&valid_headers).is_ok());

        let mut reordered = valid_headers;
        reordered.swap(0, 1);
        assert!(validate_headers(&reordered).is_err());
    }

    #[test]
    fn attributes_parse() {
        let value = parse_attributes("Color: Azul; Talla: M");
        assert_eq!(value.as_object().unwrap().len(), 2);
    }

    #[test]
    fn ambiguous_text_price_is_rejected() {
        let mut row = valid_data_row();
        row[6] = Data::String("28.000".into());
        assert!(parse_row(&row, 2).is_err());
    }

    #[test]
    fn fractional_stock_and_invalid_cost_are_rejected() {
        let mut fractional_stock = valid_data_row();
        fractional_stock[8] = Data::Float(2.7);
        let (_, message) = parse_row(&fractional_stock, 2).err().unwrap();
        assert!(message.contains("entero"));

        let mut invalid_cost = valid_data_row();
        invalid_cost[7] = Data::String("abc".into());
        let (_, message) = parse_row(&invalid_cost, 2).err().unwrap();
        assert!(message.contains("Costo inválido"));
    }
}
