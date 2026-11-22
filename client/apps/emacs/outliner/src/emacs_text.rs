//! The body of an item as Emacs shows it (ADR 077, ADR 078): one text, and the parts of it
//! that look different. It is made from the `RichText` of the native shells, so the shown
//! letters are counted as there (ADR 043, ADR 044).

use client_core::rich_text::{RichBlock, RichParagraph, RichText};
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
}

/// A part of the text with its looks, its link or its image.
#[derive(Facet, Debug, Clone, PartialEq, Serialize)]
pub struct EmacsSpan {
    /// Places in `text`, in characters (code points, ADR 043).
    pub from: u32,
    pub to: u32,
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
            self.text.push_str(&run.text);
        }
        self.text.push('\n');
    }
}
