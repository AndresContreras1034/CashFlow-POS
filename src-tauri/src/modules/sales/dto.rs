use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::modules::sales::models::PaymentMethod;

#[derive(Debug, Deserialize)]
pub struct CreateSaleItemDto {
    pub variant_id: i32,
    pub quantity: i32,
    pub discount: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSalePaymentDto {
    pub method: PaymentMethod,
    pub amount: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateSaleDto {
    pub customer_id: Option<i32>,
    pub items: Vec<CreateSaleItemDto>,
    pub payments: Vec<CreateSalePaymentDto>,
    pub discount: Option<i64>,
    pub notes: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct SaleFilterDto {
    pub status: Option<String>,
    pub customer_id: Option<i32>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: i64, page: i64, page_size: i64) -> Self {
        let total_pages = if page_size > 0 {
            (total + page_size - 1) / page_size
        } else {
            0
        };
        Self {
            data,
            total,
            page,
            page_size,
            total_pages,
        }
    }
}
