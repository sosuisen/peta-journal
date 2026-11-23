//! The body of an item as Emacs shows it (ADR 077, ADR 078): one text, and the parts of it
//! that look different. It is made from the `RichText` of the native shells, so the shown
//! letters are counted as there (ADR 043, ADR 044).

use client_core::rich_text::{
    Looks, RichBlock, RichImage, RichLink, RichParagraph, RichRun, RichTableRow, RichText,
};
use client_core::BodyModel;
use facet::Facet;
use flatmarkdown_ast::Node;
use serde::{Serialize, Serializer};
use std::sync::Arc;
use unicode_width::UnicodeWidthStr;

/// The body of an item as Emacs shows it: one text, and what each part of it looks like.
#[derive(Facet, Debug, Clone, PartialEq, Default, Serialize)]
pub struct EmacsText {
    /// The shown text. A line break after each paragraph and each line of code.
    pub text: String,
    /// The parts of the text that are not plain shown letters, in the order of `from`.
    pub spans: Vec<EmacsSpan>,
}

/// A part of the text with its looks, its link or its image.
#[derive(Facet, Debug, Clone, PartialEq, Serialize)]
pub struct EmacsSpan {
    /// Places in `text`, in characters (code points, ADR 043).
    pub from: u32,
    pub to: u32,
    pub looks: Vec<EmacsLook>,
    pub link: Option<RichLink>,
    /// An image of the assets folder, shown in place of the text of the span.
    pub image: Option<RichImage>,
    /// False for what Rust added, such as a list mark or the spaces of a table: they are
    /// not letters of the shown text (ADR 044).
    pub letters: bool,
}

impl EmacsSpan {
    /// True when `other` tells the same of its letters, wherever it is.
    fn same_as(&self, other: &EmacsSpan) -> bool {
        self.looks == other.looks
            && self.link == other.link
            && self.image == other.image
            && self.letters == other.letters
    }
}

/// How a part of the text looks.
#[derive(Facet, Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[facet(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[repr(C)]
pub enum EmacsLook {
    Bold,
    Italic,
    Struck,
    Underlined,
    /// Highlighted, as with a marker.
    Marked,
    Code,
    /// Small and above the line.
    Raised,
    /// Small and below the line.
    Lowered,
    /// A spoiler: not to be read at a glance.
    Hidden,
    /// The line of a heading, by its level.
    Heading1,
    Heading2,
    Heading3,
    Heading4,
    Heading5,
    Heading6,
    /// What stands before an entry of a list: a bullet, its number or a checkbox.
    Mark,
    /// The mark before a quoted paragraph, one for each quote it is inside.
    Quote,
    /// A line across the page.
    Rule,
    /// The lines of a code block.
    CodeBlock,
    /// A cell of the row of the titles of the columns of a table.
    TableHeader,
}

/// The look of the line of a heading of `level`, from 1 to 6. `None` for any other paragraph.
fn heading_look(level: u8) -> Option<EmacsLook> {
    match level {
        1 => Some(EmacsLook::Heading1),
        2 => Some(EmacsLook::Heading2),
        3 => Some(EmacsLook::Heading3),
        4 => Some(EmacsLook::Heading4),
        5 => Some(EmacsLook::Heading5),
        6 => Some(EmacsLook::Heading6),
        _ => None,
    }
}

/// How wide one step of the indent of a list is.
const INDENT: &str = "  ";

/// The mark of one quote.
const QUOTE: &str = "▎ ";

/// A rule is a line of this letter, this many letters wide.
const RULE: &str = "─";
const RULE_WIDTH: usize = 20;

/// What shows between the cells of a table, in place of the line break after a cell.
const BAR: &str = "│";

impl BodyModel for EmacsText {
    fn of(ast: Node, tags_dir: &str) -> EmacsText {
        let rich = RichText::of(ast, tags_dir);
        let mut writer = Writer::default();
        writer.blocks(&rich.blocks);
        writer.made
    }
}

/// The body model of the rows of the Emacs shell: an [`EmacsText`] that the core, the view and
/// the rows the shell sent all share (#960). The view is made again after every key, and the
/// core copies each body model into it and keeps a copy for the next view. A copy of the
/// shared text is a count, where a copy of the text itself was the text and all its parts.
#[derive(Debug, Clone, Default)]
pub struct SharedText(pub Arc<EmacsText>);

/// Most rows have the same text as the time before, shared: those are equal without comparing
/// the letters.
impl PartialEq for SharedText {
    fn eq(&self, other: &SharedText) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0 == other.0
    }
}

