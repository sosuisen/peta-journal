pub mod date;
pub mod filename;
pub mod week;

/// True when the text has an odd number of lines that open a code fence.
pub fn has_unclosed_code_fence(text: &str) -> bool {
    text.split('\n')
        .filter(|line| line.trim_start().starts_with("```"))
        .count()
        % 2
        != 0
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
    text.char_indices()
        .nth(char_index)
        .map(|(i, _)| i)
        .unwrap_or(text.len())
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

    // 閉じていないコードフェンスは ``` で始まる行が奇数個あることで検出され、閉じフェンスが補われる。
    #[test]
    fn code_fences() {
        assert!(has_unclosed_code_fence("```js\nlet a"));
        assert!(!has_unclosed_code_fence("```js\nlet a\n```"));
        assert_eq!(ensure_code_fence_closed("```"), "```\n```");
        assert_eq!(ensure_code_fence_closed("x"), "x");
    }

    // カーソルの下のウィキリンクとタグを、文字数の位置で読み取る。リンクの両端もリンクの上である。
    #[test]
    fn link_at_finds_the_wiki_link_or_the_tag_around_the_cursor() {
        let text = "a [[メモ/本]] b #rust c";
        let wiki = || Some(Link::Wiki("メモ/本".into()));
        assert_eq!(link_at(text, 1), None);
        assert_eq!(link_at(text, 2), wiki());
        assert_eq!(link_at(text, 5), wiki());
        assert_eq!(link_at(text, 11), wiki());
        assert_eq!(link_at(text, 12), None);
        assert_eq!(link_at(text, 15), Some(Link::Tag("rust".into())));
    }

    // ウィキリンクの中の # はタグではない。リンクの名前の前後の空白は名前に含めない。
    #[test]
    fn link_at_reads_the_whole_wiki_link_and_trims_its_name() {
        assert_eq!(link_at("[[a#b]]", 3), Some(Link::Wiki("a#b".into())));
        assert_eq!(link_at("[[ a ]]", 3), Some(Link::Wiki("a".into())));
    }

    // 名前のないリンク、閉じていないリンク、行をまたぐリンクは、リンクではない。
    #[test]
    fn link_at_ignores_what_is_not_a_link() {
        assert_eq!(link_at("[[ ]]", 2), None);
        assert_eq!(link_at("[[a", 2), None);
        assert_eq!(link_at("[[a\nb]]", 2), None);
        assert_eq!(link_at("# title", 0), None);
    }

    // テキストは文字インデックスで分割され、範囲外のインデックスは末尾に丸められ、長さは文字数で数えられる。
    #[test]
    fn char_splitting() {
        assert_eq!(split_at_char("日本語", 1), ("日", "本語"));
        assert_eq!(split_at_char("ab", 9), ("ab", ""));
        assert_eq!(char_len("日本語"), 3);
    }
}
