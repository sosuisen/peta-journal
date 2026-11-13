import AppKit
import PetaCore
import PetaTypes

/// The text of an item as it is shown: with its decoration, not as Markdown. It is drawn from
/// the AST that the core gives with the row (ADR 041). A `switch` takes every kind of node, so
/// a kind that the AST gets later stops the build until it is drawn here.
public enum ShownText {
    /// Wikilinks and hashtags name a page or a tag, and have no address. As links of the text
    /// they get an address of this kind, to tell them from links that lead out of the journal:
    /// the host says whether it is a page or a tag.
    private static let pages = URL(string: "peta-link://page/")!
    private static let tags = URL(string: "peta-link://tag/")!

    /// The text of the AST of an item.
    public static func of(_ ast: Node, zoom: Double = 1) -> NSAttributedString {
        var writer = TextWriter(size: NSFont.systemFontSize * zoom)
        writer.blocks(children(of: ast))
        return writer.text
    }

    /// What a link of the shown text leads to. Nothing for a link that stays in the item, as
    /// the one of a footnote.
    public static func target(of link: URL) -> LinkTarget? {
        guard link.scheme == pages.scheme else { return .out(link) }
        let name = String(link.path.dropFirst())
        if name.isEmpty { return nil }
        return link.host == tags.host ? .tag(name) : .page(name)
    }

    /// The address of a link to the page or the tag of this name.
    static func link(to name: String, tag: Bool) -> URL? {
        let path = name.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? name
        return URL(string: path, relativeTo: tag ? tags : pages)?.absoluteURL
    }
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

/// How the letters of a piece look. This is the look of this shell: it is not kept the same as
/// the stylesheet of the web frontend.
private struct Looks {
    var bold = false
    var italic = false
    var code = false
    var struck = false
    var underlined = false
    var marked = false
    var hidden = false
    /// Above the line (1), below it (-1), or on it (0).
    var shift = 0
    var link: URL?
}

/// Writes the blocks of an item, one line or more each, into one text.
private struct TextWriter {
    let size: Double
    private(set) var text = NSMutableAttributedString()
    /// How many lists and quotes the block is inside.
    private var depth = 0
    private var quoted = 0
    /// What the first line of the next block starts with: the mark of an entry of a list, or
    /// the name of a footnote.
    private var lead: String?
    private var heading = 0

    private static let linkColor = NSColor(srgbRed: 0x0a / 255, green: 0x58 / 255, blue: 0xca / 255, alpha: 1)
    private static let markedColor = NSColor(srgbRed: 1, green: 0xf3 / 255, blue: 0xa3 / 255, alpha: 1)
    private static let quotedColor = NSColor(white: 0x55 / 255, alpha: 1)
    private static let hiddenColor = NSColor(white: 0xdd / 255, alpha: 1)

    init(size: Double) {
        self.size = size
    }

    mutating func blocks(_ nodes: [Node]) {
        for node in nodes { block(node) }
        // Every block ends its line, the last one too
        while text.string.hasSuffix("\n") {
            text.deleteCharacters(in: NSRange(location: text.length - 1, length: 1))
        }
    }

