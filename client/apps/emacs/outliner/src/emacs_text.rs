//! The body of an item as Emacs shows it (ADR 077, ADR 078): one text, and the parts of it
//! that look different. It is made from the `RichText` of the native shells, so the shown
//! letters are counted as there (ADR 043, ADR 044).

use client_core::rich_text::{Looks, RichBlock, RichLink, RichParagraph, RichText};
use client_core::BodyModel;
use facet::Facet;
use flatmarkdown_ast::Node;
use serde::Serialize;

/// The body of an item as Emacs shows it: one text, and what each part of it looks like.
#[derive(Facet, Debug, Clone, PartialEq, Default, Serialize)]
pub struct EmacsText {
    /// The shown text. A line break after each paragraph and each line of code.
    pub text: String,
    /// The parts of the text that are not plain shown letters, in the order of `from`.
    pub spans: Vec<EmacsSpan>,
    /// How many characters `text` has.
    #[facet(skip)]
    #[serde(skip)]
    length: u32,
}

/// A part of the text with its looks, its link or its image.
#[derive(Facet, Debug, Clone, PartialEq, Serialize)]
pub struct EmacsSpan {
    /// Places in `text`, in characters (code points, ADR 043).
    pub from: u32,
    pub to: u32,
    pub looks: Vec<EmacsLook>,
    pub link: Option<RichLink>,
    /// False for what Rust added, such as a list mark or the spaces of a table: they are
    /// not letters of the shown text (ADR 044).
    pub letters: bool,
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

impl BodyModel for EmacsText {
    fn of(ast: Node, tags_dir: &str) -> EmacsText {
        let rich = RichText::of(ast, tags_dir);
        let mut text = EmacsText::default();
        for block in &rich.blocks {
            if let RichBlock::Text { paragraphs } = block {
                paragraphs
                    .iter()
                    .for_each(|paragraph| text.paragraph(paragraph));
            }
        }
        text
    }
}

impl EmacsText {
    fn paragraph(&mut self, paragraph: &RichParagraph) {
        self.before_paragraph(paragraph);
        let (start, first_span) = (self.length, self.spans.len());
        for run in &paragraph.runs {
            self.write(
                &run.text,
                Part {
                    looks: looks_of(run.looks),
                    link: run.link.clone(),
                    ..Part::default()
                },
            );
        }
        if let Some(look) = heading_look(paragraph.heading) {
            // Before the spans of its runs, so that the spans stay in the order of `from`
            self.spans.insert(
                first_span,
                EmacsSpan {
                    from: start,
                    to: self.length,
                    looks: vec![look],
                    link: None,
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

    /// Writes `letters` at the end of the text, with a span when they are not plain.
    fn write(&mut self, letters: &str, part: Part) {
        let from = self.length;
        self.text.push_str(letters);
        self.length = self
            .length
            .saturating_add(u32::try_from(letters.chars().count()).unwrap_or(u32::MAX));
        if !part.is_plain() {
            self.spans.push(EmacsSpan {
                from,
                to: self.length,
                looks: part.looks,
                link: part.link,
                letters: !part.added,
            });
        }
    }
}

/// What a span tells of its letters, before its places are known.
#[derive(Default)]
struct Part {
    looks: Vec<EmacsLook>,
    link: Option<RichLink>,
    /// True for what Rust added, which is not shown letters.
    added: bool,
}

impl Part {
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
        self.looks.is_empty() && self.link.is_none() && !self.added
    }
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