impl Serialize for SharedText {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl BodyModel for SharedText {
    fn of(ast: Node, tags_dir: &str) -> SharedText {
        SharedText(Arc::new(EmacsText::of(ast, tags_dir)))
    }
}

/// Makes an `EmacsText` from the top of the body to the bottom.
#[derive(Default)]
struct Writer {
    made: EmacsText,
    /// How many characters the text has.
    length: u32,
    /// What stands at the head of each line, for the entries and the quotes that the
    /// blocks being written are inside, from the outside in.
    prefixes: Vec<Prefix>,
}

/// What stands at the head of each line of a part that is inside another.
struct Prefix {
    /// For the first line only, with its looks, such as the mark of an entry. The first
    /// line takes it.
    first: Option<(String, Vec<EmacsLook>)>,
    /// For each line after it, with its looks.
    rest: (&'static str, Vec<EmacsLook>),
}

impl Writer {
    fn blocks(&mut self, blocks: &[RichBlock]) {
        for block in blocks {
            match block {
                RichBlock::Text { paragraphs } => paragraphs
                    .iter()
                    .for_each(|paragraph| self.paragraph(paragraph)),
                RichBlock::Code { lines, .. } => self.code(lines),
                RichBlock::Table { rows } => self.table(rows),
                RichBlock::Entries { entries } => {
                    for entry in entries {
                        self.within(
                            Prefix {
                                first: Some((format!("{} ", entry.mark), vec![EmacsLook::Mark])),
                                rest: (INDENT, Vec::new()),
                            },
                            &entry.blocks,
                        );
                    }
                }
                RichBlock::Quote { blocks } => self.within(
                    Prefix {
                        first: None,
                        rest: (QUOTE, vec![EmacsLook::Quote]),
                    },
                    blocks,
                ),
                RichBlock::Image { image } => {
                    self.line_start();
                    // The line break too: an image has no shown letters (ADR 044). A line
                    // break alone would leave no place to show the image on.
                    let alt = if image.alt.is_empty() {
                        "image"
                    } else {
                        image.alt.as_str()
                    };
                    self.write(
                        &format!("{alt}\n"),
                        Part {
                            image: Some(image.clone()),
                            added: true,
                            ..Part::default()
                        },
                    );
                }
            }
        }
    }

    /// Writes `blocks` with `prefix` at the head of each of their lines.
    fn within(&mut self, prefix: Prefix, blocks: &[RichBlock]) {
        self.prefixes.push(prefix);
        self.blocks(blocks);
        self.prefixes.pop();
    }

    /// The lines of a code block, each behind the prefixes of its line.
    fn code(&mut self, lines: &str) {
        let mut lines = lines.split('\n').peekable();
        while let Some(line) = lines.next() {
            self.line_start();
            self.write(line, Part::looking(vec![EmacsLook::CodeBlock]));
            // The line breaks between the lines are lines of the code too
            if lines.peek().is_some() {
                self.write("\n", Part::looking(vec![EmacsLook::CodeBlock]));
            } else {
                self.write("\n", Part::default());
            }
        }
    }

    /// Writes the prefixes at the head of a new line.
    fn line_start(&mut self) {
        let heads: Vec<(String, Vec<EmacsLook>)> = self
            .prefixes
            .iter_mut()
            .map(|prefix| {
                prefix
                    .first
                    .take()
                    .unwrap_or_else(|| (prefix.rest.0.to_string(), prefix.rest.1.clone()))
            })
            .collect();
        for (head, looks) in heads {
            self.write(&head, Part::added(looks));
        }
    }

    /// The letters of `runs` with their looks, and the look `more` on each.
    fn runs(&mut self, runs: &[RichRun], more: Option<EmacsLook>) {
        for run in runs {
            let mut looks = looks_of(run.looks);
            looks.extend(more);
            self.write(
                &run.text,
                Part {
                    looks,
                    link: run.link.clone(),
                    ..Part::default()
                },
            );
        }
    }

    /// The rows of a table, one line each. The cells line up in columns, as wide as Emacs
    /// shows them, with a wide letter counted as two. The line break after each cell but
    /// the last shows as a bar.
    fn table(&mut self, rows: &[RichTableRow]) {
        let mut widths: Vec<usize> = Vec::new();
        for row in rows {
            for (column, cell) in row.cells.iter().enumerate() {
                let width = cell_width(cell);
                match widths.get_mut(column) {
                    Some(wide) => *wide = (*wide).max(width),
                    None => widths.push(width),
                }
            }
        }
        for row in rows {
            self.line_start();
            let more = row.header.then_some(EmacsLook::TableHeader);
            let mut cells = row.cells.iter().zip(&widths).peekable();
            while let Some((cell, width)) = cells.next() {
                self.runs(&cell.runs, more);
                if cells.peek().is_some() {
                    let pad = " ".repeat(width.saturating_sub(cell_width(cell)) + 1);
                    self.write(&pad, Part::added(Vec::new()));
                    self.write(BAR, Part::default());
                    self.write(" ", Part::added(Vec::new()));
                } else {
                    self.write("\n", Part::default());
                }
            }
        }
    }