    private mutating func block(_ node: Node) {
        switch node {
        case .document(let children), .item(_, _, let children), .taskItem(_, let children), .unknown(_, let children):
            for child in children { block(child) }
        case .paragraph(let inline):
            line(inline)
        case .heading(let level, let inline):
            heading = Int(level)
            line(inline)
            heading = 0
        case .codeBlock(_, _, let literal):
            var lines = Substring(literal)
            while lines.hasSuffix("\n") { lines = lines.dropLast() }
            line([.code(literal: String(lines))])
        case .blockQuote(let children), .multilineBlockQuote(let children):
            quoted += 1
            for child in children { block(child) }
            quoted -= 1
        case .alert(let type, let title, let children):
            quoted += 1
            line([.strong(children: [.text(value: title ?? name(of: type))])])
            for child in children { block(child) }
            quoted -= 1
        case .list(let listType, let start, _, _, let entries):
            depth += 1
            for (index, entry) in entries.enumerated() {
                if case .taskItem(let symbol, _) = entry {
                    lead = symbol == nil ? "☐ " : "☑ "
                } else {
                    lead = listType == .ordered ? "\(Int(start) + index). " : "• "
                }
                block(entry)
                lead = nil
            }
            depth -= 1
        case .table(_, _, _, let rows):
            table(rows)
        case .tableRow(_, let cells):
            line(cells.flatMap { children(of: $0) + [.text(value: "  ")] })
        case .tableCell(let inline):
            line(inline)
        case .thematicBreak:
            line([.text(value: "────────")])
        case .htmlBlock(_, let literal):
            var lines = Substring(literal)
            while lines.hasSuffix("\n") { lines = lines.dropLast() }
            line([.text(value: String(lines))])
        case .footnoteDefinition(let name, let children):
            lead = "[\(footnoteLabel(name))]: "
            for child in children { block(child) }
            lead = nil
        case .subtext(let inline):
            line([.subscriptText(children: inline)])
        case .text, .softbreak, .linebreak, .emph, .strong, .strikethrough, .underline, .highlight,
            .superscript, .subscriptText, .spoileredText, .code, .link, .embed, .footnoteReference,
            .shortcode, .math, .htmlInline, .wikilink, .hashtag:
            // A line outside a block, as in a list without room between its entries
            line([node])
        }
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

    /// One line, or more lines of one block, with the lead of the block before it.
    private mutating func line(_ inline: [Node], style: NSParagraphStyle? = nil) {
        if let lead {
            append(lead, Looks())
            self.lead = nil
        }
        for node in inline { write(node, Looks()) }
        append("\n", Looks(), style: style)
    }

    /// The rows of a table, each cell in a box, as macOS draws a table of HTML.
    private mutating func table(_ rows: [Node]) {
        let columns = rows.map { children(of: $0).count }.max() ?? 0
        guard columns > 0 else { return }
        let table = NSTextTable()
        table.numberOfColumns = columns
        for (row, node) in rows.enumerated() {
            guard case .tableRow(let header, let cells) = node else { continue }
            for (column, cell) in cells.enumerated() {
                let box = NSTextTableBlock(table: table, startingRow: row, rowSpan: 1, startingColumn: column, columnSpan: 1)
                box.setWidth(0.5, type: .absoluteValueType, for: .border)
                box.setBorderColor(.separatorColor)
                box.setWidth(4, type: .absoluteValueType, for: .padding)
                let style = NSMutableParagraphStyle()
                style.textBlocks = [box]
                let inline = children(of: cell)
                line(header ? [.strong(children: inline)] : inline, style: style)
            }
        }
    }

    /// The letters of an inline node, with the looks of the nodes around it.
    private mutating func write(_ node: Node, _ looks: Looks) {
        func inside(_ children: [Node], _ change: (inout Looks) -> Void = { _ in }) {
            var more = looks
            change(&more)
            for child in children { write(child, more) }
        }
        switch node {
        case .text(let value): append(value, looks)
        case .softbreak: append(" ", looks)
        case .linebreak: append("\u{2028}", looks)
        case .emph(let children): inside(children) { $0.italic = true }
        case .strong(let children): inside(children) { $0.bold = true }
        case .strikethrough(let children): inside(children) { $0.struck = true }
        case .underline(let children): inside(children) { $0.underlined = true }
        case .highlight(let children): inside(children) { $0.marked = true }
        case .superscript(let children): inside(children) { $0.shift = 1 }
        case .subscriptText(let children): inside(children) { $0.shift = -1 }
        case .spoileredText(let children): inside(children) { $0.hidden = true }
        case .code(let literal):
            var code = looks
            code.code = true
            append(literal, code)
        case .link(let url, _, let children): inside(children) { $0.link = URL(string: url) }
        case .wikilink(let url, let children): inside(children) { $0.link = ShownText.link(to: url, tag: false) }
        case .hashtag(let value):
            var tag = looks
            tag.link = ShownText.link(to: value, tag: true)
            append("#" + value, tag)
        case .embed(let url, _, _, _):
            // An image is shown as the words for it, and without any as its file. The image
            // itself is not shown yet
            let alt = plainText(node)
            append("[\(alt.isEmpty ? url : alt)]", looks)
        case .footnoteReference(let name, _, _):
            var raised = looks
            raised.shift = 1
            append("[\(footnoteLabel(name))]", raised)
        case .shortcode(_, let emoji): append(emoji, looks)
        case .math(let literal), .htmlInline(let literal): append(literal, looks)
        case .unknown(_, let children): inside(children)
        case .document, .paragraph, .heading, .codeBlock, .blockQuote, .multilineBlockQuote, .list, .item,
            .taskItem, .table, .tableRow, .tableCell, .thematicBreak, .htmlBlock, .footnoteDefinition, .alert,
            .subtext:
            // A block inside a line: only its letters
            append(plainText(node), looks)
        }
    }

    private mutating func append(_ letters: String, _ looks: Looks, style: NSParagraphStyle? = nil) {
        if letters.isEmpty { return }
        let scale = heading > 0 ? Row.headingScales[min(heading, 5) - 1] : 1
        let points = size * scale * (looks.shift == 0 ? 1 : 0.75)
        var font =
            looks.code
            ? NSFont.monospacedSystemFont(ofSize: points, weight: .regular)
            : NSFont.systemFont(ofSize: points, weight: heading > 0 || looks.bold ? .bold : .regular)
        if looks.italic { font = NSFontManager.shared.convert(font, toHaveTrait: .italicFontMask) }
        var attributes: [NSAttributedString.Key: Any] = [
            .font: font,
            .foregroundColor: quoted > 0 ? Self.quotedColor : NSColor.labelColor,
            .paragraphStyle: style ?? paragraphStyle(),
        ]
        if looks.struck { attributes[.strikethroughStyle] = NSUnderlineStyle.single.rawValue }
        if looks.underlined { attributes[.underlineStyle] = NSUnderlineStyle.single.rawValue }
        if looks.marked { attributes[.backgroundColor] = Self.markedColor }
        if looks.hidden { attributes[.backgroundColor] = Self.hiddenColor }
        if looks.shift != 0 { attributes[.baselineOffset] = looks.shift > 0 ? size * 0.35 : -size * 0.15 }
        if let link = looks.link {
            attributes[.link] = link
            attributes[.foregroundColor] = Self.linkColor
        }
        text.append(NSAttributedString(string: letters, attributes: attributes))
    }

    /// Lists and quotes move their lines to the right.
    private func paragraphStyle() -> NSParagraphStyle {
        let style = NSMutableParagraphStyle()
        let indent = CGFloat(depth + quoted) * size * 1.5
        style.firstLineHeadIndent = indent
        style.headIndent = indent
        return style
    }
}

/// The text that a reader sees for a footnote. An inline footnote has a name that the parser
/// made (`__inline_1`), so its number is shown instead.
private func footnoteLabel(_ name: String) -> String {
    name.hasPrefix("__inline_") ? String(name.dropFirst("__inline_".count)) : name
}

/// What a link of a shown text leads to.
public enum LinkTarget: Equatable {
    /// The page of a wikilink, by its path.
    case page(String)
    /// The tag of a hashtag, by its name. Its page lies in the folder of the tags.
    case tag(String)
    /// An address outside the journal.
    case out(URL)
}

/// The shown texts of a screen that were made already. A text is not made while SwiftUI
/// draws: a row that has no text yet asks for it here, and is told when it is there.
@MainActor
public final class ShownTexts: ObservableObject {
    private struct Made {
        let ast: Node
        let zoom: Double
        let text: NSAttributedString
    }

