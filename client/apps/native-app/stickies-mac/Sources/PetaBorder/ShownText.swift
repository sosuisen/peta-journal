import AppKit
import Combine
@_exported import PetaShown
import PetaTypes

extension ShownText {
    /// The text of an item as it is shown: with its decoration, not as Markdown. It is drawn
    /// from the parts that the shared reader makes of the AST (ADR 041): the iOS app reads the
    /// same parts. `image` gives the picture of a file of the assets folder, such as
    /// `assets/picture.png`, when it is here. Until then the words for the image stand there.
    public static func of(
        _ blocks: [ShownBlock], zoom: Double = 1, image: @escaping (String) -> NSImage? = { _ in nil }
    ) -> NSAttributedString {
        var writer = TextWriter(size: NSFont.systemFontSize * zoom, image: image)
        writer.blocks(blocks, depth: 0, quoted: 0)
        return writer.finished()
    }
}

/// Writes the parts of an item, one line or more each, into one text. This is the look of this
/// shell: it is not kept the same as the stylesheet of the web frontend.
private struct TextWriter {
    let size: Double
    let image: (String) -> NSImage?
    private var text = NSMutableAttributedString()
    /// What the first line of the next part starts with: the mark of an entry of a list.
    private var lead: String?

    private static let linkColor = NSColor(srgbRed: 0x0a / 255, green: 0x58 / 255, blue: 0xca / 255, alpha: 1)
    private static let markedColor = NSColor(srgbRed: 1, green: 0xf3 / 255, blue: 0xa3 / 255, alpha: 1)
    private static let quotedColor = NSColor(white: 0x55 / 255, alpha: 1)
    private static let hiddenColor = NSColor(white: 0xdd / 255, alpha: 1)
    /// How wide an image is drawn at most, at the usual size, when the page asks for no width.
    private static let imageWidth = 320.0

    init(size: Double, image: @escaping (String) -> NSImage?) {
        self.size = size
        self.image = image
    }

    /// The text, without the end of the last line: every part ends its line.
    mutating func finished() -> NSAttributedString {
        while text.string.hasSuffix("\n") {
            text.deleteCharacters(in: NSRange(location: text.length - 1, length: 1))
        }
        return text
    }

    /// The parts inside `depth` lists and `quoted` quotes.
    mutating func blocks(_ blocks: [ShownBlock], depth: Int, quoted: Int) {
        for block in blocks {
            let line = Line(depth: depth, quoted: quoted)
            switch block {
            case .text(let paragraphs):
                for paragraph in paragraphs {
                    self.paragraph(paragraph, Line(depth: depth + paragraph.depth, quoted: quoted + paragraph.quoted))
                }
            case .code(let lines):
                startLine(line)
                append(lines, ShownPiece(text: lines, looks: .code), line)
                endLine(line)
            case .table(let rows):
                table(rows, line)
            case .image(let shown):
                startLine(line)
                picture(shown, line)
                endLine(line)
            case .entries(let entries):
                for entry in entries {
                    lead = entry.mark
                    self.blocks(entry.blocks, depth: depth + 1, quoted: quoted)
                    // An entry without anything in it still shows its mark
                    if lead != nil {
                        startLine(Line(depth: depth + 1, quoted: quoted))
                        endLine(line)
                    }
                }
            case .quote(let inside):
                self.blocks(inside, depth: depth, quoted: quoted + 1)
            }
        }
    }

    /// Where a line stands: how far it is moved to the right, and how large its letters are.
    private struct Line {
        var depth: Int
        var quoted: Int
        var heading = 0
        var style: NSParagraphStyle?
    }

    private mutating func paragraph(_ paragraph: ShownParagraph, _ at: Line) {
        var line = at
        line.heading = paragraph.heading
        if let mark = paragraph.mark { lead = mark }
        startLine(line)
        if paragraph.rule {
            append("────────", ShownPiece(text: ""), line)
        }
        for piece in paragraph.pieces {
            // A line break of the item stays inside its paragraph
            append(piece.text.replacingOccurrences(of: "\n", with: "\u{2028}"), piece, line)
        }
        endLine(line)
    }

    /// Writes the mark of the entry that the line is the first of.
    private mutating func startLine(_ line: Line) {
        if let lead {
            append(lead + " ", ShownPiece(text: ""), line)
            self.lead = nil
        }
    }

    private mutating func endLine(_ line: Line) {
        append("\n", ShownPiece(text: ""), line)
    }

