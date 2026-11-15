pub mod date;
pub mod filename;
pub mod utf16;
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

/// What a `[[`, an opening code fence or a closing `]` just typed becomes: the body with its
/// closing part added or typed over, and the cursor. `cursor` is the character index right after
/// the typed character. None when the body stays as it is.
pub fn auto_close(body: &str, cursor: usize) -> Option<(String, usize)> {
    let (before, after) = split_at_char(body, cursor);
    if before.ends_with("[[") && !after.starts_with("]]") {
        return Some((format!("{before}]]{after}"), cursor));
    }
    if types_over_link_closing(before, after) {
        return Some((format!("{before}{}", &after[1..]), cursor));
    }
    opens_block(before, after).then(|| (flatmarkdown::close_blocks(before), cursor))
}

/// True when the character at the end of `before` was the last one of the opening of a new
/// block, with nothing after it. Only then is the block closed as it is typed (ADR 050): a
/// closing added in the middle of a body could pair with the marks around it.
fn opens_block(before: &str, after: &str) -> bool {
    let Some((typed_at, _)) = before.char_indices().last() else {
        return false;
    };
    after.is_empty()
        && flatmarkdown::has_unclosed_block(before)
        && !flatmarkdown::has_unclosed_block(&before[..typed_at])
}

/// True when the `]` at the end of `before` was typed right in front of the rest of the `]]` that
/// closes an open wiki link.
fn types_over_link_closing(before: &str, after: &str) -> bool {
    let Some(typed_at) = before.strip_suffix(']') else {
        return false;
    };
    let rest = if typed_at.ends_with(']') { "]" } else { "]]" };
    let line = last_line(typed_at);
    let in_link = line
        .rfind("[[")
        .is_some_and(|open| !line[open..].contains("]]"));
    in_link && after.starts_with(rest)
}

fn last_line(text: &str) -> &str {
    text.rsplit('\n').next().unwrap_or(text)
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

/// What a link in a body leads to.
#[derive(Debug, PartialEq, Eq)]
pub enum Link {
    Wiki(String),
    Tag(String),
}

/// The `[[wiki link]]` or `#tag` around a character index, its two ends included.
pub fn link_at(text: &str, cursor: usize) -> Option<Link> {
    let chars: Vec<char> = text.chars().collect();
    let mut start = 0;
    while start < chars.len() && start <= cursor {
        match link_starting(&chars[start..]) {
            Some((len, link)) if cursor <= start + len => return Some(link),
            Some((len, _)) => start += len,
            None => start += 1,
        }
    }
    None
}

/// The link that starts these characters, with its length.
fn link_starting(chars: &[char]) -> Option<(usize, Link)> {
    if chars.starts_with(&['[', '[']) {
        let rest = &chars[2..];
        let len = rest.iter().position(|c| matches!(c, ']' | '\n'))?;
        if !rest[len..].starts_with(&[']', ']']) {
            return None;
        }
        let name: String = rest[..len].iter().collect();
        let name = name.trim();
        (!name.is_empty()).then(|| (len + 4, Link::Wiki(name.to_string())))
    } else if chars.first() == Some(&'#') {
        let rest = &chars[1..];
        let len = rest
            .iter()
            .position(|c| matches!(c, ' ' | '\t' | '\n' | '#'))
            .unwrap_or(rest.len());
        (len > 0).then(|| (len + 1, Link::Tag(rest[..len].iter().collect())))
    } else {
        None
    }
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

    // `[[`を打つと`]]`が補われ、カーソルはその間に留まる。すでに`]]`が続いていれば補わない。
    #[test]
    fn auto_close_closes_a_wiki_link_just_opened() {
        assert_eq!(auto_close("日[[後", 3), Some(("日[[]]後".into(), 3)));
        assert_eq!(auto_close("[[]]", 2), None);
        assert_eq!(auto_close("[", 1), None);
    }

    // リンクの中で閉じる`]`を打つと、続く`]`の上を進み、`]`は増えない。
    #[test]
    fn auto_close_types_over_the_closing_of_a_wiki_link() {
        assert_eq!(auto_close("日[[本]]]後", 5), Some(("日[[本]]後".into(), 5)));
        assert_eq!(auto_close("日[[本]]]後", 6), Some(("日[[本]]後".into(), 6)));
        assert_eq!(auto_close("[[]]]", 3), Some(("[[]]".into(), 3)));
    }

    // リンクの外、閉じたリンクの後、別の行、閉じる位置でないところの`]`は、そのまま残る。
    #[test]
    fn auto_close_leaves_a_bracket_outside_the_closing_of_a_link() {
        assert_eq!(auto_close("a]]", 2), None);
        assert_eq!(auto_close("[[a]] b]]]", 7), None);
        assert_eq!(auto_close("[[a\n]]]", 5), None);
        assert_eq!(auto_close("[[a]]b]]", 4), None);
        assert_eq!(auto_close("[[a]]", 5), None);
    }

    // 行頭で開始記号を打ち終え、後ろに本文が何もなければ、閉じる記号が補われ、カーソルは留まる。どの開始記号でも同じである（ADR 050）。
    #[test]
    fn auto_close_closes_a_block_just_opened_at_the_end() {
        for (body, closed) in [
            ("```", "```\n```"),
            ("a\n  ~~~", "a\n  ~~~\n~~~"),
            (">>>", ">>>\n>>>"),
            ("<!--", "<!--\n-->"),
            ("<pre", "<pre\n</pre>"),
        ] {
            let cursor = body.chars().count();
            assert_eq!(
                auto_close(body, cursor),
                Some((closed.into(), cursor)),
                "{body:?}"
            );
        }
    }

    // 閉じるフェンスを打ったときや、行の途中のバッククォートでは、何も補わない。
    #[test]
    fn auto_close_leaves_a_closed_fence_and_plain_backticks() {
        assert_eq!(auto_close("```\na\n```", 9), None);
        assert_eq!(auto_close("a```", 4), None);
        assert_eq!(auto_close("``", 2), None);
    }

    // カーソルの下のウィキリンクとタグを、文字数の位置で読み取る。リンクの両端もリンクの上である。
    #[test]
    fn link_at_finds_the_wiki_link_or_the_tag_around_the_cursor() {
        let text = "a [[メモ/本]] b #rust c";
        let wiki = || Some(Link::Wiki("メモ/本".into()));
        assert_eq!(link_at(text, 1), None);
        assert_eq!(link_at(text, 2), wiki());
        assert_eq!(link_at(text, 5), wiki());
        assert_eq!(link_at(text, 10), wiki());
        assert_eq!(link_at(text, 11), None);
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
