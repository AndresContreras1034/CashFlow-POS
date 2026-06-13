use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;

// ============================================================
// CATEGORÍAS
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Category {
    pub id:          i32,
    pub name:        String,
    pub description: Option<String>,
    pub is_active:   bool,
    pub created_at:  DateTime<Utc>,
    pub updated_at:  DateTime<Utc>,
}

// ============================================================
// PRODUCTOS
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Product {
    pub id:          i32,
    pub category_id: i32,
    pub name:        String,
    pub description: Option<String>,
    pub brand:       Option<String>,
    pub image_url:   Option<String>,
    pub is_active:   bool,
    pub created_at:  DateTime<Utc>,
    pub updated_at:  DateTime<Utc>,
}

/// Producto con su categoría incluida (para listados)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProductWithCategory {
    pub id:            i32,
    pub category_id:   i32,
    pub category_name: String,
    pub name:          String,
    pub description:   Option<String>,
    pub brand:         Option<String>,
    pub image_url:     Option<String>,
    pub is_active:     bool,
    pub created_at:    DateTime<Utc>,
    pub updated_at:    DateTime<Utc>,
}

// ============================================================
// VARIANTES
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProductVariant {
    pub id:             i32,
    pub product_id:     i32,
    pub attributes:     JsonValue,   // {"volumen": "100ml"} | {"talla": "M", "color": "Rojo"}
    pub sku:            Option<String>,
    pub barcode:        Option<String>,
    pub price:          i64,         // centavos
    pub cost:           i64,         // centavos
    pub stock:          i32,
    pub stock_min:      i32,
    pub allow_negative: bool,
    pub is_active:      bool,
    pub created_at:     DateTime<Utc>,
    pub updated_at:     DateTime<Utc>,
}

/// Variante con info del producto padre (para búsqueda por barcode en ventas)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VariantWithProduct {
    // Variante
    pub id:             i32,
    pub product_id:     i32,
    pub attributes:     JsonValue,
    pub sku:            Option<String>,
    pub barcode:        Option<String>,
    pub price:          i64,
    pub cost:           i64,
    pub stock:          i32,
    pub stock_min:      i32,
    pub allow_negative: bool,
    pub is_active:      bool,
    // Producto padre
    pub product_name:   String,
    pub brand:          Option<String>,
    pub image_url:      Option<String>,
    pub category_id:    i32,
    pub category_name:  String,
}

/// Stock status para alertas
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StockStatus {
    Ok,         // stock > stock_min
    Low,        // 0 < stock <= stock_min
    OutOfStock, // stock == 0
}

impl ProductVariant {
    pub fn stock_status(&self) -> StockStatus {
        if self.stock <= 0 {
            StockStatus::OutOfStock
        } else if self.stock <= self.stock_min {
            StockStatus::Low
        } else {
            StockStatus::Ok
        }
    }

    /// Precio formateado en pesos (centavos → pesos)
    pub fn price_display(&self) -> f64 {
        self.price as f64 / 100.0
    }
}

// ============================================================
// MOVIMIENTOS DE INVENTARIO (KARDEX)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "movement_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MovementType {
    Purchase,
    Sale,
    SaleReturn,
    ManualIn,
    ManualOut,
    Adjustment,
    InitialStock,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InventoryMovement {
    pub id:            i32,
    pub variant_id:    i32,
    pub movement_type: MovementType,
    pub quantity:      i32,
    pub stock_before:  i32,
    pub stock_after:   i32,
    pub unit_cost:     i64,
    pub sale_id:       Option<i32>,
    pub purchase_id:   Option<i32>,
    pub notes:         Option<String>,
    pub created_by:    String,
    pub created_at:    DateTime<Utc>,
}

/// Movimiento con info de la variante y producto (para el Kardex visual)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MovementWithDetails {
    pub id:            i32,
    pub variant_id:    i32,
    pub movement_type: MovementType,
    pub quantity:      i32,
    pub stock_before:  i32,
    pub stock_after:   i32,
    pub unit_cost:     i64,
    pub notes:         Option<String>,
    pub created_by:    String,
    pub created_at:    DateTime<Utc>,
    // Info variante/producto
    pub product_name:  String,
    pub attributes:    JsonValue,
    pub sku:           Option<String>,
    pub barcode:       Option<String>,
}