    /// The rows of a table, each cell in a box, as macOS draws a table of HTML.
    private mutating func table(_ rows: [ShownTableRow], _ at: Line) {
        let columns = rows.map(\.cells.count).max() ?? 0
        guard columns > 0 else { return }
        let table = NSTextTable()
        table.numberOfColumns = columns
        for (row, shown) in rows.enumerated() {
            for (column, cell) in shown.cells.enumerated() {
                let box = NSTextTableBlock(table: table, startingRow: row, rowSpan: 1, startingColumn: column, columnSpan: 1)
                box.setWidth(0.5, type: .absoluteValueType, for: .border)
                box.setBorderColor(.separatorColor)
                box.setWidth(4, type: .absoluteValueType, for: .padding)
                let style = NSMutableParagraphStyle()
                style.textBlocks = [box]
                var line = at
                line.style = style
                for piece in cell.pieces {
                    var looks = piece
                    if shown.header { looks.looks.insert(.bold) }
                    append(piece.text, looks, line)
                }
                endLine(line)
            }
        }
    }

    /// An image of the assets folder, at the size that the page asks for, or no wider than
    /// `imageWidth`. The words for it stand there until its file is here.
    private mutating func picture(_ shown: ShownImage, _ line: Line) {
        guard let picture = image(shown.path), picture.size.width > 0, picture.size.height > 0 else {
            append("[\(shown.alt.isEmpty ? shown.path : shown.alt)]", ShownPiece(text: ""), line)
            return
        }
        let natural = picture.size
        let zoom = size / NSFont.systemFontSize
        var width = shown.width.map { Double($0) * zoom }
        var height = shown.height.map { Double($0) * zoom }
        if width == nil, height == nil { width = min(natural.width, Self.imageWidth * zoom) }
        if let w = width, height == nil { height = w * natural.height / natural.width }
        if let h = height, width == nil { width = h * natural.width / natural.height }
        let attachment = NSTextAttachment()
        attachment.image = picture
        attachment.bounds = CGRect(x: 0, y: 0, width: width ?? natural.width, height: height ?? natural.height)
        let drawn = NSMutableAttributedString(attachment: attachment)
        drawn.addAttribute(.paragraphStyle, value: paragraphStyle(line), range: NSRange(location: 0, length: drawn.length))
        text.append(drawn)
    }

    /// Letters with the looks and the link of their piece.
    private mutating func append(_ letters: String, _ piece: ShownPiece, _ line: Line) {
        if letters.isEmpty { return }
        let looks = piece.looks
        let shifted = looks.contains(.raised) || looks.contains(.lowered)
        let scale = line.heading > 0 ? Row.headingScales[min(line.heading, 5) - 1] : 1
        let points = size * scale * (shifted ? 0.75 : 1)
        var font =
            looks.contains(.code)
            ? NSFont.monospacedSystemFont(ofSize: points, weight: .regular)
            : NSFont.systemFont(ofSize: points, weight: line.heading > 0 || looks.contains(.bold) ? .bold : .regular)
        if looks.contains(.italic) { font = NSFontManager.shared.convert(font, toHaveTrait: .italicFontMask) }
        var attributes: [NSAttributedString.Key: Any] = [
            .font: font,
            .foregroundColor: line.quoted > 0 ? Self.quotedColor : NSColor.labelColor,
            .paragraphStyle: line.style ?? paragraphStyle(line),
        ]
        if looks.contains(.struck) { attributes[.strikethroughStyle] = NSUnderlineStyle.single.rawValue }
        if looks.contains(.underlined) { attributes[.underlineStyle] = NSUnderlineStyle.single.rawValue }
        if looks.contains(.marked) { attributes[.backgroundColor] = Self.markedColor }
        if looks.contains(.hidden) { attributes[.backgroundColor] = Self.hiddenColor }
        if looks.contains(.raised) { attributes[.baselineOffset] = size * 0.35 }
        if looks.contains(.lowered) { attributes[.baselineOffset] = -size * 0.15 }
        if let link = piece.link, let address = URL(string: link, relativeTo: ShownText.base)?.absoluteURL {
            attributes[.link] = address
            attributes[.foregroundColor] = Self.linkColor
        }
        text.append(NSAttributedString(string: letters, attributes: attributes))
    }

    /// Lists and quotes move their lines to the right.
    private func paragraphStyle(_ line: Line) -> NSParagraphStyle {
        let style = NSMutableParagraphStyle()
        let indent = CGFloat(line.depth + line.quoted) * size * 1.5
        style.firstLineHeadIndent = indent
        style.headIndent = indent
        return style
    }
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

    private let shell: Shell
    private var made: [String: Made] = [:]
    /// Counts up whenever the rows are to be drawn again. A row is handed this number: SwiftUI
    /// draws a row again only when what it is given changes.
    public private(set) var changes = 0
    private var wanted: [String: (ast: Node, zoom: Double)] = [:]
    private var arrivals: AnyCancellable?

    /// The texts of the items of the page that `shell` has open.
    public init(shell: Shell) {
        self.shell = shell
        // An image that has come is drawn in the texts that waited for it
        arrivals = shell.images.$arrived.dropFirst().sink { [weak self] _ in
            self?.made = [:]
            self?.redraw()
        }
    }

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
                let text = ShownText.of(shell.shownBlocks(asked.ast), zoom: asked.zoom, image: shell.images.image)
                made[id] = Made(ast: asked.ast, zoom: asked.zoom, text: text)
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
