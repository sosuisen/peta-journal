//! How the dates of an answer are written (ADR 060). A REST route takes the format as the
//! query `dateFormat`, and an MCP tool as the argument of the same name.

use journal_core::infrastructure::export::export_service::format_instant;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

/// The fields of a page and of an item that hold a date.
const DATE_FIELDS: [&str; 2] = ["createdDate", "modifiedDate"];

/// The values of the properties are what the user wrote. A key of a user may have the name of
/// a date field, so nothing below this field is rewritten.
const PROPERTIES_FIELD: &str = "properties";

/// How an answer writes its dates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DateFormat {
    /// Milliseconds since 1970-01-01 UTC, as a number.
    #[default]
    Millis,
    /// UTC ISO 8601 text, such as `2026-11-11T10:00:00Z`.
    Iso,
}

impl DateFormat {
    /// The JSON of an answer with its dates written in this format.
    pub fn write(self, mut answer: Value) -> Value {
        if self == DateFormat::Iso {
            write_iso(&mut answer);
        }
        answer
    }
}

fn write_iso(value: &mut Value) {
    match value {
        Value::Array(values) => values.iter_mut().for_each(write_iso),
        Value::Object(fields) => {
            for (name, field) in fields {
                match field.as_i64() {
                    Some(millis) if DATE_FIELDS.contains(&name.as_str()) => {
                        *field = Value::String(format_instant(millis));
                    }
                    _ if name != PROPERTIES_FIELD => write_iso(field),
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn answer() -> Value {
        json!({
            "page": { "id": "p", "createdDate": 1_762_855_200_000_i64, "modifiedDate": 1_762_855_200_500_i64 },
            "items": [{ "id": "i", "createdDate": 0, "modifiedDate": 1000, "level": 1 }],
        })
    }

    // ISOの形式では、ページと項目の日付がUTCのISO 8601の文字列になる。ほかの数は変わらない。
    #[test]
    fn iso_writes_the_dates_of_pages_and_items_as_utc_text() {
        assert_eq!(
            DateFormat::Iso.write(answer()),
            json!({
                "page": { "id": "p", "createdDate": "2025-11-11T10:00:00Z", "modifiedDate": "2025-11-11T10:00:00.500Z" },
                "items": [{ "id": "i", "createdDate": "1970-01-01T00:00:00Z", "modifiedDate": "1970-01-01T00:00:01Z", "level": 1 }],
            })
        );
    }

    // ミリ秒の形式では、答えは変わらない。
    #[test]
    fn millis_leaves_the_answer_as_it_is() {
        assert_eq!(DateFormat::Millis.write(answer()), answer());
    }

    // 利用者のプロパティは、日付の欄と同じ名前でも書き換えられない。
    #[test]
    fn a_property_with_the_name_of_a_date_field_is_not_rewritten() {
        let item = json!({ "modifiedDate": 0, "properties": { "createdDate": 5 } });

        assert_eq!(
            DateFormat::Iso.write(item),
            json!({ "modifiedDate": "1970-01-01T00:00:00Z", "properties": { "createdDate": 5 } })
        );
    }

    // 形式は、名前のない既定がミリ秒で、名前は小文字で読まれる。知らない名前は拒否される。
    #[test]
    fn the_format_is_read_by_its_lower_case_name_and_an_unknown_name_is_refused() {
        let read = |name: &str| serde_json::from_value::<DateFormat>(json!(name));

        assert_eq!(DateFormat::default(), DateFormat::Millis);
        assert_eq!(read("iso").unwrap(), DateFormat::Iso);
        assert_eq!(read("millis").unwrap(), DateFormat::Millis);
        assert!(read("rfc").is_err());
    }
}
