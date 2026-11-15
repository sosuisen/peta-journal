//! The type of a property follows from the suffix of its key name, so every device decides
//! it alike without a word between them (ADR 047).

/// The type of the values of a property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    Text,
}

/// The type of the property with this key name.
pub fn type_of(_name: &str) -> ValueType {
    ValueType::Text
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
}
