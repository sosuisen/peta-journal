import AppKit
import Combine
import PetaCore
import PetaTypes

/// The text of an item as it is shown: with its decoration, not as Markdown.
public enum ShownText {
    /// The text that is drawn from the parts that the core makes of the body (ADR 042). `image`
    /// gives the picture of a file of the assets folder, such as `assets/picture.png`, when it
    /// is here. Until then the words for the image stand there.
    public static func of(
        _ rich: RichText, zoom: Double = 1, image: @escaping (String) -> NSImage? = { _ in nil }
    ) -> NSAttributedString {
        var writer = TextWriter(size: NSFont.systemFontSize * zoom, image: image)
        writer.blocks(rich.blocks, indent: 0, quoted: 0)
        return writer.finished()
    }

    /// How many letters of the shown text of the item stand before a click at `index` of a
    /// shown text, as the core counts them (`RichParagraph.start`, ADR 043). A click on a
    /// decoration, such as the mark of an entry, is at the letter after it; one at the end of
    /// a line is after its last letter. `nil` when no letter of the core is near.
    public static func letter(in text: NSAttributedString, at index: Int) -> Int? {
        func counted(at place: Int) -> (start: Int, range: NSRange)? {
            guard place >= 0, place < text.length else { return nil }
            var range = NSRange()
            let start = text.attribute(.shownLetter, at: place, effectiveRange: &range) as? Int
            return start.map { ($0, range) }
        }
        func letters(of range: NSRange, before place: Int) -> Int {
            let letters = (text.string as NSString).substring(with: range)
            return Int(charsOfUtf16(text: letters, units: UInt64(place - range.location)))
        }
        if let (start, range) = counted(at: index) { return start + letters(of: range, before: index) }
        if let (start, range) = counted(at: index - 1) {
            return start + letters(of: range, before: range.location + range.length)
        }
        var next = index + 1
        while next < text.length {
            if let (start, _) = counted(at: next) { return start }
            next += 1
        }
        return nil
    }

    /// Where the link of the letters at `index` of a shown text leads. `nil` for letters
    /// without a link.
    public static func link(in text: NSAttributedString, at index: Int) -> RichLink? {
        (text.attribute(.richLink, at: index, effectiveRange: nil) as? HeldLink)?.link
    }
}

extension Looks {
    /// Letters without any look of their own.
    static let plain = Looks(
        bold: false, italic: false, struck: false, underlined: false, marked: false, code: false, raised: false,
        lowered: false, hidden: false)
}

extension NSAttributedString.Key {
    /// Where the letters lead, as the core says (`HeldLink`).
    static let richLink = NSAttributedString.Key("PetaRichLink")
    /// How many letters of the shown text of the item stand before the letters, as the core
    /// counts them. Decorations that this shell adds have none.
    static let shownLetter = NSAttributedString.Key("PetaShownLetter")
}

/// A link that a text holds as the value of an attribute. Two that lead to the same place are
/// equal, so that the letters of one link stay one run.
final class HeldLink: NSObject {
    let link: RichLink

    init(_ link: RichLink) {
        self.link = link
    }

    override func isEqual(_ object: Any?) -> Bool {
        (object as? HeldLink)?.link == link
    }

    override var hash: Int { link.hashValue }
}

/// A picture as wide as the page asks for. A picture whose page asks for no size is as wide as
/// the room it has in the line, and never wider than its file.
final class FittedPicture: NSTextAttachment {
    /// The width and the height that the page asks for, made larger by the zoom.
    private let asked: CGSize?

    init(_ picture: NSImage, asked: CGSize?) {
        self.asked = asked
        super.init(data: nil, ofType: nil)
        image = picture
        bounds = CGRect(origin: .zero, size: asked ?? picture.size)
    }

    required init?(coder: NSCoder) {
        nil
    }

    override func attachmentBounds(
        for textContainer: NSTextContainer?, proposedLineFragment lineFrag: CGRect, glyphPosition position: CGPoint,
        characterIndex charIndex: Int
    ) -> CGRect {
        CGRect(origin: .zero, size: size(room: lineFrag.width))
    }

    /// The size in a line that is `room` wide.
    func size(room: CGFloat) -> CGSize {
        if let asked { return asked }
        guard let natural = image?.size, natural.width > 0 else { return bounds.size }
        let width = min(natural.width, max(room, 1))
        return CGSize(width: width, height: width * natural.height / natural.width)
    }
}

