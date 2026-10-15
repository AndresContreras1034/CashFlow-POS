use tauri::State;

use crate::db::DbPool;
use crate::errors::app_error::AppError;
use crate::modules::audit::{
    actor::declared_actor,
    failure::{finish, FailureContext},
    models::AuditModule,
};
use crate::modules::suppliers::payables::{self, SupplierPayables};
use crate::modules::suppliers::{
    bulk::{self, BulkLinkResult, LinkableIds, LinkablePage},
    dto::{
        BulkLinkDto, CreateSupplierDto, LinkSupplierVariantDto, LinkableFilterDto,
        PaginatedResponse, RestockFilterDto, SetSupplierActiveDto, SetSupplierCategoriesDto,
        SupplierFilterDto, SupplierVariantFilterDto, UnlinkSupplierVariantDto, UpdateSupplierDto,
        UpdateSupplierVariantDto,
    },
    models::{
        RestockGroup, RestockOverview, Supplier, SupplierCategory, SupplierOverview,
        SupplierVariantLink, SupplierVariantView, VariantSupplier,
    },
    service,
};

#[tauri::command]
pub async fn list_suppliers(
    pool: State<'_, DbPool>,
    filter: SupplierFilterDto,
) -> Result<PaginatedResponse<SupplierOverview>, String> {
    service::list_suppliers(&pool, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn set_supplier_categories(
    pool: State<'_, DbPool>,
    supplier_id: i32,
    dto: SetSupplierCategoriesDto,
) -> Result<(), String> {
    let context = FailureContext {
        module: AuditModule::Suppliers,
        action: "set_categories",
        entity_type: Some("supplier"),
        entity_id: Some(supplier_id.to_string()),
        actor: declared_actor(&dto.updated_by),
    };
    finish(
        &pool,
        context,
        service::set_supplier_categories(&pool, supplier_id, dto).await,
    )
    .await
}

#[tauri::command]
pub async fn list_supplier_categories(
    pool: State<'_, DbPool>,
    supplier_id: i32,
) -> Result<Vec<SupplierCategory>, String> {
    service::list_supplier_categories(&pool, supplier_id)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn link_supplier_variant(
    pool: State<'_, DbPool>,
    supplier_id: i32,
    dto: LinkSupplierVariantDto,
) -> Result<SupplierVariantLink, String> {
    let context = FailureContext {
        module: AuditModule::Suppliers,
        action: "link_variant",
        entity_type: Some("supplier_variant"),
        entity_id: None,
        actor: declared_actor(&dto.created_by),
    };
    finish(
        &pool,
        context,
        service::link_supplier_variant(&pool, supplier_id, dto).await,
    )
    .await
}

#[tauri::command]
pub async fn update_supplier_variant(
    pool: State<'_, DbPool>,
    link_id: i32,
    dto: UpdateSupplierVariantDto,
) -> Result<SupplierVariantLink, String> {
    let context = FailureContext {
        module: AuditModule::Suppliers,
        action: "update_variant",
        entity_type: Some("supplier_variant"),
        entity_id: Some(link_id.to_string()),
        actor: declared_actor(&dto.updated_by),
    };
    finish(
        &pool,
        context,
        service::update_supplier_variant(&pool, link_id, dto).await,
    )
    .await
}

#[tauri::command]
pub async fn unlink_supplier_variant(
    pool: State<'_, DbPool>,
    link_id: i32,
    dto: UnlinkSupplierVariantDto,
) -> Result<(), String> {
    let context = FailureContext {
        module: AuditModule::Suppliers,
        action: "unlink_variant",
        entity_type: Some("supplier_variant"),
        entity_id: Some(link_id.to_string()),
        actor: declared_actor(&dto.updated_by),
    };
    finish(
        &pool,
        context,
        service::unlink_supplier_variant(&pool, link_id, dto).await,
    )
    .await
}

#[tauri::command]
pub async fn list_supplier_variants(
    pool: State<'_, DbPool>,
    supplier_id: i32,
    filter: SupplierVariantFilterDto,
) -> Result<PaginatedResponse<SupplierVariantView>, String> {
    service::list_supplier_variants(&pool, supplier_id, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn link_supplier_variants(
    pool: State<'_, DbPool>,
    supplier_id: i32,
    dto: BulkLinkDto,
) -> Result<BulkLinkResult, String> {
    let context = FailureContext {
        module: AuditModule::Suppliers,
        action: "link_variants",
        entity_type: Some("supplier"),
        entity_id: Some(supplier_id.to_string()),
        actor: declared_actor(&dto.updated_by),
    };
    finish(
        &pool,
        context,
        bulk::link_supplier_variants(&pool, supplier_id, dto).await,
    )
    .await
}

#[tauri::command]
pub async fn search_linkable_variants(
    pool: State<'_, DbPool>,
    supplier_id: i32,
    filter: LinkableFilterDto,
) -> Result<LinkablePage, String> {
    bulk::search_linkable_variants(&pool, supplier_id, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn list_linkable_variant_ids(
    pool: State<'_, DbPool>,
    supplier_id: i32,
    filter: LinkableFilterDto,
) -> Result<LinkableIds, String> {
    bulk::list_linkable_variant_ids(&pool, supplier_id, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn list_variant_suppliers(
    pool: State<'_, DbPool>,
    variant_id: i32,
) -> Result<Vec<VariantSupplier>, String> {
    service::list_variant_suppliers(&pool, variant_id)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn get_supplier(pool: State<'_, DbPool>, id: i32) -> Result<SupplierOverview, String> {
    service::get_supplier(&pool, id)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn create_supplier(
    pool: State<'_, DbPool>,
    dto: CreateSupplierDto,
) -> Result<Supplier, String> {
    let context = FailureContext {
        module: AuditModule::Suppliers,
        action: "create",
        entity_type: Some("supplier"),
        entity_id: None,
        actor: declared_actor(&dto.created_by),
    };
    finish(&pool, context, service::create_supplier(&pool, dto).await).await
}

#[tauri::command]
pub async fn update_supplier(
    pool: State<'_, DbPool>,
    id: i32,
    dto: UpdateSupplierDto,
) -> Result<Supplier, String> {
    let context = FailureContext {
        module: AuditModule::Suppliers,
        action: "update",
        entity_type: Some("supplier"),
        entity_id: Some(id.to_string()),
        actor: declared_actor(&dto.updated_by),
    };
    finish(
        &pool,
        context,
        service::update_supplier(&pool, id, dto).await,
    )
    .await
}

#[tauri::command]
pub async fn set_supplier_active(
    pool: State<'_, DbPool>,
    id: i32,
    dto: SetSupplierActiveDto,
) -> Result<Supplier, String> {
    let context = FailureContext {
        module: AuditModule::Suppliers,
        action: "set_active",
        entity_type: Some("supplier"),
        entity_id: Some(id.to_string()),
        actor: declared_actor(&dto.updated_by),
    };
    finish(
        &pool,
        context,
        service::set_supplier_active(&pool, id, dto).await,
    )
    .await
}

#[tauri::command]
pub async fn get_restock_overview(
    pool: State<'_, DbPool>,
    filter: RestockFilterDto,
) -> Result<RestockOverview, String> {
    service::get_restock_overview(&pool, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn get_supplier_restock(
    pool: State<'_, DbPool>,
    supplier_id: i32,
    window_days: Option<i32>,
    cover_days: Option<i32>,
) -> Result<RestockGroup, String> {
    service::get_supplier_restock(&pool, supplier_id, window_days, cover_days)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn get_supplier_payables(
    pool: State<'_, DbPool>,
    supplier_id: i32,
) -> Result<SupplierPayables, String> {
    payables::get_supplier_payables(&pool, supplier_id)
        .await
        .map_err(|error: AppError| error.to_string())
}
