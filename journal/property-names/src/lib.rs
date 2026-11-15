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

impl ValueType {
    /// The types whose key names end with a suffix: all but text.
    const SUFFIXED: [ValueType; 6] = [
        ValueType::Number,
        ValueType::Date,
        ValueType::Datetime,
        ValueType::Checkbox,
        ValueType::Bounds,
        ValueType::List,
    ];

    /// The suffix of the key names of the type. Text has none.
    pub fn suffix(self) -> &'static str {
        match self {
            ValueType::Text => "",
            ValueType::Number => "_n",
            ValueType::Date => "_d",
            ValueType::Datetime => "_dt",
            ValueType::Checkbox => "_cb",
            ValueType::Bounds => "_bounds",
            ValueType::List => "_list",
        }
    }
}

/// The type of the property with this key name.
pub fn type_of(name: &str) -> ValueType {
    ValueType::SUFFIXED
        .into_iter()
        .find(|value_type| name.ends_with(value_type.suffix()))
        .unwrap_or(ValueType::Text)
}

/// The full key name of a property of this type, whose body the user gives.
pub fn name_of(body: &str, value_type: ValueType) -> Result<String, ReservedSuffix> {
    if type_of(body) != ValueType::Text {
        return Err(ReservedSuffix);
    }
    Ok(format!("{body}{}", value_type.suffix()))
}

/// The body of the key name, which the user sees: the name without its suffix.
pub fn body_of(name: &str) -> &str {
    &name[..name.len() - type_of(name).suffix().len()]
}

/// A body that ends with a suffix, which would make a name such as `rating_n_n`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservedSuffix;

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

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
        assert_eq!(
            name_of("rating", ValueType::Number),
            Ok("rating_n".to_string())
        );
        assert_eq!(name_of("rating", ValueType::Text), Ok("rating".to_string()));
    }

    // 予約の接尾辞で終わる本体は、どの型でも拒まれる。
    #[test]
    fn a_body_that_ends_with_a_reserved_suffix_is_refused_for_any_type() {
        assert_eq!(name_of("rating_n", ValueType::Number), Err(ReservedSuffix));
        assert_eq!(name_of("member_list", ValueType::Text), Err(ReservedSuffix));
    }

    // 完全な名前から、接尾辞を除いた本体が取り出せる。
    #[test]
    fn the_body_comes_out_of_the_full_name_without_the_suffix() {
        assert_eq!(body_of("rating_n"), "rating");
        assert_eq!(body_of("start_dt"), "start");
        assert_eq!(body_of("rating"), "rating");
    }

    // 型の名前は、DBとAPIで使う名前である。
    #[test]
    fn each_type_has_the_name_the_db_and_the_api_use() {
        let names = [
            (ValueType::Text, "text"),
            (ValueType::Number, "number"),
            (ValueType::Date, "date"),
            (ValueType::Datetime, "datetime"),
            (ValueType::Checkbox, "checkbox"),
            (ValueType::Bounds, "bounds"),
            (ValueType::List, "list"),
        ];
        for (value_type, name) in names {
            assert_eq!(value_type.as_str(), name);
        }
    }

    fn any_type() -> impl Strategy<Value = ValueType> {
        prop::sample::select(vec![
            ValueType::Text,
            ValueType::Number,
            ValueType::Date,
            ValueType::Datetime,
            ValueType::Checkbox,
            ValueType::Bounds,
            ValueType::List,
        ])
    }

    fn any_body() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9_-]{1,16}".prop_filter("a body takes no reserved suffix", |body| {
            type_of(body) == ValueType::Text
        })
    }

    proptest! {
        // 本体と型から作った名前は、その型と本体に戻る。
        #[test]
        fn a_name_made_of_a_body_and_a_type_gives_them_back(body in any_body(), value_type in any_type()) {
            let name = name_of(&body, value_type).unwrap();
            prop_assert_eq!(type_of(&name), value_type);
            prop_assert_eq!(body_of(&name), body.as_str());
        }
    }
}
