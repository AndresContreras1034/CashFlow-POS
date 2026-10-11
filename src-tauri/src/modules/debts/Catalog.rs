//! Catálogo de opciones del módulo: categorías, subcategorías sugeridas,
//! naturalezas, métodos de pago y estados, con sus etiquetas en español.
//!
//! Es la ÚNICA definición de estos textos. El frontend los pide con
//! `get_debt_options` y no los duplica. Las subcategorías son sugerencias:
//! el campo sigue siendo texto libre.

use serde::Serialize;

use crate::modules::debts::models::{DebtCategory, DebtNature, DebtPaymentMethod, DebtStatus};

#[derive(Debug, Clone, Serialize)]
pub struct LabeledOption<T> {
    pub value: T,
    pub label: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct CategoryOption {
    pub value: DebtCategory,
    pub label: &'static str,
    pub description: &'static str,
    pub subcategories: &'static [&'static str],
}

#[derive(Debug, Clone, Serialize)]
pub struct DebtOptions {
    pub categories: Vec<CategoryOption>,
    pub natures: Vec<LabeledOption<DebtNature>>,
    pub payment_methods: Vec<LabeledOption<DebtPaymentMethod>>,
    pub statuses: Vec<LabeledOption<DebtStatus>>,
}

const ALL_CATEGORIES: [DebtCategory; 4] = [
    DebtCategory::Maintenance,
    DebtCategory::BusinessOperation,
    DebtCategory::FixedServices,
    DebtCategory::OtherObligations,
];

const ALL_NATURES: [DebtNature; 4] = [
    DebtNature::Operating,
    DebtNature::FinancialLoan,
    DebtNature::Tax,
    DebtNature::Other,
];

const ALL_METHODS: [DebtPaymentMethod; 5] = [
    DebtPaymentMethod::Cash,
    DebtPaymentMethod::Card,
    DebtPaymentMethod::Transfer,
    DebtPaymentMethod::Check,
    DebtPaymentMethod::Other,
];

const ALL_STATUSES: [DebtStatus; 5] = [
    DebtStatus::Pending,
    DebtStatus::PartiallyPaid,
    DebtStatus::Overdue,
    DebtStatus::Paid,
    DebtStatus::Voided,
];

// Los `match` son exhaustivos a propósito: añadir una variante a un enum
// obliga a darle etiqueta aquí, o el proyecto no compila.

fn category_option(category: DebtCategory) -> CategoryOption {
    match category {
        DebtCategory::Maintenance => CategoryOption {
            value: category,
            label: "Mantenimiento",
            description: "Reparaciones, instalaciones, equipos y conservación del establecimiento",
            subcategories: &[
                "Reparaciones",
                "Instalaciones",
                "Equipos",
                "Conservación del local",
            ],
        },
        DebtCategory::BusinessOperation => CategoryOption {
            value: category,
            label: "Operación del negocio",
            description: "Mercancía, insumos y obligaciones operativas",
            subcategories: &["Mercancía", "Insumos", "Obligaciones operativas"],
        },
        DebtCategory::FixedServices => CategoryOption {
            value: category,
            label: "Servicios y gastos fijos",
            description: "Arriendo, electricidad, agua, internet y otros servicios recurrentes",
            subcategories: &[
                "Arriendo",
                "Electricidad",
                "Agua",
                "Internet",
                "Otros servicios",
            ],
        },
        DebtCategory::OtherObligations => CategoryOption {
            value: category,
            label: "Otras obligaciones",
            description: "Préstamos y compromisos que no encajan en las demás categorías",
            subcategories: &["Préstamos", "Compromisos varios"],
        },
    }
}

fn nature_label(nature: DebtNature) -> &'static str {
    match nature {
        DebtNature::Operating => "Operativa",
        DebtNature::FinancialLoan => "Préstamo financiero",
        DebtNature::Tax => "Impuestos",
        DebtNature::Other => "Otra",
    }
}

fn method_label(method: DebtPaymentMethod) -> &'static str {
    match method {
        DebtPaymentMethod::Cash => "Efectivo",
        DebtPaymentMethod::Card => "Tarjeta",
        DebtPaymentMethod::Transfer => "Transferencia",
        DebtPaymentMethod::Check => "Cheque",
        DebtPaymentMethod::Other => "Otro",
    }
}

fn status_label(status: DebtStatus) -> &'static str {
    match status {
        DebtStatus::Pending => "Pendiente",
        DebtStatus::PartiallyPaid => "Pago parcial",
        DebtStatus::Overdue => "Vencida",
        DebtStatus::Paid => "Pagada",
        DebtStatus::Voided => "Anulada",
    }
}

pub fn options() -> DebtOptions {
    DebtOptions {
        categories: ALL_CATEGORIES.into_iter().map(category_option).collect(),
        natures: ALL_NATURES
            .into_iter()
            .map(|value| LabeledOption {
                value,
                label: nature_label(value),
            })
            .collect(),
        payment_methods: ALL_METHODS
            .into_iter()
            .map(|value| LabeledOption {
                value,
                label: method_label(value),
            })
            .collect(),
        statuses: ALL_STATUSES
            .into_iter()
            .map(|value| LabeledOption {
                value,
                label: status_label(value),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_values_like_the_database_and_has_every_option() {
        let json = serde_json::to_value(options()).unwrap();
        assert_eq!(json["categories"].as_array().unwrap().len(), 4);
        assert_eq!(json["categories"][1]["value"], "business_operation");
        assert_eq!(json["natures"].as_array().unwrap().len(), 4);
        assert_eq!(json["payment_methods"].as_array().unwrap().len(), 5);
        assert_eq!(json["statuses"].as_array().unwrap().len(), 5);
        assert_eq!(json["statuses"][1]["value"], "partially_paid");
    }

    #[test]
    fn suggested_subcategories_are_nonempty_and_fit_the_column() {
        for category in options().categories {
            assert!(!category.subcategories.is_empty());
            for sub in category.subcategories {
                assert!(!sub.trim().is_empty());
                assert!(sub.chars().count() <= 60, "{sub}");
            }
        }
    }

    #[test]
    fn labels_are_unique_within_each_list() {
        let o = options();
        let mut labels: Vec<_> = o.categories.iter().map(|c| c.label).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), 4);
        let mut labels: Vec<_> = o.statuses.iter().map(|s| s.label).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), 5);
    }
}