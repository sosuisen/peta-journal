//! What the settings screen of the web shell shows of the property definitions.

use client_core::model::{shown_name, PropertyDefinition, USER_VALUE_TYPES};

/// The names the screen shows for the keys of `definitions`, given as JSON, in their order.
pub fn shown_names(definitions: &str) -> Result<Vec<String>, String> {
    let definitions: Vec<PropertyDefinition> = crate::border::read(definitions)?;
    Ok(definitions
        .iter()
        .map(|d| shown_name(&d.key, &definitions))
        .collect())
}

/// The types a user may give a new key, in the order the settings offer them.
pub fn user_value_types() -> Vec<String> {
    USER_VALUE_TYPES
        .iter()
        .map(|t| t.as_str().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // キーは、型の印を除いた名前で出す。同じ名前のキーが2つあれば、型を添えて見分ける。システムのキーは、そのまま出す。
    #[test]
    fn keys_show_without_their_type_and_the_type_tells_two_alike_apart() {
        let definitions = r#"[
            {"key":"rating_n","value_type":"number","system":false,"page":false},
            {"key":"rating","value_type":"text","system":false,"page":false},
            {"key":"due_d","value_type":"date","system":false,"page":true},
            {"key":"atlas_bounds","value_type":"bounds","system":true,"page":true}
        ]"#;

        assert_eq!(
            shown_names(definitions),
            Ok(vec![
                "rating (number)".to_string(),
                "rating (text)".to_string(),
                "due".to_string(),
                "atlas_bounds".to_string(),
            ])
        );
    }

    // 読めない定義は、理由とともに断る。
    #[test]
    fn unreadable_definitions_are_refused() {
        assert!(shown_names("[{").is_err());
    }

    // 新しいキーに選べる型は、設定に並べる順である。
    #[test]
    fn the_types_of_a_new_key_are_in_the_order_of_the_settings() {
        assert_eq!(
            user_value_types(),
            ["text", "number", "checkbox", "date", "datetime", "bounds", "list"]
        );
    }
}
