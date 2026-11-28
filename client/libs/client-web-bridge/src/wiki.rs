//! The `[[wiki link]]` completion for the web shell: what is typed inside the brackets.

use client_core::util::utf16::chars_of_utf16;

/// The text between `[[` and the cursor at the UTF-16 place `cursor`, while the cursor is
/// inside a `[[...]]`. None outside one.
pub fn query_at(text: &str, cursor: u64) -> Option<String> {
    let units = usize::try_from(cursor).unwrap_or(usize::MAX);
    client_core::ops::wiki::query_at(text, chars_of_utf16(text, units))
}

#[cfg(test)]
mod tests {
    use super::*;

    // カーソルが[[と]]の間にあれば、[[からカーソルまでの文字を答える。位置はUTF-16で数える。
    #[test]
    fn inside_the_brackets_the_text_up_to_the_cursor_is_the_query() {
        let text = "👍[[no]]";

        assert_eq!(query_at(text, 6), Some("no".into()));
        assert_eq!(query_at(text, 5), Some("n".into()));
    }

    // 括弧の外では、何も答えない。
    #[test]
    fn outside_the_brackets_there_is_no_query() {
        assert_eq!(query_at("ab [[no]]", 1), None);
    }

    // 選んだページ名を入れた文と、その後のカーソルを答える。位置はUTF-16で数える。リンクの外では何も答えない。
    #[test]
    fn a_picked_name_gives_the_text_and_the_cursor_after_it_in_utf16() {
        assert_eq!(
            picked_at("👍[[no]]", 5, "Notes"),
            Some(("👍[[Notes]]".into(), 9))
        );
        assert_eq!(picked_at("ab [[no]]", 1, "Notes"), None);
    }
}
