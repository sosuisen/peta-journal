import Foundation
import PetaCore
import PetaTypes

/// How the letters of a piece look.
public struct Looks: OptionSet, Hashable {
    public let rawValue: Int

    public init(rawValue: Int) {
        self.rawValue = rawValue
    }

    public static let bold = Looks(rawValue: 1 << 0)
    public static let italic = Looks(rawValue: 1 << 1)
    public static let struck = Looks(rawValue: 1 << 2)
    public static let underlined = Looks(rawValue: 1 << 3)
    /// Highlighted, as with a marker.
    public static let marked = Looks(rawValue: 1 << 4)
    public static let code = Looks(rawValue: 1 << 5)
    /// Small and above the line.
    public static let raised = Looks(rawValue: 1 << 6)
    /// Small and below the line.
    public static let lowered = Looks(rawValue: 1 << 7)
    /// A spoiler: not to be read at a glance.
    public static let hidden = Looks(rawValue: 1 << 8)
}

/// Letters with one look. `link` is where a touch leads, such as `/pages/notes` or a web address.
public struct ShownPiece: Equatable {
    public var text: String
    public var looks: Looks
    public var link: String?

    public init(text: String, looks: Looks = [], link: String? = nil) {
        self.text = text
        self.looks = looks
        self.link = link
    }
}

/// One paragraph of the text of an item.
public struct ShownParagraph: Equatable {
    /// The level of a heading, from 1 to 6, and 0 for any other paragraph.
    public var heading: Int
    /// How many lists the paragraph is inside.
    public var depth: Int
    /// How many quotes the paragraph is inside.
    public var quoted: Int
    /// What stands before the first paragraph of an entry of a list: a bullet, its number or
    /// a checkbox.
    public var mark: String?
    /// True for a line across the page, which has no pieces.
    public var rule: Bool
    public var pieces: [ShownPiece]

    public init(
        heading: Int = 0, depth: Int = 0, quoted: Int = 0, mark: String? = nil, rule: Bool = false,
        pieces: [ShownPiece] = []
    ) {
        self.heading = heading
        self.depth = depth
        self.quoted = quoted
        self.mark = mark
        self.rule = rule
        self.pieces = pieces
    }
}

/// The paragraphs of the text of these nodes of the AST: blocks, as in a document.
public func shownParagraphs(of nodes: [Node], tagsDir: String) -> [ShownParagraph] {
    var writer = ParagraphWriter(tagsDir: tagsDir)
    writer.blocks(nodes)
    return writer.paragraphs
}

/// The children of a node, and none for a leaf.
func children(of node: Node) -> [Node] {
    switch node {
    case .document(let children), .paragraph(let children), .heading(_, let children),
        .blockQuote(let children), .multilineBlockQuote(let children), .list(_, _, _, _, let children),
        .item(_, _, let children), .taskItem(_, let children), .table(_, _, _, let children),
        .tableRow(_, let children), .tableCell(let children), .footnoteDefinition(_, let children),
        .alert(_, _, let children), .subtext(let children), .emph(let children), .strong(let children),
        .strikethrough(let children), .underline(let children), .highlight(let children),
        .superscript(let children), .subscriptText(let children), .spoileredText(let children),
        .link(_, _, let children), .embed(_, _, _, let children), .wikilink(_, let children),
        .unknown(_, let children):
        return children
    case .codeBlock, .thematicBreak, .htmlBlock, .text, .softbreak, .linebreak, .code, .footnoteReference,
        .shortcode, .math, .htmlInline, .hashtag:
        return []
    }
}

/// The text under the node, without its decoration.
func plainText(_ node: Node) -> String {
    switch node {
    case .text(let value): return value
    default: return children(of: node).map(plainText).joined()
    }
}

/// The text that a reader sees for a footnote. An inline footnote has a name that the parser
/// made (`__inline_1`), so its number is shown instead.
private func footnoteLabel(_ name: String) -> String {
    name.hasPrefix("__inline_") ? String(name.dropFirst("__inline_".count)) : name
}

private struct ParagraphWriter {
    let tagsDir: String
    private(set) var paragraphs: [ShownParagraph] = []

    /// The paragraph that takes the next text, or nil between two paragraphs.
    private var open: ShownParagraph?
    private var depth = 0
    private var quoted = 0
    /// The mark of the entry that has started and has no paragraph yet.
    private var mark: String?
    /// What the next paragraph starts with, such as the label of a footnote.
    private var lead: String?

    init(tagsDir: String) {
        self.tagsDir = tagsDir
    }

    mutating func blocks(_ nodes: [Node]) {
        for node in nodes { block(node) }
    }

