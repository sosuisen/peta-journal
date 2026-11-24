pub mod date;
pub mod filename;
pub mod utf16;
pub mod week;

use crate::asts;
use flatmarkdown_ast::Node;

/// What a `[[`, the opening of a block or a closing `]` just typed becomes: the body with its
/// closing part added or typed over. `cursor` is the character index right after the typed
/// character. None when the body stays as it is.
///
/// The cursor stays where it is: only the text after it changes.
pub fn auto_close(body: &str, cursor: usize) -> Option<String> {
    let (before, after) = split_at_char(body, cursor);
    if before.ends_with("[[") && !after.starts_with("]]") {
        return Some(format!("{before}]]{after}"));
    }
    if types_over_link_closing(before, after) {
        return Some(format!("{before}{}", &after[1..]));
    }
    opens_block(before, after).then(|| flatmarkdown::close_blocks(before))
}

/// True when the character at the end of `before` was the last one of the opening of a new
/// block, with nothing after it. Only then is the block closed as it is typed (ADR 050): a
/// closing added in the middle of a body could pair with the marks around it.
fn opens_block(before: &str, after: &str) -> bool {
    let before_typed = before
        .char_indices()
        .last()
        .map_or("", |(typed_at, _)| &before[..typed_at]);
    after.is_empty()
        && flatmarkdown::has_unclosed_block(before)
        && !flatmarkdown::has_unclosed_block(before_typed)
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    Wiki(String),
    Tag(String),
}

/// The `[[wiki link]]` or `#tag` around a character index, its two ends included.
///
/// The parser of the bodies says what a link is and what its name is, so a link opens the
/// page that it shows. The parser gives no place in the text, so the place is found here: a
/// piece of the text that may be a link is read by the parser alone, and it counts only when
/// the whole body has the same link, which leaves out what stands in code.
pub fn link_at(text: &str, cursor: usize) -> Option<Link> {
    let chars: Vec<char> = text.chars().collect();
    let mut start = 0;
    let mut found = None;
    while start < chars.len() && start <= cursor {
        match link_starting(&chars, start) {
            Some((len, link)) if cursor <= start + len => {
                found = Some(link);
                break;
            }
            Some((len, _)) => start += len,
            None => start += 1,
        }
    }
    found.filter(|link| links_of(&asts::parse(text)).contains(link))
}

/// The link that starts at `start`, with its length in characters.
fn link_starting(chars: &[char], start: usize) -> Option<(usize, Link)> {
    let rest = &chars[start..];
    if rest.starts_with(&['[', '[']) {
        let inner = rest[2..].iter().position(|c| matches!(c, ']' | '\n'))?;
        if !rest[2 + inner..].starts_with(&[']', ']']) {
            return None;
        }
        let len = inner + 4;
        let written: String = rest[..len].iter().collect();
        match links_of(&asts::parse(&written)).as_slice() {
            [link @ Link::Wiki(_)] => Some((len, link.clone())),
            _ => None,
        }
    } else if rest.first() == Some(&'#') && (start == 0 || chars[start - 1] == ' ') {
        let end = rest
            .iter()
            .position(|c| c.is_whitespace())
            .unwrap_or(rest.len());
        let written: String = rest[..end].iter().collect();
        match links_of(&asts::parse(&written)).first() {
            Some(Link::Tag(tag)) => Some((char_len(tag) + 1, Link::Tag(tag.clone()))),
            _ => None,
        }
    } else {
        None
    }
}

/// The wiki links and the tags of a parsed body, in their order.
fn links_of(ast: &Node) -> Vec<Link> {
    let mut links = Vec::new();
    ast.walk(&mut |node| match node {
        Node::Wikilink { url, .. } => links.push(Link::Wiki(url.clone())),
        Node::Hashtag { value } => links.push(Link::Tag(value.clone())),
        _ => {}
    });
    links
}

#[cfg(test)]
mod tests {
    use super::*;

    // `[[`を打つと`]]`が補われる。すでに`]]`が続いていれば補わない。
    #[test]
    fn auto_close_closes_a_wiki_link_just_opened() {
        assert_eq!(auto_close("日[[後", 3), Some("日[[]]後".into()));
        assert_eq!(auto_close("[[]]", 2), None);
        assert_eq!(auto_close("[", 1), None);
    }

    // リンクの中で閉じる`]`を打つと、続く`]`の上を進み、`]`は増えない。
    #[test]
    fn auto_close_types_over_the_closing_of_a_wiki_link() {
        assert_eq!(auto_close("日[[本]]]後", 5), Some("日[[本]]後".into()));
        assert_eq!(auto_close("日[[本]]]後", 6), Some("日[[本]]後".into()));
        assert_eq!(auto_close("[[]]]", 3), Some("[[]]".into()));
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

    // 行頭で開始記号を打ち終え、後ろに本文が何もなければ、閉じる記号が補われる。どの開始記号でも同じである（ADR 050）。
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
            assert_eq!(auto_close(body, cursor), Some(closed.into()), "{body:?}");
        }
    }

    // 閉じるフェンスを打ったときや、行の途中のバッククォートでは、何も補わない。
    #[test]
    fn auto_close_leaves_a_closed_fence_and_plain_backticks() {
        assert_eq!(auto_close("```\na\n```", 9), None);
        assert_eq!(auto_close("a```", 4), None);
        assert_eq!(auto_close("``", 2), None);
    }

    // 開始記号の後ろに本文があるとき、開いたブロックの中で開始記号を打ったとき、情報文字列を打つときは、何も補わない（ADR 050）。
    #[test]
    fn auto_close_leaves_an_opening_with_text_after_it_or_inside_an_open_block() {
        assert_eq!(auto_close("a\n  ```\nb", 7), None);
        assert_eq!(auto_close("```\n>>>", 7), None);
        assert_eq!(auto_close("```r", 4), None);
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

    // 表示の名前を付けたウィキリンクは、`|`の前のページを指す。
    #[test]
    fn link_at_reads_the_page_of_a_wiki_link_with_a_label() {
        assert_eq!(link_at("[[a|label]]", 3), Some(Link::Wiki("a".into())));
        assert_eq!(link_at("[[a|label]]", 11), Some(Link::Wiki("a".into())));
    }

    // タグは、タグに使えない文字の前で終わる。その文字の後ろは、タグの上ではない。
    #[test]
    fn link_at_ends_a_tag_before_a_character_that_a_tag_cannot_hold() {
        assert_eq!(link_at("#rust, ok", 5), Some(Link::Tag("rust".into())));
        assert_eq!(link_at("#rust, ok", 6), None);
        assert_eq!(link_at("#a/b/", 4), Some(Link::Tag("a/b".into())));
        assert_eq!(link_at("#a/b/", 5), None);
    }

    // 語の途中の`#`、エスケープした`#`、コードの中のリンクとタグは、リンクではない。
    #[test]
    fn link_at_ignores_what_the_parser_does_not_show_as_a_link() {
        assert_eq!(link_at("a#b", 2), None);
        assert_eq!(link_at("\\#rust", 3), None);
        assert_eq!(link_at("`[[a]]`", 3), None);
        assert_eq!(link_at("`#rust`", 3), None);
        assert_eq!(link_at("```\n[[a]] #rust\n```", 6), None);
        assert_eq!(link_at("`[[a]]` [[b]]", 10), Some(Link::Wiki("b".into())));
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
