//! The type of a property follows from the suffix of its key name, so every device decides
//! it alike without a word between them (ADR 047).

#[cfg(test)]
mod tests {
    use super::*;

    // 接尾辞のない名前はtext型である。
    #[test]
    fn a_name_without_a_suffix_is_text() {
        assert_eq!(type_of("rating"), ValueType::Text);
    }
}
