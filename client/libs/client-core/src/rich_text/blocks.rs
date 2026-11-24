//! The parts of a body: a code block, a table and an image of the assets folder stand on their
//! own, between the text before and after them.

use super::paragraphs::{marks, paragraphs};
use super::{links, RichBlock, RichEntry, RichImage, RichParagraph, RichTableRow};
use flatmarkdown_ast::{extract_text, Node, PropValue};

/// The parts of the body whose AST is `ast`, from top to bottom.
///
/// An image of the web is not drawn: it becomes a link to its address. A list or a quote that
/// holds a part of its own kind cannot be one text: it is taken apart, and what is in it is
/// read the same way. A list or a quote that holds none stays in the text.
pub(super) fn blocks(ast: &Node, tags_dir: &str) -> Vec<RichBlock> {
    Reader::new(tags_dir).read(ast.children())
}

struct Reader<'t> {
    tags_dir: &'t str,
    blocks: Vec<RichBlock>,
    /// The nodes since the last part of its own kind, which become one text together.
    others: Vec<Node>,
}

impl<'t> Reader<'t> {
    fn new(tags_dir: &'t str) -> Reader<'t> {
        Reader {
            tags_dir,
            blocks: Vec::new(),
            others: Vec::new(),
        }
    }

    /// The parts of these nodes: those of the document, of an entry of a list, or of a quote.
    fn read(mut self, nodes: &[Node]) -> Vec<RichBlock> {
        for node in nodes {
            match node {
                Node::List {
                    list_type,
                    start,
                    children,
                    ..
                } if holds_own_part(node) => {
                    self.end_text();
                    let entries = marks(*list_type, *start, children)
                        .map(|(mark, entry)| RichEntry {
                            mark,
                            blocks: self.within(entry.children()),
                        })
                        .collect();
                    self.blocks.push(RichBlock::Entries {
                        entries,
                        indent: super::INDENT_STEP,
                    });
                }
                Node::BlockQuote { children } | Node::MultilineBlockQuote { children }
                    if holds_own_part(node) =>
                {
                    self.end_text();
                    let blocks = self.within(children);
                    self.blocks.push(RichBlock::Quote {
                        blocks,
                        indent: super::INDENT_STEP,
                    });
                }
                Node::CodeBlock { literal, .. } => {
                    self.end_text();
                    let lines = literal.trim_end_matches('\n').to_string();
                    self.blocks.push(RichBlock::Code { lines, start: 0 });
                }
                Node::Table { children, .. } => {
                    self.end_text();
                    let rows = children.iter().map(|row| self.row(row)).collect();
                    self.blocks.push(RichBlock::Table { rows });
                }
                Node::Paragraph { children } => self.paragraph(children),
                _ => self.others.push(node.clone()),
            }
        }
        self.end_text();
        self.blocks
    }

    fn within(&self, nodes: &[Node]) -> Vec<RichBlock> {
        Reader::new(self.tags_dir).read(nodes)
    }

    /// A paragraph is cut at each image of the assets folder: the image stands on its own.
    fn paragraph(&mut self, inline: &[Node]) {
        let mut run: Vec<Node> = Vec::new();
        for child in inline {
            if let Some(image) = asset_image(child) {
                self.end_run(&mut run);
                self.end_text();
                self.blocks.push(RichBlock::Image { image });
            } else {
                run.push(child.clone());
            }
        }
        self.end_run(&mut run);
    }

    fn end_run(&mut self, run: &mut Vec<Node>) {
        if !run.is_empty() {
            let children = std::mem::take(run);
            self.others.push(Node::Paragraph { children });
        }
    }

    fn end_text(&mut self) {
        if self.others.is_empty() {
            return;
        }
        // Between two images there may be only a space, which makes no paragraph
        let paragraphs = paragraphs(&self.others, self.tags_dir);
        if !paragraphs.is_empty() {
            self.blocks.push(RichBlock::Text { paragraphs });
        }
        self.others.clear();
    }

    fn row(&self, row: &Node) -> RichTableRow {
        let Node::TableRow { header, children } = row else {
            return RichTableRow {
                header: false,
                cells: Vec::new(),
            };
        };
        // What is in a cell is part of a line: it is one paragraph
        let cells = children
            .iter()
            .map(|cell| {
                let line = Node::Paragraph {
                    children: cell.children().to_vec(),
                };
                let paragraph = paragraphs(std::slice::from_ref(&line), self.tags_dir);
                paragraph.into_iter().next().unwrap_or_default()
            })
            .collect::<Vec<RichParagraph>>();
        RichTableRow {
            header: *header,
            cells,
        }
    }
}

/// True when a part of its own kind is somewhere under the node.
fn holds_own_part(node: &Node) -> bool {
    node.children().iter().any(|child| match child {
        Node::CodeBlock { .. } | Node::Table { .. } => true,
        _ => asset_image(child).is_some() || holds_own_part(child),
    })
}

/// The image that the node is, when its file is in the assets folder. A page writes the
/// address as `../assets/picture.png`, from the URL of the page in the web app.
fn asset_image(node: &Node) -> Option<RichImage> {
    let Node::Embed { url, props, .. } = node else {
        return None;
    };
    let path = links::asset_path(url)?;
    // A side that is not a number above zero is not asked for, as in the web app
    let side = |name: &str| match props.get(name) {
        Some(PropValue::Text(value)) => value.parse::<u32>().ok().filter(|n| *n > 0),
        _ => None,
    };
    Some(RichImage {
        path,
        alt: extract_text(node),
        width: side("w"),
        height: side("h"),
    })
}
