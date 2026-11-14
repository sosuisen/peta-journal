//! The key of a property, which tells by its name whether its values are a list (ADR 046).

#[cfg(test)]
mod tests {
    use super::*;

    // tagsは、リストのキーである。
    #[test]
    fn tags_is_a_list_key() {
        assert_eq!(
            PropertyKey::parse("tags").unwrap(),
            PropertyKey::List(ListKey::Tags)
        );
    }

    // 名前が_listで終わるキーは、ユーザー定義のリストのキーである。
    #[test]
    fn a_key_whose_name_ends_with_list_is_a_list_key_of_the_user() {
        let key = PropertyKey::parse("member_list").unwrap();

        assert!(matches!(key, PropertyKey::List(ListKey::User(_))));
        assert_eq!(key.as_str(), "member_list");
    }

    // 名前が_listで終わらないキーは、スカラーのキーである。
    #[test]
    fn a_key_whose_name_does_not_end_with_list_is_a_scalar_key() {
        let key = PropertyKey::parse("score").unwrap();

        assert!(matches!(key, PropertyKey::Scalar(_)));
        assert_eq!(key.as_str(), "score");
    }

    // キーに使えない文字のある名前は、キーにならない。
    #[test]
    fn a_name_with_a_letter_a_key_cannot_have_is_no_key() {
        assert!(PropertyKey::parse("my key").is_err());
    }
}
