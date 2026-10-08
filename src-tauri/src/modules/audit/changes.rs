use serde::Serialize;
use serde_json::{json, Map, Value as JsonValue};

/// Los textos más largos que esto se guardan resumidos en `changes`
/// (por ejemplo, un logo en base64 o una descripción enorme), para que un
/// solo cambio no infle la tabla de auditoría.
const MAX_VALUE_CHARS: usize = 500;

fn shorten(value: &JsonValue) -> JsonValue {
    match value {
        JsonValue::String(text) if text.chars().count() > MAX_VALUE_CHARS => {
            json!(format!("[texto de {} caracteres]", text.chars().count()))
        }
        other => other.clone(),
    }
}

/// Compara dos versiones de la misma fila y devuelve solo los campos que
/// cambiaron, como `{"campo": {"from": .., "to": ..}}`. Vacío si no cambió
/// nada. `ignore` lista campos que no interesan (`id`, timestamps).
///
/// Cualquier columna nueva que se agregue al struct queda auditada por
/// defecto. La comparación se hace con los valores completos; solo el valor
/// que se guarda se resume si es muy largo.
///
/// Los structs del proyecto solo contienen tipos simples, así que serializar
/// no falla; si alguna vez no produjera un objeto JSON, devuelve vacío.
pub fn changed_fields<T: Serialize>(
    before: &T,
    after: &T,
    ignore: &[&str],
) -> Map<String, JsonValue> {
    let (JsonValue::Object(before), JsonValue::Object(after)) = (
        serde_json::to_value(before).unwrap_or(JsonValue::Null),
        serde_json::to_value(after).unwrap_or(JsonValue::Null),
    ) else {
        return Map::new();
    };

    let mut changes = Map::new();
    for (field, from) in &before {
        if ignore.contains(&field.as_str()) {
            continue;
        }
        let to = after.get(field).unwrap_or(&JsonValue::Null);
        if from != to {
            changes.insert(
                field.clone(),
                json!({ "from": shorten(from), "to": shorten(to) }),
            );
        }
    }
    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct Row {
        id: i32,
        name: String,
        note: Option<String>,
        is_active: bool,
        updated_at: i64,
    }

    fn row() -> Row {
        Row {
            id: 1,
            name: "Perfumes".to_string(),
            note: None,
            is_active: true,
            updated_at: 100,
        }
    }

    #[test]
    fn identical_rows_have_no_changes() {
        assert!(changed_fields(&row(), &row(), &["id", "updated_at"]).is_empty());
    }

    #[test]
    fn reports_only_changed_fields_with_before_and_after() {
        let mut after = row();
        after.name = "Fragancias".to_string();
        after.note = Some("nueva".to_string());

        let changes = changed_fields(&row(), &after, &["id", "updated_at"]);

        assert_eq!(changes.len(), 2);
        assert_eq!(
            changes["name"],
            json!({ "from": "Perfumes", "to": "Fragancias" })
        );
        assert_eq!(changes["note"], json!({ "from": null, "to": "nueva" }));
    }

    #[test]
    fn ignored_fields_are_not_reported() {
        let mut after = row();
        after.id = 2;
        after.updated_at = 999;

        assert!(changed_fields(&row(), &after, &["id", "updated_at"]).is_empty());
        assert_eq!(changed_fields(&row(), &after, &[]).len(), 2);
    }

    #[test]
    fn long_text_is_summarized_but_still_compared_in_full() {
        let mut before = row();
        let mut after = row();
        before.note = Some("a".repeat(800));
        after.note = Some(format!("{}b", "a".repeat(799)));

        let changes = changed_fields(&before, &after, &["id", "updated_at"]);

        assert_eq!(
            changes["note"],
            json!({
                "from": "[texto de 800 caracteres]",
                "to": "[texto de 800 caracteres]"
            })
        );

        let same = Row {
            note: Some("a".repeat(800)),
            ..row()
        };
        assert!(changed_fields(&same, &same, &["id", "updated_at"]).is_empty());
    }
}
