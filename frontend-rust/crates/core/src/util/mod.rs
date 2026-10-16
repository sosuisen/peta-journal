pub mod date;
pub mod filename;
pub mod week;

/// True when the text has an odd number of lines that open a code fence.
pub fn has_unclosed_code_fence(text: &str) -> bool {
    text.split('\n').filter(|line| line.trim_start().starts_with("```")).count() % 2 != 0
}

/// Closes an open code fence, so the saved body never ends inside one.
pub fn ensure_code_fence_closed(text: &str) -> String {
    if has_unclosed_code_fence(text) {
        format!("{text}\n```")
    } else {
        text.to_string()
    }
}

/// The byte offset of a character index, clamped to the text. Cursor positions arrive from
/// the shell as UTF-16 code units, which the shell converts to character counts first.
pub fn byte_offset(text: &str, char_index: usize) -> usize {
    text.char_indices().nth(char_index).map(|(i, _)| i).unwrap_or(text.len())
}

/// Splits a text at a character index.
pub fn split_at_char(text: &str, char_index: usize) -> (&str, &str) {
    text.split_at(byte_offset(text, char_index))
}

/// The number of characters in a text; cursor positions count characters.
pub fn char_len(text: &str) -> usize {
    text.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_fences() {
        assert!(has_unclosed_code_fence("```js\nlet a"));
        assert!(!has_unclosed_code_fence("```js\nlet a\n```"));
        assert_eq!(ensure_code_fence_closed("```"), "```\n```");
        assert_eq!(ensure_code_fence_closed("x"), "x");
    }

    #[test]
    fn char_splitting() {
        assert_eq!(split_at_char("日本語", 1), ("日", "本語"));
        assert_eq!(split_at_char("ab", 9), ("ab", ""));
        assert_eq!(char_len("日本語"), 3);
    }
}
