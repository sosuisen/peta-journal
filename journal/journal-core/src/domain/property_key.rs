//! The key of a property, which tells by its name whether its values are a list (ADR 046).

use crate::error::{AppError, AppResult};

/// The suffix of the name of every list key a user defines.
pub const LIST_SUFFIX: &str = "_list";

/// A key of a property, parsed from its name once at the boundary. Whether its values are a
/// list follows from the name, so every device decides it alike, before any definition
/// arrives (ADR 046).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyKey {
    Scalar(ScalarKey),
    List(ListKey),
}

/// A key of one value. Its name does not end with `_list` and is not `tags`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScalarKey(String);

/// A key whose values are a list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListKey {
    /// The system key of the tags.
    Tags,
    /// A key of the user, whose name ends with `_list`.
    User(UserListKey),
}

/// A list key of the user. Its name ends with `_list`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserListKey(String);

impl PropertyKey {
    /// The key of the name. A name with a letter a key cannot have is refused.
    pub fn parse(name: &str) -> AppResult<PropertyKey> {
        validate_key(name)?;
        Ok(if name == "tags" {
            PropertyKey::List(ListKey::Tags)
        } else if name.ends_with(LIST_SUFFIX) {
            PropertyKey::List(ListKey::User(UserListKey(name.to_string())))
        } else {
            PropertyKey::Scalar(ScalarKey(name.to_string()))
        })
    }

    pub fn as_str(&self) -> &str {
        match self {
            PropertyKey::Scalar(ScalarKey(name)) => name,
            PropertyKey::List(ListKey::Tags) => "tags",
            PropertyKey::List(ListKey::User(UserListKey(name))) => name,
        }
    }
}

/// A key is a word of letters, digits, `_` and `-`: it is written unquoted in the metadata
/// line of an item and as a YAML key in the front matter of a page.
pub fn validate_key(key: &str) -> AppResult<()> {
    if key.is_empty() {
        return Err(AppError::bad_request("Property key is required"));
    }
    if !key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(AppError::bad_request(format!(
            "Property key '{key}' may only contain letters, digits, '_' and '-'"
        )));
    }
    Ok(())
}

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
