//! The paragraphs of a text: the nesting of the inline nodes of the AST becomes runs of
//! letters, each with its looks and its link.

use super::{links, Looks, RichLink, RichParagraph, RichRun};
use flatmarkdown_ast::{extract_text, AlertType, ListType, Node};

/// The paragraphs of the text of these nodes of the AST: blocks, as in a document. The page of
/// a tag lies under `tags_dir`.
pub(super) fn paragraphs(nodes: &[Node], tags_dir: &str) -> Vec<RichParagraph> {
    let mut writer = Writer::new(tags_dir);
    writer.blocks(nodes);
    writer.paragraphs
}

/// What stands before an entry of a list: a checkbox for a task, its number in a numbered
/// list, and a bullet otherwise.
pub(super) fn mark(entry: &Node, ordered: bool, number: u32) -> String {
    match entry {
        Node::TaskItem { symbol: None, .. } => "☐".to_string(),
        Node::TaskItem {
            symbol: Some(_), ..
        } => "☑".to_string(),
        _ if ordered => format!("{number}."),
        _ => "•".to_string(),
    }
}

/// The marks of the entries of a list, in their order.
pub(super) fn marks<'a>(
    list_type: ListType,
    start: u32,
    entries: &'a [Node],
) -> impl Iterator<Item = (String, &'a Node)> + 'a {
    let ordered = list_type == ListType::Ordered;
    (start..)
        .zip(entries)
        .map(move |(number, entry)| (mark(entry, ordered, number), entry))
}

/// The text that a reader sees for a footnote. An inline footnote has a name that the parser
/// made (`__inline_1`), so its number is shown instead.
fn footnote_label(name: &str) -> &str {
    name.strip_prefix("__inline_").unwrap_or(name)
}

fn alert_name(alert_type: AlertType) -> &'static str {
    match alert_type {
        AlertType::Note => "Note",
        AlertType::Tip => "Tip",
        AlertType::Important => "Important",
        AlertType::Warning => "Warning",
        AlertType::Caution => "Caution",
    }
}

fn looks(set: impl FnOnce(&mut Looks)) -> Looks {
    let mut looks = Looks::default();
    set(&mut looks);
    looks
}

struct Writer<'t> {
    tags_dir: &'t str,
    paragraphs: Vec<RichParagraph>,
    /// The paragraph that takes the next text, or `None` between two paragraphs.
    open: Option<RichParagraph>,
    depth: u32,
    quoted: u32,
    /// The mark of the entry that has started and has no paragraph yet.
    mark: Option<String>,
    /// What the next paragraph starts with, such as the label of a footnote.
    lead: Option<String>,
}

