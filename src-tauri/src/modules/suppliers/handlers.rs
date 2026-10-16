use std::path::Path;
use tauri::State;

use crate::db::DbPool;
use crate::errors::app_error::AppError;
use crate::modules::audit::{
    actor::declared_actor,
    failure::{finish, FailureContext},
    models::AuditModule,
};
use crate::modules::suppliers::payables::{self, SupplierPayables};
use crate::modules::suppliers::purchase_lists::{
    self, AddItemsDto, AddItemsResult, AddRecommendedDto, CreatePurchaseListDto, PurchasableIds,
    PurchasablePage, PurchaseList, PurchaseListDetail, PurchaseListFilterDto, PurchaseListSummary,
    SetListStatusDto, UpdateItemDto, UpdatePurchaseListDto,
};
use crate::modules::suppliers::receive::{self, ReceiveListDto, ReceiveResult};
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

#[tauri::command]
pub async fn list_purchase_lists(
    pool: State<'_, DbPool>,
    filter: PurchaseListFilterDto,
) -> Result<PaginatedResponse<PurchaseListSummary>, String> {
    purchase_lists::list_purchase_lists(&pool, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn get_purchase_list(
    pool: State<'_, DbPool>,
    id: i32,
) -> Result<PurchaseListDetail, String> {
    purchase_lists::get_purchase_list(&pool, id)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn save_purchase_pdf(file_path: String, bytes: Vec<u8>) -> Result<(), String> {
    const MAX_PDF_BYTES: usize = 20 * 1024 * 1024;

    let path = Path::new(&file_path);
    if !path.is_absolute() {
        return Err("La ruta del PDF debe ser absoluta".to_string());
    }
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
    {
        return Err("El archivo debe tener extensión .pdf".to_string());
    }
    if bytes.is_empty() || bytes.len() > MAX_PDF_BYTES {
        return Err("El PDF está vacío o excede el tamaño permitido".to_string());
    }

    tokio::fs::write(path, bytes)
        .await
        .map_err(|error| format!("No se pudo guardar el PDF en {}: {error}", path.display()))
}

#[tauri::command]
pub async fn create_purchase_list(
    pool: State<'_, DbPool>,
    dto: CreatePurchaseListDto,
) -> Result<PurchaseList, String> {
    let context = FailureContext {
        module: AuditModule::Suppliers,
        action: "purchase_list_create",
        entity_type: Some("purchase_list"),
        entity_id: None,
        actor: declared_actor(&dto.created_by),
    };
    finish(
        &pool,
        context,
        purchase_lists::create_purchase_list(&pool, dto).await,
    )
    .await
}

#[tauri::command]
pub async fn update_purchase_list(
    pool: State<'_, DbPool>,
    id: i32,
    dto: UpdatePurchaseListDto,
) -> Result<PurchaseList, String> {
    let context = FailureContext {
        module: AuditModule::Suppliers,
        action: "purchase_list_update",
        entity_type: Some("purchase_list"),
        entity_id: Some(id.to_string()),
        actor: declared_actor(&dto.updated_by),
    };
    finish(
        &pool,
        context,
        purchase_lists::update_purchase_list(&pool, id, dto).await,
    )
    .await
}

#[tauri::command]
pub async fn set_purchase_list_status(
    pool: State<'_, DbPool>,
    id: i32,
    dto: SetListStatusDto,
) -> Result<PurchaseList, String> {
    let context = FailureContext {
        module: AuditModule::Suppliers,
        action: "purchase_list_status",
        entity_type: Some("purchase_list"),
        entity_id: Some(id.to_string()),
        actor: declared_actor(&dto.updated_by),
    };
    finish(
        &pool,
        context,
        purchase_lists::set_purchase_list_status(&pool, id, dto).await,
    )
    .await
}

#[tauri::command]
pub async fn receive_purchase_list(
    pool: State<'_, DbPool>,
    list_id: i32,
    dto: ReceiveListDto,
) -> Result<ReceiveResult, String> {
    let context = FailureContext {
        module: AuditModule::Suppliers,
        action: "purchase_list_receive",
        entity_type: Some("purchase_list"),
        entity_id: Some(list_id.to_string()),
        actor: declared_actor(&dto.updated_by),
    };
    finish(
        &pool,
        context,
        receive::receive_purchase_list(&pool, list_id, dto).await,
    )
    .await
}

#[tauri::command]
pub async fn add_purchase_items(
    pool: State<'_, DbPool>,
    list_id: i32,
    dto: AddItemsDto,
) -> Result<AddItemsResult, String> {
    let context = FailureContext {
        module: AuditModule::Suppliers,
        action: "purchase_list_add_items",
        entity_type: Some("purchase_list"),
        entity_id: Some(list_id.to_string()),
        actor: declared_actor(&dto.updated_by),
    };
    finish(
        &pool,
        context,
        purchase_lists::add_purchase_items(&pool, list_id, dto).await,
    )
    .await
}

#[tauri::command]
pub async fn add_recommended_to_list(
    pool: State<'_, DbPool>,
    list_id: i32,
    dto: AddRecommendedDto,
) -> Result<AddItemsResult, String> {
    let context = FailureContext {
        module: AuditModule::Suppliers,
        action: "purchase_list_add_items",
        entity_type: Some("purchase_list"),
        entity_id: Some(list_id.to_string()),
        actor: declared_actor(&dto.updated_by),
    };
    finish(
        &pool,
        context,
        purchase_lists::add_recommended_to_list(&pool, list_id, dto).await,
    )
    .await
}

#[tauri::command]
pub async fn update_purchase_item(
    pool: State<'_, DbPool>,
    item_id: i32,
    dto: UpdateItemDto,
) -> Result<purchase_lists::PurchaseItem, String> {
    let context = FailureContext {
        module: AuditModule::Suppliers,
        action: "purchase_item_update",
        entity_type: Some("purchase_list_item"),
        entity_id: Some(item_id.to_string()),
        actor: declared_actor(&dto.updated_by),
    };
    finish(
        &pool,
        context,
        purchase_lists::update_purchase_item(&pool, item_id, dto).await,
    )
    .await
}

#[tauri::command]
pub async fn remove_purchase_item(pool: State<'_, DbPool>, item_id: i32) -> Result<(), String> {
    let context = FailureContext {
        module: AuditModule::Suppliers,
        action: "purchase_item_remove",
        entity_type: Some("purchase_list_item"),
        entity_id: Some(item_id.to_string()),
        actor: None,
    };
    finish(
        &pool,
        context,
        purchase_lists::remove_purchase_item(&pool, item_id).await,
    )
    .await
}

#[tauri::command]
pub async fn search_purchasable_variants(
    pool: State<'_, DbPool>,
    list_id: i32,
    filter: LinkableFilterDto,
) -> Result<PurchasablePage, String> {
    purchase_lists::search_purchasable_variants(&pool, list_id, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}

#[tauri::command]
pub async fn list_purchasable_variant_ids(
    pool: State<'_, DbPool>,
    list_id: i32,
    filter: LinkableFilterDto,
) -> Result<PurchasableIds, String> {
    purchase_lists::list_purchasable_variant_ids(&pool, list_id, filter)
        .await
        .map_err(|error: AppError| error.to_string())
}
