use std::collections::{HashMap, HashSet};

use calamine::{open_workbook, Data, DataType, Reader, Xlsx};
use serde_json::{json, Map, Value as JsonValue};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::errors::app_error::AppError;
use crate::modules::audit::{
    dto::NewAuditEvent,
    models::{AuditCategory, AuditModule, AuditOutcome},
    repository as audit_repo,
};
use crate::modules::inventory::dto::{ImportRowResult, ImportSummaryDto};
use crate::modules::inventory::export::IMPORT_HEADERS;

/// Máximo de filas que se devuelven a la interfaz (errores y omitidas primero).
/// Devolver 20.000 filas congelaba React al renderizar la tabla.
const MAX_RESULT_ROWS: usize = 500;

/// Filas por sentencia INSERT ... UNNEST.
const BATCH_SIZE: usize = 5_000;

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

/// Clave canónica de un objeto de atributos (independiente del orden de las claves).
fn attrs_key(value: &JsonValue) -> String {
    match value {
        JsonValue::Object(map) => {
            let mut entries: Vec<(&String, &JsonValue)> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            entries
                .iter()
                .map(|(key, value)| format!("{}={}", key, value))
                .collect::<Vec<_>>()
                .join("\u{1}")
        }
        other => other.to_string(),
    }
}

// ============================================================
// EJECUCIÓN (compartida por preview y execute)
//
// Estrategia:
//   1. Parsear el Excel una vez (en un hilo bloqueante).
//   2. Precargar de PostgreSQL, en pocas consultas con = ANY($1), todo lo
//      que hace falta para validar: SKUs, códigos de barras, categorías,
//      productos por nombre y atributos de variantes de esos productos.
//   3. Validar las 20.000 filas EN MEMORIA (sin SQL por fila).
//   4. Solo en "execute" y sin errores: insertar por lotes con UNNEST.
//      El preview no escribe nada.
// ============================================================

struct NewProduct {
    key: String,
    category_key: String,
    name: String,
    description: Option<String>,
    brand: Option<String>,
}

struct AcceptedVariant {
    product_key: String,
    attributes: JsonValue,
    sku: Option<String>,
    barcode: Option<String>,
    price: i64,
    cost: i64,
    stock: i32,
    stock_min: i32,
    allow_negative: bool,
}

struct Lookups {
    /// SKUs existentes en minúsculas (más los aceptados durante la validación).
    skus: HashSet<String>,
    /// Códigos de barras existentes (más los aceptados durante la validación).
    barcodes: HashSet<String>,
    /// nombre de categoría normalizado -> id
    categories: HashMap<String, i32>,
    /// nombre de producto normalizado -> id
    products: HashMap<String, i32>,
    /// (producto normalizado, atributos canónicos) ya ocupados
    variant_attrs: HashSet<(String, String)>,
}