    fn paragraph(&mut self, paragraph: &RichParagraph) {
        self.line_start();
        self.before_paragraph(paragraph);
        let (start, first_span) = (self.length, self.made.spans.len());
        self.runs(&paragraph.runs, None);
        if let Some(look) = heading_look(paragraph.heading) {
            // Before the spans of its runs, so that the spans stay in the order of `from`
            self.made.spans.insert(
                first_span,
                EmacsSpan {
                    from: start,
                    to: self.length,
                    looks: vec![look],
                    link: None,
                    image: None,
                    letters: true,
                },
            );
        }
        self.write("\n", Part::default());
    }

    /// The marks of the quotes the paragraph is inside, the indent of its lists, and the
    /// mark of its entry.
    fn before_paragraph(&mut self, paragraph: &RichParagraph) {
        if paragraph.quoted > 0 {
            self.write(
                &QUOTE.repeat(paragraph.quoted as usize),
                Part::added(vec![EmacsLook::Quote]),
            );
        }
        let steps = paragraph.depth.saturating_sub(1) as usize;
        if steps > 0 {
            self.write(&INDENT.repeat(steps), Part::added(Vec::new()));
        }
        if let Some(mark) = &paragraph.mark {
            self.write(&format!("{mark} "), Part::added(vec![EmacsLook::Mark]));
        }
        if paragraph.rule {
            self.write(&RULE.repeat(RULE_WIDTH), Part::added(vec![EmacsLook::Rule]));
        }
    }

    /// Writes `letters` at the end of the text, with a span when they are not plain. A span
    /// right after one that is the same grows it instead.
    fn write(&mut self, letters: &str, part: Part) {
        let from = self.length;
        self.made.text.push_str(letters);
        self.length = self
            .length
            .saturating_add(u32::try_from(letters.chars().count()).unwrap_or(u32::MAX));
        if part.is_plain() {
            return;
        }
        let span = EmacsSpan {
            from,
            to: self.length,
            looks: part.looks,
            link: part.link,
            image: part.image,
            letters: !part.added,
        };
        match self.made.spans.last_mut() {
            Some(last) if last.to == from && last.same_as(&span) => last.to = span.to,
            _ => self.made.spans.push(span),
        }
    }
}

/// What a span tells of its letters, before its places are known.
#[derive(Default)]
struct Part {
    looks: Vec<EmacsLook>,
    link: Option<RichLink>,
    image: Option<RichImage>,
    /// True for what Rust added, which is not shown letters.
    added: bool,
}

impl Part {
    /// Shown letters with `looks`.
    fn looking(looks: Vec<EmacsLook>) -> Part {
        Part {
            looks,
            ..Part::default()
        }
    }

    /// What Rust adds, with `looks`.
    fn added(looks: Vec<EmacsLook>) -> Part {
        Part {
            looks,
            added: true,
            ..Part::default()
        }
    }

    /// True for plain shown letters, which need no span.
    fn is_plain(&self) -> bool {
        self.looks.is_empty() && self.link.is_none() && self.image.is_none() && !self.added
    }
}

/// How wide Emacs shows the letters of a cell, with a wide letter counted as two.
fn cell_width(cell: &RichParagraph) -> usize {
    cell.runs.iter().map(|run| run.text.width()).sum()
}

/// The looks of a run, in the order of `EmacsLook`.
fn looks_of(looks: Looks) -> Vec<EmacsLook> {
    [
        (looks.bold, EmacsLook::Bold),
        (looks.italic, EmacsLook::Italic),
        (looks.struck, EmacsLook::Struck),
        (looks.underlined, EmacsLook::Underlined),
        (looks.marked, EmacsLook::Marked),
        (looks.code, EmacsLook::Code),
        (looks.raised, EmacsLook::Raised),
        (looks.lowered, EmacsLook::Lowered),
        (looks.hidden, EmacsLook::Hidden),
    ]
    .into_iter()
    .filter_map(|(on, look)| on.then_some(look))
    .collect()
}