    private var made: [String: Made] = [:]
    /// Counts up whenever the rows are to be drawn again. A row is handed this number: SwiftUI
    /// draws a row again only when what it is given changes.
    public private(set) var changes = 0
    private var wanted: [String: (ast: Node, zoom: Double)] = [:]

    public init() {}

    /// The text of the item with this AST, when it was made already.
    public func text(of itemId: String, ast: Node, zoom: Double) -> NSAttributedString? {
        guard let made = made[itemId], made.ast == ast, made.zoom == zoom else { return nil }
        return made.text
    }

    /// Makes the text of the item soon, outside the drawing, and tells that it changed.
    public func want(_ itemId: String, ast: Node, zoom: Double) {
        let first = wanted.isEmpty
        wanted[itemId] = (ast, zoom)
        guard first else { return }
        // All the rows of one drawing are made together, and told once
        DispatchQueue.main.async { [self] in
            objectWillChange.send()
            changes += 1
            for (id, asked) in wanted {
                made[id] = Made(ast: asked.ast, zoom: asked.zoom, text: ShownText.of(asked.ast, zoom: asked.zoom))
            }
            wanted = [:]
        }
    }

    /// Has the rows drawn again soon. The cursor left an item: nothing that the row is given
    /// says so, and the item is to show its decoration again.
    public func redraw() {
        DispatchQueue.main.async { [self] in
            objectWillChange.send()
            changes += 1
        }
    }
}
