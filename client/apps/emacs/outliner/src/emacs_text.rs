//! The body of an item as Emacs shows it (ADR 077, ADR 078): one text, and the parts of it
//! that look different. It is made from the `RichText` of the native shells, so the shown
//! letters are counted as there (ADR 043, ADR 044).

use client_core::rich_text::{Looks, RichBlock, RichParagraph, RichText};
use client_core::BodyModel;
use facet::Facet;
use flatmarkdown_ast::Node;
use serde::Serialize;

/// The body of an item as Emacs shows it: one text, and what each part of it looks like.
#[derive(Facet, Debug, Clone, PartialEq, Default, Serialize)]
pub struct EmacsText {
    /// The shown text. A line break after each paragraph and each line of code.
    pub text: String,
    /// The parts of the text that are not plain shown letters, in order.
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
}

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
        for run in &paragraph.runs {
            self.write(&run.text, looks_of(run.looks));
        }
        self.write("\n", Vec::new());
    }

    /// Writes `letters` at the end of the text, with a span when they look different.
    fn write(&mut self, letters: &str, looks: Vec<EmacsLook>) {
        let from = self.length;
        self.text.push_str(letters);
        self.length = self
            .length
            .saturating_add(u32::try_from(letters.chars().count()).unwrap_or(u32::MAX));
        if !looks.is_empty() {
            self.spans.push(EmacsSpan {
                from,
                to: self.length,
                looks,
            });
        }
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
