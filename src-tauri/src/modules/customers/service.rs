use sqlx::PgPool;

use crate::errors::app_error::AppError;
use crate::modules::customers::{
    dto::{
        CreateCustomerDto, CustomerFilterDto, PaginatedResponse, SetCustomerActiveDto,
        UpdateCustomerDto,
    },
    models::{Customer, CustomerName, CustomerOverview},
    repository::{self, CustomerError},
};

const MAX_NAME: usize = 150;
const MAX_DOC_NUMBER: usize = 30;
const MAX_PHONE: usize = 50;
const MAX_EMAIL: usize = 150;
const MAX_TEXT: usize = 2000;
const MAX_SEARCH: usize = 100;
const DOCUMENT_TYPES: [&str; 6] = ["CC", "NIT", "CE", "TI", "PP", "OTRO"];

pub async fn get_customer_names(
    pool: &PgPool,
    mut ids: Vec<i32>,
) -> Result<Vec<CustomerName>, AppError> {
    ids.sort_unstable();
    ids.dedup();
    ids.truncate(200);
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    repository::names_by_ids(pool, &ids)
        .await
        .map_err(AppError::from)
}

fn optional_text(
    value: Option<String>,
    label: &str,
    max: usize,
) -> Result<Option<String>, AppError> {
    let Some(text) = value
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
    else {
        return Ok(None);
    };
    if text.chars().count() > max {
        return Err(AppError::validation(&format!(
            "{label} no puede superar {max} caracteres"
        )));
    }
    Ok(Some(text))
}

struct Fields {
    name: String,
    document_type: Option<String>,
    document_number: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    address: Option<String>,
    notes: Option<String>,
}

#[allow(clippy::too_many_arguments)]
fn normalize(
    name: String,
    document_type: Option<String>,
    document_number: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    address: Option<String>,
    notes: Option<String>,
) -> Result<Fields, AppError> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::validation("El nombre es obligatorio"));
    }
    if name.chars().count() > MAX_NAME {
        return Err(AppError::validation(&format!(
            "El nombre no puede superar {MAX_NAME} caracteres"
        )));
    }

    let document_type =
        optional_text(document_type, "El tipo de documento", 10)?.map(|text| text.to_uppercase());
    let document_number = optional_text(document_number, "El documento", MAX_DOC_NUMBER)?;
    match (&document_type, &document_number) {
        (Some(_), None) => {
            return Err(AppError::validation(
                "Indica el número de documento o quita el tipo",
            ))
        }
        (None, Some(_)) => return Err(AppError::validation("Elige el tipo de documento")),
        (Some(kind), Some(_)) if !DOCUMENT_TYPES.contains(&kind.as_str()) => {
            return Err(AppError::validation("El tipo de documento no es válido"))
        }
        _ => {}
    }

    let email = optional_text(email, "El email", MAX_EMAIL)?;
    if let Some(value) = &email {
        let valid = value.contains('@')
            && !value.starts_with('@')
            && !value.ends_with('@')
            && !value.contains(char::is_whitespace);
        if !valid {
            return Err(AppError::validation("El email no es válido"));
        }
    }

    Ok(Fields {
        name,
        document_type,
        document_number,
        phone: optional_text(phone, "El teléfono", MAX_PHONE)?,
        email,
        address: optional_text(address, "La dirección", MAX_TEXT)?,
        notes: optional_text(notes, "Las notas", MAX_TEXT)?,
    })
}

fn map_error(error: CustomerError) -> AppError {
    match error {
        CustomerError::Db(error) => AppError::from(error),
        CustomerError::NotFound => AppError::not_found("Cliente no encontrado"),
        CustomerError::DuplicateDocument => {
            AppError::validation("Ya existe un cliente con ese documento")
        }
    }
}

pub async fn create_customer(pool: &PgPool, dto: CreateCustomerDto) -> Result<Customer, AppError> {
    let fields = normalize(
        dto.name,
        dto.document_type,
        dto.document_number,
        dto.phone,
        dto.email,
        dto.address,
        dto.notes,
    )?;
    repository::create_customer(
        pool,
        CreateCustomerDto {
            name: fields.name,
            document_type: fields.document_type,
            document_number: fields.document_number,
            phone: fields.phone,
            email: fields.email,
            address: fields.address,
            notes: fields.notes,
            created_by: dto.created_by,
        },
    )
    .await
    .map_err(map_error)
}

pub async fn update_customer(
    pool: &PgPool,
    id: i32,
    dto: UpdateCustomerDto,
) -> Result<Customer, AppError> {
    let fields = normalize(
        dto.name,
        dto.document_type,
        dto.document_number,
        dto.phone,
        dto.email,
        dto.address,
        dto.notes,
    )?;
    repository::update_customer(
        pool,
        id,
        UpdateCustomerDto {
            name: fields.name,
            document_type: fields.document_type,
            document_number: fields.document_number,
            phone: fields.phone,
            email: fields.email,
            address: fields.address,
            notes: fields.notes,
            updated_by: dto.updated_by,
        },
    )
    .await
    .map_err(map_error)
}

pub async fn set_customer_active(
    pool: &PgPool,
    id: i32,
    dto: SetCustomerActiveDto,
) -> Result<Customer, AppError> {
    repository::set_customer_active(pool, id, dto)
        .await
        .map_err(map_error)
}

pub async fn get_customer(pool: &PgPool, id: i32) -> Result<CustomerOverview, AppError> {
    repository::get_overview(pool, id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("Cliente no encontrado"))
}

pub async fn list_customers(
    pool: &PgPool,
    mut filter: CustomerFilterDto,
) -> Result<PaginatedResponse<CustomerOverview>, AppError> {
    filter.search = optional_text(filter.search.take(), "La búsqueda", MAX_SEARCH)?;
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(20).clamp(1, 100);
    filter.page = Some(page);
    filter.page_size = Some(page_size);

    let (data, total) = repository::list_customers(pool, &filter)
        .await
        .map_err(AppError::from)?;
    Ok(PaginatedResponse::new(data, total, page, page_size))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normalize_test(
        name: &str,
        kind: Option<&str>,
        number: Option<&str>,
        email: Option<&str>,
    ) -> Result<Fields, AppError> {
        normalize(
            name.to_string(),
            kind.map(str::to_string),
            number.map(str::to_string),
            None,
            email.map(str::to_string),
            None,
            None,
        )
    }

    #[test]
    fn name_is_required_and_trimmed() {
        assert!(normalize_test("   ", None, None, None).is_err());
        assert_eq!(
            normalize_test("  Ana  ", None, None, None).unwrap().name,
            "Ana"
        );
    }

    #[test]
    fn document_type_and_number_go_together() {
        assert!(normalize_test("Ana", Some("CC"), None, None).is_err());
        assert!(normalize_test("Ana", None, Some("123"), None).is_err());
        assert!(normalize_test("Ana", Some("XX"), Some("123"), None).is_err());
        let fields = normalize_test("Ana", Some("cc"), Some(" 123 "), None).unwrap();
        assert_eq!(fields.document_type.as_deref(), Some("CC"));
        assert_eq!(fields.document_number.as_deref(), Some("123"));
    }

    #[test]
    fn email_is_checked_and_blank_becomes_none() {
        assert!(normalize_test("Ana", None, None, Some("sin-arroba")).is_err());
        assert!(normalize_test("Ana", None, None, Some("a b@c.com")).is_err());
        assert_eq!(
            normalize_test("Ana", None, None, Some("  ")).unwrap().email,
            None
        );
        assert!(normalize_test("Ana", None, None, Some("ana@mail.com")).is_ok());
    }
}
