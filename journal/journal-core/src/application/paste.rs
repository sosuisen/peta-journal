//! What a pasted text becomes in an item: the pieces it is cut into, joined with the body around
//! the cursor. Nothing here reads or writes the database.

use super::flat_markdown_splitter::FlatMarkdownSplitter;
use super::tree_markdown;

/// How a pasted text is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteMode {
    /// Tree markdown is converted, and the text is split into items.
    Auto,
    /// The text is Flat Markdown, as this app copies it: it is split into items as it is.
    Flat,
    /// The text goes into the item as it is.
    Raw,
}

/// A part of a pasted text that goes into one item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    pub body: String,
    /// How many levels below the item pasted into this piece is; never above it.
    pub level: i32,
}

/// What to write for a paste. The first piece is the new body of the item pasted into, and each
/// piece after it is a new item below the one before.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PastePlan {
    pub pieces: Vec<Piece>,
    /// The cursor right after the pasted text, as a character index in the last piece.
    pub cursor: usize,
}

/// Plans the paste of `text` at the character `cursor` of `body`. The first piece joins the
/// text before the cursor, and the last piece takes the text after it.
pub fn plan(
    splitter: &FlatMarkdownSplitter,
    body: &str,
    cursor: usize,
    text: &str,
    mode: PasteMode,
) -> anyhow::Result<PastePlan> {
    let (before, after) = split_at_char(body, cursor);
    let whole = || {
        vec![Piece {
            body: text.to_string(),
            level: 0,
        }]
    };
    let mut pieces = match mode {
        PasteMode::Raw => whole(),
        PasteMode::Auto if text.trim_start().starts_with("- ") => split_into_pieces(
            splitter,
            &tree_markdown::tree_markdown_to_flat_markdown(text),
        )?,
        // Plain text pasted into a block is in the block: its blank lines do not end an item
        PasteMode::Auto if flatmarkdown::has_unclosed_block(before) => whole(),
        PasteMode::Auto | PasteMode::Flat => split_into_pieces(splitter, text)?,
    };
    let last = pieces.len() - 1;
    let pasted_chars = pieces[last].body.chars().count();
    let cursor = if last == 0 {
        before.chars().count() + pasted_chars
    } else {
        pasted_chars
    };
    pieces[0].body.insert_str(0, before);
    pieces[last].body.push_str(after);
    Ok(PastePlan { pieces, cursor })
}

/// Splits a text at a character index. An index past the end splits at the end.
pub(crate) fn split_at_char(text: &str, char_index: usize) -> (&str, &str) {
    let byte_index = text
        .char_indices()
        .nth(char_index)
        .map_or(text.len(), |(i, _)| i);
    text.split_at(byte_index)
}

/// Cuts Flat Markdown into items at the block boundaries of a markdown file. The levels count
/// from the first piece.
fn split_into_pieces(splitter: &FlatMarkdownSplitter, flat: &str) -> anyhow::Result<Vec<Piece>> {
    let items = splitter.split(flat)?.items;
    let Some(top_level) = items.first().map(|item| item.level) else {
        // Nothing to paste: the body is still written, as it may hold text not saved yet
        return Ok(vec![Piece {
            body: String::new(),
            level: 0,
        }]);
    };
    Ok(items
        .into_iter()
        .map(|item| Piece {
            level: (item.level - top_level).max(0),
            body: item.body,
        })
        .collect())
}
