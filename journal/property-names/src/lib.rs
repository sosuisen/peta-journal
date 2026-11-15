//! The type of a property follows from the suffix of its key name, so every device decides
//! it alike without a word between them (ADR 047).

/// The type of the values of a property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    Text,
    Number,
    Date,
    Datetime,
    Checkbox,
    Bounds,
    List,
}

/// The suffix of the key names of each type but text, whose names have none.
const SUFFIXES: [(&str, ValueType); 6] = [
    ("_n", ValueType::Number),
    ("_d", ValueType::Date),
    ("_dt", ValueType::Datetime),
    ("_cb", ValueType::Checkbox),
    ("_bounds", ValueType::Bounds),
    ("_list", ValueType::List),
];

/// The type of the property with this key name.
pub fn type_of(name: &str) -> ValueType {
    SUFFIXES
        .iter()
        .find(|(suffix, _)| name.ends_with(suffix))
        .map_or(ValueType::Text, |(_, value_type)| *value_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 接尾辞のない名前はtext型である。
    #[test]
    fn a_name_without_a_suffix_is_text() {
        assert_eq!(type_of("rating"), ValueType::Text);
    }

    // 接尾辞のある名前は、その接尾辞の型である。
    #[test]
    fn a_name_with_a_suffix_is_of_the_type_of_the_suffix() {
        let names = [
            ("rating_n", ValueType::Number),
            ("start_d", ValueType::Date),
            ("start_dt", ValueType::Datetime),
            ("read_cb", ValueType::Checkbox),
            ("place_bounds", ValueType::Bounds),
            ("member_list", ValueType::List),
        ];
        for (name, value_type) in names {
            assert_eq!(type_of(name), value_type, "{name}");
        }
    }

    // 大文字の接尾辞は本体の一部で、その名前はtext型である。
    #[test]
    fn a_suffix_in_capitals_is_a_part_of_the_body_and_the_name_is_text() {
        assert_eq!(type_of("rating_N"), ValueType::Text);
    }

    // 型は最後の接尾辞だけで決まる。
    #[test]
    fn only_the_last_suffix_decides_the_type() {
        assert_eq!(type_of("rating_n_d"), ValueType::Date);
    }

    // 本体と型から、完全な名前ができる。
    #[test]
    fn a_body_and_a_type_make_the_full_name() {
        assert_eq!(name_of("rating", ValueType::Number), Ok("rating_n".to_string()));
        assert_eq!(name_of("rating", ValueType::Text), Ok("rating".to_string()));
    }
}