/// Writes the parts of an item, one line or more each, into one text. The sizes of the headings
/// and the indents come from the core, so that every shell has the same ones (#795).
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

    /// The parts `indent` ems to the right, inside `quoted` quotes. The core says how far each
    /// list and quote moves what is in it (#795).
    mutating func blocks(_ blocks: [RichBlock], indent: Double, quoted: Int) {
        for block in blocks {
            let line = Line(indent: indent, quoted: quoted)
            switch block {
            case .text(let paragraphs):
                for paragraph in paragraphs {
                    let at = Line(indent: indent + Double(paragraph.indent), quoted: quoted + Int(paragraph.quoted))
                    self.paragraph(paragraph, at)
                }
            case .code(let lines, let start):
                startLine(line)
                append(lines, Self.code, line, letter: Int(start))
                endLine(line)
            case .table(let rows):
                table(rows, line)
            case .image(let shown):
                startLine(line)
                picture(shown, line)
                endLine(line)
            case .entries(let entries, let step):
                let inner = Line(indent: indent + Double(step), quoted: quoted)
                for entry in entries {
                    lead = entry.mark
                    self.blocks(entry.blocks, indent: inner.indent, quoted: quoted)
                    // An entry without anything in it still shows its mark
                    if lead != nil {
                        startLine(inner)
                        endLine(inner)
                    }
                }
            case .quote(let inside, let step):
                self.blocks(inside, indent: indent + Double(step), quoted: quoted + 1)
            }
        }
    }

    /// Where a line stands: how far it is moved to the right, and how large its letters are.
    private struct Line {
        /// How far to the right it stands, in ems.
        var indent: Double
        var quoted: Int
        var heading = 0
        /// How much larger than the usual text its letters are: the scale of its heading.
        var scale = 1.0
        var style: NSParagraphStyle?
    }

    private mutating func paragraph(_ paragraph: RichParagraph, _ at: Line) {
        var line = at
        line.heading = Int(paragraph.heading)
        line.scale = Double(paragraph.scale)
        if let mark = paragraph.mark { lead = mark }
        startLine(line)
        if paragraph.rule {
            append("────────", Self.plain, line)
        }
        var letter = Int(paragraph.start)
        for run in paragraph.runs {
            // A line break of the item stays inside its paragraph
            append(run.text.replacingOccurrences(of: "\n", with: "\u{2028}"), run, line, letter: letter)
            letter += run.text.unicodeScalars.count
        }
        endLine(line)
    }

    /// Writes the mark of the entry that the line is the first of.
    private mutating func startLine(_ line: Line) {
        if let lead {
            append(lead + " ", Self.plain, line)
            self.lead = nil
        }
    }

    private mutating func endLine(_ line: Line) {
        append("\n", Self.plain, line)
    }

    /// The rows of a table, each cell in a box, as macOS draws a table of HTML.
    private mutating func table(_ rows: [RichTableRow], _ at: Line) {
        let columns = rows.map(\.cells.count).max() ?? 0
        guard columns > 0 else { return }
        let table = NSTextTable()
        table.numberOfColumns = columns
        for (row, shown) in rows.enumerated() {
            for (column, cell) in shown.cells.enumerated() {
                let box = NSTextTableBlock(
                    table: table, startingRow: row, rowSpan: 1, startingColumn: column, columnSpan: 1)
                box.setWidth(0.5, type: .absoluteValueType, for: .border)
                box.setBorderColor(.separatorColor)
                box.setWidth(4, type: .absoluteValueType, for: .padding)
                let style = NSMutableParagraphStyle()
                style.textBlocks = [box]
                var line = at
                line.style = style
                var letter = Int(cell.start)
                for run in cell.runs {
                    var looks = run
                    if shown.header { looks.looks.bold = true }
                    append(run.text, looks, line, letter: letter)
                    letter += run.text.unicodeScalars.count
                }
                endLine(line)
            }
        }
    }

    /// An image of the assets folder, at the size that the page asks for. An image whose page
    /// asks for no size fits the width of the sticky. The words for it stand there until its
    /// file is here.
    private mutating func picture(_ shown: RichImage, _ line: Line) {
        guard let picture = image(shown.path), picture.size.width > 0, picture.size.height > 0 else {
            append("[\(shown.alt.isEmpty ? shown.path : shown.alt)]", Self.plain, line)
            return
        }
        let natural = picture.size
        let zoom = size / NSFont.systemFontSize
        var width = shown.width.map { Double($0) * zoom }
        var height = shown.height.map { Double($0) * zoom }
        if let w = width, height == nil { height = w * natural.height / natural.width }
        if let h = height, width == nil { width = h * natural.width / natural.height }
        let asked = width.flatMap { w in height.map { CGSize(width: w, height: $0) } }
        let attachment = FittedPicture(picture, asked: asked)
        let drawn = NSMutableAttributedString(attachment: attachment)
        drawn.addAttribute(
            .paragraphStyle, value: paragraphStyle(line), range: NSRange(location: 0, length: drawn.length))
        text.append(drawn)
    }

    private static let plain = RichRun(text: "", looks: .plain, link: nil)
    private static let code: RichRun = {
        var looks = Looks.plain
        looks.code = true
        return RichRun(text: "", looks: looks, link: nil)
    }()

    /// Letters with the looks and the link of their run. `letter` is how many letters of the
    /// shown text, as the core counts them, stand before them; a decoration has none.
    private mutating func append(_ letters: String, _ run: RichRun, _ line: Line, letter: Int? = nil) {
        if letters.isEmpty { return }
        let looks = run.looks
        let shifted = looks.raised || looks.lowered
        let points = size * line.scale * (shifted ? 0.75 : 1)
        var font =
            looks.code
            ? NSFont.monospacedSystemFont(ofSize: points, weight: .regular)
            : NSFont.systemFont(ofSize: points, weight: line.heading > 0 || looks.bold ? .bold : .regular)
        if looks.italic { font = NSFontManager.shared.convert(font, toHaveTrait: .italicFontMask) }
        var attributes: [NSAttributedString.Key: Any] = [
            .font: font,
            .foregroundColor: line.quoted > 0 ? Self.quotedColor : NSColor.labelColor,
            .paragraphStyle: line.style ?? paragraphStyle(line),
        ]
        if looks.struck { attributes[.strikethroughStyle] = NSUnderlineStyle.single.rawValue }
        if looks.underlined { attributes[.underlineStyle] = NSUnderlineStyle.single.rawValue }
        if looks.marked { attributes[.backgroundColor] = Self.markedColor }
        if looks.hidden { attributes[.backgroundColor] = Self.hiddenColor }
        if looks.raised { attributes[.baselineOffset] = size * 0.35 }
        if looks.lowered { attributes[.baselineOffset] = -size * 0.15 }
        if let letter { attributes[.shownLetter] = letter }
        if let link = run.link {
            attributes[.richLink] = HeldLink(link)
            attributes[.foregroundColor] = Self.linkColor
        }
        text.append(NSAttributedString(string: letters, attributes: attributes))
    }

    /// Lists and quotes move their lines to the right.
    private func paragraphStyle(_ line: Line) -> NSParagraphStyle {
        let style = NSMutableParagraphStyle()
        let indent = CGFloat(line.indent * size)
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
        let rich: RichText
        let zoom: Double
        let text: NSAttributedString
    }

    private let shell: Shell
    private var made: [String: Made] = [:]
    /// Counts up whenever the rows are to be drawn again. A row is handed this number: SwiftUI
    /// draws a row again only when what it is given changes.
    public private(set) var changes = 0
    private var wanted: [String: (rich: RichText, zoom: Double)] = [:]
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

    /// The text of the item with these parts, when it was made already.
    public func text(of itemId: String, rich: RichText, zoom: Double) -> NSAttributedString? {
        guard let made = made[itemId], made.rich == rich, made.zoom == zoom else { return nil }
        return made.text
    }

    /// Makes the text of the item soon, outside the drawing, and tells that it changed.
    public func want(_ itemId: String, rich: RichText, zoom: Double) {
        let first = wanted.isEmpty
        wanted[itemId] = (rich, zoom)
        guard first else { return }
        // All the rows of one drawing are made together, and told once
        DispatchQueue.main.async { [self] in
            objectWillChange.send()
            changes += 1
            for (id, asked) in wanted {
                let text = ShownText.of(asked.rich, zoom: asked.zoom, image: shell.images.image)
                made[id] = Made(rich: asked.rich, zoom: asked.zoom, text: text)
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