    private mutating func block(_ node: Node) {
        switch node {
        case .document(let children), .item(_, _, let children), .taskItem(_, let children),
            .table(_, _, _, let children), .unknown(_, let children):
            blocks(children)
        case .paragraph(let inline), .tableRow(_, let inline), .tableCell(let inline):
            paragraph(heading: 0, inline)
        case .heading(let level, let inline):
            paragraph(heading: Int(level), inline)
        case .codeBlock(_, _, let literal):
            begin(heading: 0)
            var lines = Substring(literal)
            while lines.hasSuffix("\n") { lines = lines.dropLast() }
            append(String(lines), looks: .code, link: nil)
            finish()
        case .blockQuote(let children), .multilineBlockQuote(let children):
            enterQuote()
            blocks(children)
            leaveQuote()
        case .alert(let type, let title, let children):
            enterQuote()
            paragraph(heading: 0, [.text(value: title ?? name(of: type))])
            blocks(children)
            leaveQuote()
        case .list(let listType, let start, _, _, let entries):
            finish()
            depth += 1
            for (index, entry) in entries.enumerated() {
                mark = PetaBorder.mark(of: entry, ordered: listType == .ordered, number: Int(start) + index)
                block(entry)
                // An entry without text of its own leaves its mark to no other paragraph
                finish()
                mark = nil
            }
            depth -= 1
        case .thematicBreak:
            finish()
            paragraphs.append(ShownParagraph(depth: depth, quoted: quoted, rule: true))
        case .htmlBlock(_, let literal):
            paragraph(heading: 0, [.text(value: literal)])
        case .footnoteDefinition(let name, let children):
            finish()
            lead = "[\(footnoteLabel(name))]: "
            blocks(children)
            lead = nil
        case .subtext(let inline):
            paragraph(heading: 0, [.subscriptText(children: inline)])
        case .text, .softbreak, .linebreak, .emph, .strong, .strikethrough, .underline, .highlight,
            .superscript, .subscriptText, .spoileredText, .code, .link, .embed, .footnoteReference,
            .shortcode, .math, .htmlInline, .wikilink, .hashtag:
            // A line outside a block, as in a list without room between its entries
            paragraph(heading: 0, [node])
        }
    }

    private mutating func enterQuote() {
        finish()
        quoted += 1
    }

    private mutating func leaveQuote() {
        finish()
        quoted -= 1
    }

    private func name(of type: AlertType) -> String {
        switch type {
        case .note: return "Note"
        case .tip: return "Tip"
        case .important: return "Important"
        case .warning: return "Warning"
        case .caution: return "Caution"
        }
    }

    private mutating func paragraph(heading: Int, _ inline: [Node]) {
        begin(heading: heading)
        for node in inline { write(node, looks: [], link: nil) }
        finish()
    }

    /// The letters of an inline node, with the looks and the link of the nodes around it.
    private mutating func write(_ node: Node, looks: Looks, link: String?) {
        func inside(_ children: [Node], _ more: Looks = [], link: String? = link) {
            for child in children { write(child, looks: looks.union(more), link: link) }
        }
        switch node {
        case .text(let value): append(value, looks: looks, link: link)
        case .softbreak: append(" ", looks: looks, link: link)
        case .linebreak: append("\n", looks: looks, link: link)
        case .emph(let children): inside(children, .italic)
        case .strong(let children): inside(children, .bold)
        case .strikethrough(let children): inside(children, .struck)
        case .underline(let children): inside(children, .underlined)
        case .highlight(let children): inside(children, .marked)
        case .superscript(let children): inside(children, .raised)
        case .subscriptText(let children): inside(children, .lowered)
        case .spoileredText(let children): inside(children, .hidden)
        case .code(let literal): append(literal, looks: looks.union(.code), link: link)
        case .link(let url, _, let children): inside(children, link: url)
        case .wikilink(let url, let children): inside(children, link: pageUrl(page: url))
        case .hashtag(let value): append("#" + value, looks: looks, link: tagUrl(tagsDir: tagsDir, tag: value))
        case .embed(let url, _, _, _):
            // An image that has no part of its own shows what stands for it. One of the web is
            // a link to its address
            let alt = plainText(node)
            if url.hasPrefix("https://") || url.hasPrefix("http://") {
                append(alt.isEmpty ? url : alt, looks: looks, link: url)
            } else {
                append(alt, looks: looks, link: link)
            }
        case .footnoteReference(let name, _, _):
            append("[\(footnoteLabel(name))]", looks: looks.union(.raised), link: link)
        case .shortcode(_, let emoji): append(emoji, looks: looks, link: link)
        case .math(let literal), .htmlInline(let literal): append(literal, looks: looks, link: link)
        case .unknown(_, let children): inside(children)
        case .document, .paragraph, .heading, .codeBlock, .blockQuote, .multilineBlockQuote, .list, .item,
            .taskItem, .table, .tableRow, .tableCell, .thematicBreak, .htmlBlock, .footnoteDefinition, .alert,
            .subtext:
            // A block inside a line: only its letters
            append(plainText(node), looks: looks, link: link)
        }
    }

    /// Starts a paragraph. The first one of an entry of a list takes the mark of the entry.
    private mutating func begin(heading: Int) {
        finish()
        open = ShownParagraph(heading: heading, depth: depth, quoted: quoted, mark: mark)
        mark = nil
        if let lead {
            open?.pieces.append(ShownPiece(text: lead))
            self.lead = nil
        }
    }

    private mutating func append(_ text: String, looks: Looks, link: String?) {
        if text.isEmpty { return }
        if open == nil { begin(heading: 0) }
        if let last = open?.pieces.indices.last, open?.pieces[last].looks == looks, open?.pieces[last].link == link {
            open?.pieces[last].text += text
        } else {
            open?.pieces.append(ShownPiece(text: text, looks: looks, link: link))
        }
    }

    /// Ends the paragraph that is open. Space at its start and its end is no text: the text
    /// beside an image is cut there with the space between them.
    private mutating func finish() {
        guard var done = open else { return }
        open = nil
        if let first = done.pieces.indices.first {
            while done.pieces[first].text.first?.isWhitespace == true { done.pieces[first].text.removeFirst() }
            if done.pieces[first].text.isEmpty { done.pieces.removeFirst() }
        }
        if let last = done.pieces.indices.last {
            while done.pieces[last].text.last?.isWhitespace == true { done.pieces[last].text.removeLast() }
            if done.pieces[last].text.isEmpty { done.pieces.removeLast() }
        }
        if !done.pieces.isEmpty || done.mark != nil { paragraphs.append(done) }
    }
}
