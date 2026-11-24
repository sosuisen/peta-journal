//! The body of an item as a native shell shows it (ADR 042): parts in a row from top to bottom,
//! each with its paragraphs and the look of its letters. The core makes it from the AST, so
//! that the shells only draw it with the parts of their OS.
//!
//! The AST is nested; the text of a native shell is flat. Two things are made flat here:
//!
//! - The nesting of inline nodes, such as bold in a link, becomes runs of letters, each with
//!   its looks and its link.
//! - A code block, a table and an image of the assets folder become parts of their own,
//!   between the text before and after them. SwiftUI and Compose cannot put them in a text.

mod blocks;
mod links;
mod paragraphs;
mod places;

#[cfg(test)]
mod tests;

pub use links::asset_path;
pub use places::{markdown_place, place_after, touched_place};

use crate::body_model::BodyModel;
use flatmarkdown_ast::Node;
use serde::Serialize;

/// The body of an item as it is shown: its parts from top to bottom.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct RichText {
    pub blocks: Vec<RichBlock>,
}

impl BodyModel for RichText {
    fn of(ast: Node, tags_dir: &str) -> RichText {
        let mut blocks = blocks::blocks(&ast, tags_dir);
        places::count_letters(&mut blocks, &mut places::ShownLetters::default());
        RichText { blocks }
    }
}

/// One part of a body as it is shown.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum RichBlock {
    /// Text with the look of its letters and its links.
    Text {
        paragraphs: Vec<RichParagraph>,
    },
    /// A block of code: its lines as they were written.
    Code {
        lines: String,
        /// How many letters of the shown text stand before the code, as for a paragraph.
        start: u32,
    },
    Table {
        rows: Vec<RichTableRow>,
    },
    /// An image of the assets folder.
    Image {
        image: RichImage,
    },
    /// A list that holds a part of its own kind, such as a code block: its entries apart.
    Entries {
        entries: Vec<RichEntry>,
    },
    /// A quote that holds a part of its own kind.
    Quote {
        blocks: Vec<RichBlock>,
    },
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RichTableRow {
    /// True for the row of the titles of the columns.
    pub header: bool,
    /// What is in each cell: one paragraph each.
    pub cells: Vec<RichParagraph>,
}

/// An image of a page.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RichImage {
    /// Where its file is under the Markdown folder, such as `assets/picture.png`.
    pub path: String,
    /// What stands for the image where it cannot be seen.
    pub alt: String,
    /// The width that the page asks for, in points. `None` when it asks for none.
    pub width: Option<u32>,
    /// The height that the page asks for, the same way.
    pub height: Option<u32>,
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RichEntry {
    /// What stands before the entry of a list: a bullet, its number or a checkbox.
    pub mark: String,
    pub blocks: Vec<RichBlock>,
}

/// One paragraph of a text.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RichParagraph {
    /// The level of a heading, from 1 to 6, and 0 for any other paragraph.
    pub heading: u8,
    /// How much larger than the usual text the letters are drawn: the scale of the heading,
    /// and 1 for any other paragraph. Every shell takes it from here (#795).
    pub scale: f32,
    /// How many lists the paragraph is inside.
    pub depth: u32,
    /// How many quotes the paragraph is inside.
    pub quoted: u32,
    /// What stands before the first paragraph of an entry of a list: a bullet, its number or
    /// a checkbox.
    pub mark: Option<String>,
    /// True for a line across the page, which has no runs.
    pub rule: bool,
    pub runs: Vec<RichRun>,
    /// How many letters of the shown text of the item stand before the paragraph. The shown
    /// text is the letters of the runs of each paragraph and of the lines of each code block,
    /// from top to bottom, each with a line break after it. A shell adds the letters before a
    /// touch in the paragraph to this, and tells the core (`EditPlace::ShownLetter`).
    pub start: u32,
}

impl Default for RichParagraph {
    fn default() -> RichParagraph {
        RichParagraph {
            heading: 0,
            scale: 1.0,
            depth: 0,
            quoted: 0,
            mark: None,
            rule: false,
            runs: Vec::new(),
            start: 0,
        }
    }
}

/// How much larger than the usual text a heading of each level is, from the first to the
/// fifth. A sixth level is as large as the fifth: smaller than the usual text, it would read
/// as less than a heading.
const HEADING_SCALES: [f32; 5] = [1.6, 1.4, 1.2, 1.1, 1.0];

/// How much larger than the usual text the letters of a heading of `level` are. 1 for a level
/// that is no heading.
pub fn heading_scale(level: u8) -> f32 {
    match level {
        0 => 1.0,
        level => HEADING_SCALES[usize::from(level.min(5)) - 1],
    }
}

/// Letters with one look and one link.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct RichRun {
    pub text: String,
    pub looks: Looks,
    /// Where a click or a touch leads. `None` for letters that lead nowhere.
    pub link: Option<RichLink>,
}

/// How the letters of a run look.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Looks {
    pub bold: bool,
    pub italic: bool,
    pub struck: bool,
    pub underlined: bool,
    /// Highlighted, as with a marker.
    pub marked: bool,
    pub code: bool,
    /// Small and above the line.
    pub raised: bool,
    /// Small and below the line.
    pub lowered: bool,
    /// A spoiler: not to be read at a glance.
    pub hidden: bool,
}

impl Looks {
    /// These looks and those of `more` together.
    fn with(self, more: Looks) -> Looks {
        Looks {
            bold: self.bold || more.bold,
            italic: self.italic || more.italic,
            struck: self.struck || more.struck,
            underlined: self.underlined || more.underlined,
            marked: self.marked || more.marked,
            code: self.code || more.code,
            raised: self.raised || more.raised,
            lowered: self.lowered || more.lowered,
            hidden: self.hidden || more.hidden,
        }
    }
}

/// Where a link of an item leads.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum RichLink {
    /// A page of the app: its URL, such as `/pages/notes#item`, as it is written and not
    /// encoded. A wikilink, a hashtag and a link to a `.md` file lead to a page.
    Page { url: String },
    /// An address that the browser opens.
    Web { url: String },
    /// A file of the assets folder: its path under the Markdown folder, such as
    /// `assets/report.pdf`.
    File { path: String },
}