impl<'t> Writer<'t> {
    fn new(tags_dir: &'t str) -> Writer<'t> {
        Writer {
            tags_dir,
            paragraphs: Vec::new(),
            open: None,
            depth: 0,
            quoted: 0,
            mark: None,
            lead: None,
        }
    }

    fn blocks(&mut self, nodes: &[Node]) {
        for node in nodes {
            self.block(node);
        }
    }

    fn block(&mut self, node: &Node) {
        match node {
            Node::Document { children }
            | Node::Item { children, .. }
            | Node::TaskItem { children, .. }
            | Node::Table { children, .. }
            | Node::Unknown { children, .. } => self.blocks(children),
            Node::Paragraph { children }
            | Node::TableRow { children, .. }
            | Node::TableCell { children } => self.paragraph(0, children),
            Node::Heading { level, children } => self.paragraph(*level, children),
            Node::CodeBlock { literal, .. } => {
                self.begin(0);
                let code = looks(|l| l.code = true);
                self.append(literal.trim_end_matches('\n'), code, None);
                self.finish();
            }
            Node::BlockQuote { children } | Node::MultilineBlockQuote { children } => {
                self.enter_quote();
                self.blocks(children);
                self.leave_quote();
            }
            Node::Alert {
                alert_type,
                title,
                children,
            } => {
                self.enter_quote();
                let title = title.as_deref().unwrap_or(alert_name(*alert_type));
                self.paragraph(0, &[text(title)]);
                self.blocks(children);
                self.leave_quote();
            }
            Node::List {
                list_type,
                start,
                children,
                ..
            } => {
                self.finish();
                self.depth += 1;
                for (mark, entry) in marks(*list_type, *start, children) {
                    self.mark = Some(mark);
                    self.block(entry);
                    self.finish();
                    // An entry without text still shows its mark, and leaves it to no other
                    // paragraph
                    if let Some(mark) = self.mark.take() {
                        self.paragraphs.push(RichParagraph {
                            depth: self.depth,
                            quoted: self.quoted,
                            indent: super::indent(self.depth, self.quoted),
                            mark: Some(mark),
                            ..RichParagraph::default()
                        });
                    }
                }
                self.depth -= 1;
            }
            Node::ThematicBreak => {
                self.finish();
                self.paragraphs.push(RichParagraph {
                    depth: self.depth,
                    quoted: self.quoted,
                    indent: super::indent(self.depth, self.quoted),
                    rule: true,
                    ..RichParagraph::default()
                });
            }
            Node::HtmlBlock { literal, .. } => self.paragraph(0, &[text(literal)]),
            Node::FootnoteDefinition { name, children } => {
                self.finish();
                self.lead = Some(format!("[{}]: ", footnote_label(name)));
                self.blocks(children);
                self.lead = None;
            }
            Node::Subtext { children } => {
                let lowered = Node::Subscript {
                    children: children.clone(),
                };
                self.paragraph(0, &[lowered]);
            }
            // A line outside a block, as in a list without room between its entries
            _ => self.paragraph(0, std::slice::from_ref(node)),
        }
    }

    fn enter_quote(&mut self) {
        self.finish();
        self.quoted += 1;
    }

    fn leave_quote(&mut self) {
        self.finish();
        self.quoted -= 1;
    }

    fn paragraph(&mut self, heading: u8, inline: &[Node]) {
        self.begin(heading);
        for node in inline {
            self.write(node, Looks::default(), None);
        }
        self.finish();
    }

    /// The letters of an inline node, with the looks and the link of the nodes around it.
    fn write(&mut self, node: &Node, looks: Looks, link: Option<&RichLink>) {
        match node {
            Node::Softbreak => self.append(" ", looks, link),
            Node::Linebreak => self.append("\n", looks, link),
            Node::Emph { children } => {
                self.write_all(children, looks.with(self::looks(|l| l.italic = true)), link)
            }
            Node::Strong { children } => {
                self.write_all(children, looks.with(self::looks(|l| l.bold = true)), link)
            }
            Node::Strikethrough { children } => {
                self.write_all(children, looks.with(self::looks(|l| l.struck = true)), link)
            }
            Node::Underline { children } => self.write_all(
                children,
                looks.with(self::looks(|l| l.underlined = true)),
                link,
            ),
            Node::Highlight { children } => {
                self.write_all(children, looks.with(self::looks(|l| l.marked = true)), link)
            }
            Node::Superscript { children } => {
                self.write_all(children, looks.with(self::looks(|l| l.raised = true)), link)
            }
            Node::Subscript { children } => self.write_all(
                children,
                looks.with(self::looks(|l| l.lowered = true)),
                link,
            ),
            Node::SpoileredText { children } => {
                self.write_all(children, looks.with(self::looks(|l| l.hidden = true)), link)
            }
            Node::Code { literal } => {
                self.append(literal, looks.with(self::looks(|l| l.code = true)), link)
            }
            Node::Link { url, children, .. } => {
                let target = links::target(url);
                self.write_all(children, looks, target.as_ref());
            }
            Node::Wikilink { url, children } => {
                let target = links::page(url);
                self.write_all(children, looks, Some(&target));
            }
            Node::Hashtag { value } => {
                let target = links::tag(self.tags_dir, value);
                self.append(&format!("#{value}"), looks, Some(&target));
            }
            Node::Embed { url, .. } => {
                // An image that has no part of its own shows what stands for it. One of the web
                // is a link to its address
                let alt = extract_text(node);
                if url.starts_with("https://") || url.starts_with("http://") {
                    let target = RichLink::Web { url: url.clone() };
                    let shown = if alt.is_empty() { url } else { &alt };
                    self.append(shown, looks, Some(&target));
                } else {
                    self.append(&alt, looks, link);
                }
            }
            Node::FootnoteReference { name, .. } => {
                let label = format!("[{}]", footnote_label(name));
                self.append(&label, looks.with(self::looks(|l| l.raised = true)), link);
            }
            Node::Shortcode { emoji, .. } => self.append(emoji, looks, link),
            Node::Math { literal } => self.append(literal, looks, link),
            Node::HtmlInline { value } => self.append(value, looks, link),
            Node::Unknown { children, .. } => self.write_all(children, looks, link),
            // Text, and a block inside a line: only its letters
            _ => self.append(&extract_text(node), looks, link),
        }
    }

    fn write_all(&mut self, nodes: &[Node], looks: Looks, link: Option<&RichLink>) {
        for node in nodes {
            self.write(node, looks, link);
        }
    }

    /// Starts a paragraph. The first one of an entry of a list takes the mark of the entry.
    fn begin(&mut self, heading: u8) {
        self.finish();
        let mut runs = Vec::new();
        if let Some(lead) = self.lead.take() {
            runs.push(RichRun {
                text: lead,
                ..RichRun::default()
            });
        }
        self.open = Some(RichParagraph {
            heading,
            scale: super::heading_scale(heading),
            depth: self.depth,
            quoted: self.quoted,
            indent: super::indent(self.depth, self.quoted),
            mark: self.mark.take(),
            rule: false,
            runs,
            start: 0,
        });
    }

    fn append(&mut self, text: &str, looks: Looks, link: Option<&RichLink>) {
        if text.is_empty() {
            return;
        }
        // Letters are written only inside a paragraph
        let Some(open) = self.open.as_mut() else {
            return;
        };
        match open.runs.last_mut() {
            Some(last) if last.looks == looks && last.link.as_ref() == link => {
                last.text.push_str(text)
            }
            _ => open.runs.push(RichRun {
                text: text.to_string(),
                looks,
                link: link.cloned(),
            }),
        }
    }

    /// Ends the paragraph that is open. Space at its start and its end is no text: the text
    /// beside an image is cut there with the space between them.
    fn finish(&mut self) {
        let Some(mut done) = self.open.take() else {
            return;
        };
        if let Some(first) = done.runs.first_mut() {
            first.text = first.text.trim_start().to_string();
            if first.text.is_empty() {
                done.runs.remove(0);
            }
        }
        if let Some(last) = done.runs.last_mut() {
            last.text = last.text.trim_end().to_string();
            if last.text.is_empty() {
                done.runs.pop();
            }
        }
        if !done.runs.is_empty() || done.mark.is_some() {
            self.paragraphs.push(done);
        }
    }
}

fn text(value: &str) -> Node {
    Node::Text {
        value: value.to_string(),
    }
}
