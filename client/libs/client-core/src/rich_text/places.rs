//! Where a touch on the shown text of an item puts the cursor in its Markdown. The AST does not
//! say where in the Markdown its nodes stand, so the letters are lined up instead.

use super::{blocks, RichBlock, RichParagraph};
use crate::asts::parse;
use crate::util::utf16::after_letter;

/// What leaving out a letter of the shown text costs, where a letter of the Markdown costs 1.
/// The Markdown has many letters that are not shown, such as the address of a link; the shown
/// text has few that the Markdown lacks. A link longer than this, with only a letter or two
/// after it, puts the cursor before its address.
const SHOWN_LEFT_OUT: usize = 32;

/// The place in `markdown`, in characters, that stands right after `shown_before`: the letters
/// of the shown text before a touch.
///
/// The shown text is the Markdown without its marks, such as `**` and the address of a link.
/// The two are lined up so that the fewest letters are left out, and the place is the first
/// one where that few are. A letter of the shown text is lined up whenever the Markdown has it
/// within [`SHOWN_LEFT_OUT`] letters, and left out when it is further on: then only the shown
/// text has it.
pub fn markdown_place(markdown: &str, shown_before: &str) -> usize {
    let markdown: Vec<char> = markdown.chars().collect();
    // What leaving out letters costs at least, to line up the letters so far with the first
    // `j` of the Markdown. Only one row is kept: a body may be long
    let mut row: Vec<usize> = (0..=markdown.len()).collect();
    for letter in shown_before.chars() {
        let mut next = Vec::with_capacity(row.len());
        next.push(row[0] + SHOWN_LEFT_OUT);
        for (j, &mark) in markdown.iter().enumerate() {
            let left_out = (next[j] + 1).min(row[j + 1] + SHOWN_LEFT_OUT);
            next.push(if same_letter(mark, letter) {
                left_out.min(row[j])
            } else {
                left_out
            });
        }
        row = next;
    }
    let least = row.iter().min().copied().unwrap_or(0);
    row.iter().position(|&cost| cost == least).unwrap_or(0)
}

/// Whether a letter of the Markdown shows as `letter`. Letters are the same whatever their
/// case, as the title of an alert, `Tip`, is written `[!TIP]`; and the line break after a cell
/// of a table is written as the bar between the cells.
fn same_letter(mark: char, letter: char) -> bool {
    mark == letter || mark.to_lowercase().eq(letter.to_lowercase()) || (mark, letter) == ('|', '\n')
}

/// The place in the Markdown of an item, in characters, of a touch after `letter` letters of
/// its shown text. The page of a tag lies under `tags_dir`, as for its `RichText`.
pub fn touched_place(markdown: &str, tags_dir: &str, letter: usize) -> usize {
    let mut letters = ShownLetters::default();
    count_letters(
        &mut blocks::blocks(&parse(markdown), tags_dir),
        &mut letters,
    );
    let before: String = letters.text.chars().take(letter).collect();
    place_after(markdown, &before)
}

/// The place in the Markdown of an item, in characters, right after the shown letters
/// `before`, at the end of a letter as a person sees it (ADR 043).
pub fn place_after(markdown: &str, before: &str) -> usize {
    after_letter(markdown, markdown_place(markdown, before))
}

/// The shown text of an item as it is written out, with the count of its letters.
#[derive(Default)]
pub(super) struct ShownLetters {
    pub(super) text: String,
    count: u32,
}

impl ShownLetters {
    /// Writes `letters` and a line break after them, and gives how many letters stood before.
    fn write<'a>(&mut self, letters: impl IntoIterator<Item = &'a str>) -> u32 {
        let start = self.count;
        for part in letters {
            self.text.push_str(part);
            self.count = self.count.saturating_add(count(part));
        }
        self.text.push('\n');
        self.count = self.count.saturating_add(1);
        start
    }
}

/// Gives each paragraph and code block of `blocks` the count of the letters of the shown text
/// before it, and writes the shown text into `letters`.
pub(super) fn count_letters(blocks: &mut [RichBlock], letters: &mut ShownLetters) {
    for block in blocks {
        match block {
            RichBlock::Text { paragraphs } => paragraphs
                .iter_mut()
                .for_each(|paragraph| count_paragraph(paragraph, letters)),
            RichBlock::Code { lines, start } => *start = letters.write([lines.as_str()]),
            RichBlock::Table { rows } => rows
                .iter_mut()
                .flat_map(|row| row.cells.iter_mut())
                .for_each(|cell| count_paragraph(cell, letters)),
            RichBlock::Image { .. } => {}
            RichBlock::Entries { entries, .. } => entries
                .iter_mut()
                .for_each(|entry| count_letters(&mut entry.blocks, letters)),
            RichBlock::Quote { blocks, .. } => count_letters(blocks, letters),
        }
    }
}

fn count_paragraph(paragraph: &mut RichParagraph, letters: &mut ShownLetters) {
    paragraph.start = letters.write(paragraph.runs.iter().map(|run| run.text.as_str()));
}

fn count(letters: &str) -> u32 {
    u32::try_from(letters.chars().count()).unwrap_or(u32::MAX)
}