async fn load_lookups(conn: &mut PgConnection, rows: &[&ParsedRow]) -> Result<Lookups, AppError> {
    let sku_list: Vec<String> = rows
        .iter()
        .filter_map(|row| row.sku.as_ref().map(|sku| sku.to_lowercase()))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let barcode_list: Vec<String> = rows
        .iter()
        .filter_map(|row| row.barcode.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let product_key_list: Vec<String> = rows
        .iter()
        .map(|row| row.product_name.trim().to_lowercase())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let mut skus = HashSet::new();
    if !sku_list.is_empty() {
        let found: Vec<String> = sqlx::query_scalar(
            "SELECT LOWER(sku) FROM product_variants WHERE LOWER(sku) = ANY($1)",
        )
        .bind(&sku_list)
        .fetch_all(&mut *conn)
        .await
        .map_err(AppError::from)?;
        skus.extend(found);
    }

    let mut barcodes = HashSet::new();
    if !barcode_list.is_empty() {
        let found: Vec<String> =
            sqlx::query_scalar("SELECT barcode FROM product_variants WHERE barcode = ANY($1)")
                .bind(&barcode_list)
                .fetch_all(&mut *conn)
                .await
                .map_err(AppError::from)?;
        barcodes.extend(found);
    }

    let mut categories = HashMap::new();
    let category_rows: Vec<(i32, String)> =
        sqlx::query_as("SELECT id, LOWER(TRIM(name)) FROM categories ORDER BY id")
            .fetch_all(&mut *conn)
            .await
            .map_err(AppError::from)?;
    for (id, key) in category_rows {
        categories.entry(key).or_insert(id);
    }

    let mut products: HashMap<String, i32> = HashMap::new();
    if !product_key_list.is_empty() {
        let product_rows: Vec<(i32, String)> = sqlx::query_as(
            "SELECT id, LOWER(TRIM(name)) FROM products
             WHERE LOWER(TRIM(name)) = ANY($1) ORDER BY id",
        )
        .bind(&product_key_list)
        .fetch_all(&mut *conn)
        .await
        .map_err(AppError::from)?;
        for (id, key) in product_rows {
            products.entry(key).or_insert(id);
        }
    }

    let mut variant_attrs = HashSet::new();
    if !products.is_empty() {
        let id_to_key: HashMap<i32, &String> = products.iter().map(|(k, id)| (*id, k)).collect();
        let product_ids: Vec<i32> = id_to_key.keys().copied().collect();
        let variant_rows: Vec<(i32, JsonValue)> = sqlx::query_as(
            "SELECT product_id, attributes FROM product_variants WHERE product_id = ANY($1)",
        )
        .bind(&product_ids)
        .fetch_all(&mut *conn)
        .await
        .map_err(AppError::from)?;
        for (product_id, attributes) in variant_rows {
            if let Some(key) = id_to_key.get(&product_id) {
                variant_attrs.insert(((*key).clone(), attrs_key(&attributes)));
            }
        }
    }

    Ok(Lookups {
        skus,
        barcodes,
        categories,
        products,
        variant_attrs,
    })
}

fn limit_rows(mut rows: Vec<ImportRowResult>) -> Vec<ImportRowResult> {
    if rows.len() <= MAX_RESULT_ROWS {
        return rows;
    }
    // Estable: errores primero, luego omitidas, luego el resto (en orden de fila).
    rows.sort_by_key(|row| match row.status.as_str() {
        "error" => 0u8,
        "skipped_duplicate_sku" => 1,
        _ => 2,
    });
    rows.truncate(MAX_RESULT_ROWS);
    rows.sort_by_key(|row| row.row_number);
    rows
}

async fn run_import(
    pool: &PgPool,
    path: &str,
    dry_run: bool,
) -> Result<ImportSummaryDto, AppError> {
    // Parseo del XLSX: CPU + I/O síncrono. Fuera del hilo async de tokio.
    let path_owned = path.to_string();
    let parsed = tokio::task::spawn_blocking(move || parse_workbook(&path_owned))
        .await
        .map_err(|e| AppError::internal(&format!("Falló la lectura del Excel: {}", e)))??;

    let mut tx = pool.begin().await.map_err(AppError::from)?;

    let decimals: i16 =
        sqlx::query_scalar("SELECT currency_decimals FROM app_settings WHERE id = 1")
            .fetch_one(&mut *tx)
            .await
            .map_err(AppError::from)?;
    let money_step: i64 = if decimals == 0 { 100 } else { 1 };

    let mut lookups = {
        let ok_rows: Vec<&ParsedRow> = parsed
            .iter()
            .filter_map(|outcome| match outcome {
                RowOutcome::Ok(row) => Some(row),
                RowOutcome::Error { .. } => None,
            })
            .collect();
        load_lookups(&mut *tx, &ok_rows).await?
    };

    let mut new_category_keys: HashSet<String> = HashSet::new();
    let mut new_categories_list: Vec<(String, String)> = Vec::new(); // (key, nombre)
    let mut new_product_keys: HashSet<String> = HashSet::new();
    let mut new_products_list: Vec<NewProduct> = Vec::new();
    let mut accepted: Vec<AcceptedVariant> = Vec::new();

    let mut result_rows: Vec<ImportRowResult> = Vec::with_capacity(parsed.len());
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

        if row.price % money_step != 0 || row.cost % money_step != 0 {
            errors += 1;
            result_rows.push(ImportRowResult {
                row_number: row.row_number,
                product_name: row.product_name.clone(),
                sku: row.sku.clone(),
                status: "error".into(),
                message: Some(
                    "La moneda configurada no admite centavos: usa pesos enteros en precio y costo"
                        .into(),
                ),
            });
            continue;
        }

        // -- SKU duplicado (en BD o en filas anteriores del archivo): se omite --
        if let Some(sku) = &row.sku {
            if lookups.skus.contains(&sku.to_lowercase()) {
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
        if let Some(barcode) = &row.barcode {
            if lookups.barcodes.contains(barcode) {
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

        // -- Categoría: existente o por crear --
        let category_key = row.category_name.trim().to_lowercase();
        if !lookups.categories.contains_key(&category_key)
            && !new_category_keys.contains(&category_key)
        {
            new_category_keys.insert(category_key.clone());
            new_categories_list.push((category_key.clone(), row.category_name.trim().to_string()));
            new_categories += 1;
        }

        // -- Producto: existente o por crear --
        let product_key = row.product_name.trim().to_lowercase();
        let is_new_product = if lookups.products.contains_key(&product_key)
            || new_product_keys.contains(&product_key)
        {
            false
        } else {
            new_product_keys.insert(product_key.clone());
            new_products_list.push(NewProduct {
                key: product_key.clone(),
                category_key: category_key.clone(),
                name: row.product_name.trim().to_string(),
                description: row.description.clone(),
                brand: row.brand.clone(),
            });
            new_products += 1;
            true
        };

        // -- Variante duplicada (mismo producto + mismos atributos) --
        let attributes_id = attrs_key(&row.attributes);
        if !is_new_product
            && lookups
                .variant_attrs
                .contains(&(product_key.clone(), attributes_id.clone()))
        {
            errors += 1;
            result_rows.push(ImportRowResult {
                row_number: row.row_number,
                product_name: row.product_name.clone(),
                sku: row.sku.clone(),
                status: "error".into(),
                message: Some("Ya existe una variante de este producto con esos atributos".into()),
            });
            continue;
        }

        // -- Fila aceptada: reservar SKU, barcode y atributos para las siguientes --
        lookups
            .variant_attrs
            .insert((product_key.clone(), attributes_id));
        if let Some(sku) = &row.sku {
            lookups.skus.insert(sku.to_lowercase());
        }
        if let Some(barcode) = &row.barcode {
            lookups.barcodes.insert(barcode.clone());
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
        accepted.push(AcceptedVariant {
            product_key,
            attributes: row.attributes,
            sku: row.sku,
            barcode: row.barcode,
            price: row.price,
            cost: row.cost,
            stock: row.stock,
            stock_min: row.stock_min,
            allow_negative: row.allow_negative,
        });
    }

    // Solo "execute" sin errores escribe. El preview nunca inserta nada.
    if !dry_run && errors == 0 {
        write_import(
            &mut *tx,
            &lookups.categories,
            &lookups.products,
            &new_categories_list,
            &new_products_list,
            &accepted,
        )
        .await?;

        let file_name = std::path::Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string);

        audit_repo::insert_event(
            &mut *tx,
            NewAuditEvent {
                correlation_id: Some(Uuid::new_v4()),
                category: AuditCategory::Business,
                module: AuditModule::Import,
                action: "execute".to_string(),
                outcome: AuditOutcome::Success,
                actor: None,
                entity_type: Some("inventory_import".to_string()),
                entity_id: None,
                summary: format!(
                    "Importación masiva: {} filas, {} productos y {} variantes nuevas",
                    result_rows.len(),
                    new_products,
                    new_variants
                ),
                changes: None,
                metadata: Some(json!({
                    "file_name": file_name,
                    "total_rows": result_rows.len(),
                    "new_categories": new_categories,
                    "new_products": new_products,
                    "new_variants": new_variants,
                    "skipped": skipped,
                    "errors": errors,
                })),
                error_message: None,
            },
        )
        .await
        .map_err(AppError::from)?;
    }

    if dry_run || errors > 0 {
        tx.rollback().await.map_err(AppError::from)?;
    } else {
        tx.commit().await.map_err(AppError::from)?;
    }

    let total_rows = result_rows.len();
    let rows = limit_rows(result_rows);
    let rows_truncated = total_rows > rows.len();

    Ok(ImportSummaryDto {
        new_categories,
        new_products,
        new_variants,
        skipped,
        errors,
        rows,
        total_rows,
        rows_truncated,
        can_execute: errors == 0,
    })
}

/// Inserta categorías, productos, variantes y movimientos de stock inicial por lotes.
/// Todo dentro de la transacción del llamador.
async fn write_import(
    conn: &mut PgConnection,
    existing_categories: &HashMap<String, i32>,
    existing_products: &HashMap<String, i32>,
    new_categories: &[(String, String)],
    new_products: &[NewProduct],
    variants: &[AcceptedVariant],
) -> Result<(), AppError> {
    // ---- Categorías ----
    let mut category_ids = existing_categories.clone();
    if !new_categories.is_empty() {
        let names: Vec<String> = new_categories
            .iter()
            .map(|(_, name)| name.clone())
            .collect();
        let key_by_name: HashMap<&str, &str> = new_categories
            .iter()
            .map(|(key, name)| (name.as_str(), key.as_str()))
            .collect();
        let inserted: Vec<(i32, String)> = sqlx::query_as(
            "INSERT INTO categories (name) SELECT * FROM UNNEST($1::text[]) RETURNING id, name",
        )
        .bind(names)
        .fetch_all(&mut *conn)
        .await
        .map_err(AppError::from)?;
        for (id, name) in inserted {
            if let Some(key) = key_by_name.get(name.as_str()) {
                category_ids.insert((*key).to_string(), id);
            }
        }
    }

    // ---- Productos ----
    let mut product_ids = existing_products.clone();
    for chunk in new_products.chunks(BATCH_SIZE) {
        let mut categories: Vec<i32> = Vec::with_capacity(chunk.len());
        let mut names: Vec<String> = Vec::with_capacity(chunk.len());
        let mut descriptions: Vec<Option<String>> = Vec::with_capacity(chunk.len());
        let mut brands: Vec<Option<String>> = Vec::with_capacity(chunk.len());
        let mut key_by_name: HashMap<&str, &str> = HashMap::with_capacity(chunk.len());

        for product in chunk {
            let category_id = *category_ids.get(&product.category_key).ok_or_else(|| {
                AppError::internal("No se pudo resolver una categoría durante la importación")
            })?;
            categories.push(category_id);
            names.push(product.name.clone());
            descriptions.push(product.description.clone());
            brands.push(product.brand.clone());
            key_by_name.insert(product.name.as_str(), product.key.as_str());
        }

        let inserted: Vec<(i32, String)> = sqlx::query_as(
            "INSERT INTO products (category_id, name, description, brand)
             SELECT * FROM UNNEST($1::int4[], $2::text[], $3::text[], $4::text[])
             RETURNING id, name",
        )
        .bind(categories)
        .bind(names)
        .bind(descriptions)
        .bind(brands)
        .fetch_all(&mut *conn)
        .await
        .map_err(AppError::from)?;
        for (id, name) in inserted {
            if let Some(key) = key_by_name.get(name.as_str()) {
                product_ids.insert((*key).to_string(), id);
            }
        }
    }

    if variants.is_empty() {
        return Ok(());
    }

    // ---- IDs de variantes: se reservan de la secuencia para poder enlazar los
    // movimientos sin depender del orden de RETURNING ----
    let variant_ids: Vec<i32> = sqlx::query_scalar(
        "SELECT nextval(pg_get_serial_sequence('product_variants', 'id'))::int4
         FROM generate_series(1, $1::int4)",
    )
    .bind(variants.len() as i32)
    .fetch_all(&mut *conn)
    .await
    .map_err(AppError::from)?;
    if variant_ids.len() != variants.len() {
        return Err(AppError::internal(
            "No se pudieron reservar los identificadores de variantes",
        ));
    }

    let mut offset = 0usize;
    for chunk in variants.chunks(BATCH_SIZE) {
        let ids = &variant_ids[offset..offset + chunk.len()];
        offset += chunk.len();

        let mut product_col: Vec<i32> = Vec::with_capacity(chunk.len());
        let mut attrs_col: Vec<String> = Vec::with_capacity(chunk.len());
        let mut sku_col: Vec<Option<String>> = Vec::with_capacity(chunk.len());
        let mut barcode_col: Vec<Option<String>> = Vec::with_capacity(chunk.len());
        let mut price_col: Vec<i64> = Vec::with_capacity(chunk.len());
        let mut cost_col: Vec<i64> = Vec::with_capacity(chunk.len());
        let mut stock_col: Vec<i32> = Vec::with_capacity(chunk.len());
        let mut stock_min_col: Vec<i32> = Vec::with_capacity(chunk.len());
        let mut negative_col: Vec<bool> = Vec::with_capacity(chunk.len());

        // Para los movimientos de stock inicial (solo stock > 0)
        let mut mv_variant: Vec<i32> = Vec::new();
        let mut mv_quantity: Vec<i32> = Vec::new();
        let mut mv_cost: Vec<i64> = Vec::new();

        for (variant, &variant_id) in chunk.iter().zip(ids.iter()) {
            let product_id = *product_ids.get(&variant.product_key).ok_or_else(|| {
                AppError::internal("No se pudo resolver un producto durante la importación")
            })?;
            product_col.push(product_id);
            attrs_col.push(variant.attributes.to_string());
            sku_col.push(variant.sku.clone());
            barcode_col.push(variant.barcode.clone());
            price_col.push(variant.price);
            cost_col.push(variant.cost);
            stock_col.push(variant.stock);
            stock_min_col.push(variant.stock_min);
            negative_col.push(variant.allow_negative);

            if variant.stock > 0 {
                mv_variant.push(variant_id);
                mv_quantity.push(variant.stock);
                mv_cost.push(variant.cost);
            }
        }

        sqlx::query(
            "INSERT INTO product_variants
                 (id, product_id, attributes, sku, barcode, price, cost, stock, stock_min, allow_negative)
             SELECT t.id, t.product_id, t.attributes::jsonb, t.sku, t.barcode,
                    t.price, t.cost, t.stock, t.stock_min, t.allow_negative
             FROM UNNEST(
                 $1::int4[], $2::int4[], $3::text[], $4::text[], $5::text[],
                 $6::int8[], $7::int8[], $8::int4[], $9::int4[], $10::bool[]
             ) AS t(id, product_id, attributes, sku, barcode,
                    price, cost, stock, stock_min, allow_negative)",
        )
        .bind(ids.to_vec())
        .bind(product_col)
        .bind(attrs_col)
        .bind(sku_col)
        .bind(barcode_col)
        .bind(price_col)
        .bind(cost_col)
        .bind(stock_col)
        .bind(stock_min_col)
        .bind(negative_col)
        .execute(&mut *conn)
        .await
        .map_err(AppError::from)?;

        if !mv_variant.is_empty() {
            sqlx::query(
                "INSERT INTO inventory_movements
                     (variant_id, movement_type, quantity, stock_before, stock_after,
                      unit_cost, notes, created_by)
                 SELECT t.variant_id, 'initial_stock'::movement_type, t.quantity, 0, t.quantity,
                        t.unit_cost, 'Importación masiva vía Excel', 'system'
                 FROM UNNEST($1::int4[], $2::int4[], $3::int8[])
                      AS t(variant_id, quantity, unit_cost)",
            )
            .bind(mv_variant)
            .bind(mv_quantity)
            .bind(mv_cost)
            .execute(&mut *conn)
            .await
            .map_err(AppError::from)?;
        }
    }

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
    fn attributes_key_ignores_order() {
        let a = parse_attributes("Color: Azul; Talla: M");
        let b = parse_attributes("Talla: M; Color: Azul");
        assert_eq!(attrs_key(&a), attrs_key(&b));
        let c = parse_attributes("Color: Rojo; Talla: M");
        assert_ne!(attrs_key(&a), attrs_key(&c));
    }

    #[test]
    fn result_rows_are_capped_errors_first() {
        let rows: Vec<ImportRowResult> = (1..=2_000)
            .map(|n| ImportRowResult {
                row_number: n,
                product_name: format!("p{}", n),
                sku: None,
                status: if n % 1_000 == 0 {
                    "error".into()
                } else {
                    "new_product".into()
                },
                message: None,
            })
            .collect();
        let limited = limit_rows(rows);
        assert_eq!(limited.len(), MAX_RESULT_ROWS);
        assert!(limited.iter().any(|r| r.row_number == 1_000));
        assert!(limited.iter().any(|r| r.row_number == 2_000));
        assert!(limited
            .windows(2)
            .all(|w| w[0].row_number < w[1].row_number));
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
