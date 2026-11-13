import Foundation

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

/// Letters with one look. `link` is where a touch leads, as the `href` of the HTML has it.
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

/// Reads the HTML that the core makes of the text of an item (`ShownBlock.text`). iOS can
/// make text with attributes of HTML, but only with a web view behind it, on the main thread
/// and slowly. The core writes few tags, so they are read here.
public func shownParagraphs(html: String) -> [ShownParagraph] {
    var reader = HtmlReader()
    for part in parts(of: html) { reader.take(part) }
    reader.end()
    return reader.paragraphs
}

private enum Part {
    case text(String)
    /// `name` in lower case. `closes` for `</name>`.
    case tag(name: String, closes: Bool, attributes: [String: String])
}

/// Cuts HTML into its tags and the text between them.
private func parts(of html: String) -> [Part] {
    var found: [Part] = []
    var text = ""
    var rest = Substring(html)
    while let open = rest.firstIndex(of: "<") {
        text += rest[..<open]
        let after = rest[rest.index(after: open)...]
        // A tag starts with a letter, or with the slash of a closing tag
        if let first = after.first, first.isLetter || first == "/", let close = after.firstIndex(of: ">") {
            if !text.isEmpty { found.append(.text(decoded(text))) }
            text = ""
            found.append(tag(after[..<close]))
            rest = after[after.index(after: close)...]
        } else {
            text += "<"
            rest = after
        }
    }
    text += rest
    if !text.isEmpty { found.append(.text(decoded(text))) }
    return found
}

/// Reads what is between `<` and `>`.
private func tag(_ inside: Substring) -> Part {
    var rest = inside
    let closes = rest.first == "/"
    if closes { rest.removeFirst() }
    let name = rest.prefix { $0.isLetter || $0.isNumber }.lowercased()
    rest = rest.dropFirst(name.count)
    var attributes: [String: String] = [:]
    while true {
        rest = rest.drop { $0.isWhitespace || $0 == "/" }
        let key = rest.prefix { !$0.isWhitespace && $0 != "=" && $0 != "/" }
        if key.isEmpty { break }
        rest = rest.dropFirst(key.count)
        var value = ""
        if rest.first == "=" {
            rest.removeFirst()
            if rest.first == "\"" {
                rest.removeFirst()
                value = String(rest.prefix { $0 != "\"" })
                rest = rest.dropFirst(value.count + 1)
            } else {
                value = String(rest.prefix { !$0.isWhitespace })
                rest = rest.dropFirst(value.count)
            }
        }
        attributes[key.lowercased()] = decoded(value)
    }
    return .tag(name: name, closes: closes, attributes: attributes)
}

private let named: [String: String] = ["amp": "&", "lt": "<", "gt": ">", "quot": "\"", "apos": "'", "nbsp": "\u{a0}"]

/// Puts the letters back that are written as `&name;` or `&#number;`.
private func decoded(_ text: String) -> String {
    guard text.contains("&") else { return text }
    var out = ""
    var rest = Substring(text)
    while let at = rest.firstIndex(of: "&") {
        out += rest[..<at]
        let after = rest[rest.index(after: at)...]
        // The longest name is short: a `;` far away belongs to something else
        if let end = after.prefix(10).firstIndex(of: ";"), let letter = letter(of: after[..<end]) {
            out += letter
            rest = after[after.index(after: end)...]
        } else {
            out += "&"
            rest = after
        }
    }
    return out + rest
}

private func letter(of name: Substring) -> String? {
    if let known = named[String(name)] { return known }
    guard name.first == "#" else { return nil }
    let digits = name.dropFirst()
    let code = digits.first == "x" || digits.first == "X" ? UInt32(digits.dropFirst(), radix: 16) : UInt32(digits)
    return code.flatMap(Unicode.Scalar.init).map { String(Character($0)) }
}

private struct HtmlReader {
    private(set) var paragraphs: [ShownParagraph] = []

    /// The paragraph that takes the next text, or nil between two paragraphs.
    private var open: ShownParagraph?
    /// The lists that are open, each with the number of its next entry, or nil for bullets.
    private var lists: [Int?] = []
    private var quoted = 0
    /// The mark of the entry that has started and has no paragraph yet.
    private var mark: String?
    /// How many tags of each look are open.
    private var looks: [(String, Looks)] = []
    private var links: [String] = []
    /// A line break was just written: the line end that follows it in the HTML is no space.
    private var broke = false

    private static let lookOf: [String: Looks] = [
        "strong": .bold, "b": .bold, "em": .italic, "i": .italic, "del": .struck, "s": .struck,
        "u": .underlined, "mark": .marked, "code": .code, "sup": .raised, "sub": .lowered,
    ]

    mutating func take(_ part: Part) {
        switch part {
        case .text(let text):
            write(text)
        case .tag(let name, let closes, let attributes):
            if closes { close(name) } else { start(name, attributes) }
        }
    }

    mutating func end() {
        finish()
    }

    private mutating func start(_ name: String, _ attributes: [String: String]) {
        switch name {
        case "p":
            begin(heading: 0)
        case "h1", "h2", "h3", "h4", "h5", "h6":
            begin(heading: Int(name.dropFirst()) ?? 0)
        case "ul":
            finish()
            lists.append(nil)
        case "ol":
            finish()
            lists.append(Int(attributes["start"] ?? "") ?? 1)
        case "li":
            finish()
            if let last = lists.indices.last, let number = lists[last] {
                mark = "\(number)."
                lists[last] = number + 1
            } else {
                mark = "•"
            }
        case "input":
            // The checkbox of a task stands for the bullet of its entry
            let box = attributes["checked"] == nil ? "☐" : "☑"
            if open == nil { mark = box } else { open?.mark = box }
        case "blockquote":
            finish()
            quoted += 1
        case "hr":
            finish()
            paragraphs.append(ShownParagraph(depth: lists.count, quoted: quoted, rule: true))
        case "br":
            append("\n")
            broke = true
        case "a":
            links.append(attributes["href"] ?? "")
        case "img":
            // An image has a part of its own (`ShownBlock.image`). One that is left in the
            // text shows what stands for it
            write(attributes["alt"] ?? "")
        case "span":
            looks.append((name, attributes["class"] == "spoiler" ? .hidden : []))
        default:
            if let look = HtmlReader.lookOf[name] { looks.append((name, look)) }
        }
    }

    private mutating func close(_ name: String) {
        switch name {
        case "p", "h1", "h2", "h3", "h4", "h5", "h6":
            finish()
        case "li":
            finish()
            // An entry without text of its own leaves its mark to no other paragraph
            mark = nil
        case "ul", "ol":
            finish()
            if !lists.isEmpty { lists.removeLast() }
        case "blockquote":
            finish()
            quoted = max(0, quoted - 1)
        case "a":
            if !links.isEmpty { links.removeLast() }
        default:
            if let at = looks.lastIndex(where: { $0.0 == name }) { looks.remove(at: at) }
        }
    }

    /// Starts a paragraph. The first one of an entry of a list takes the mark of the entry.
    private mutating func begin(heading: Int) {
        finish()
        open = ShownParagraph(heading: heading, depth: lists.count, quoted: quoted, mark: mark)
        mark = nil
    }

    private mutating func write(_ text: String) {
        var words = text.replacingOccurrences(of: "\r", with: "")
        if broke, words.hasPrefix("\n") { words.removeFirst() }
        // Between two paragraphs the HTML has line ends that are no text
        if open == nil, words.allSatisfy(\.isWhitespace) { return }
        append(words.replacingOccurrences(of: "\n", with: " "))
    }

    private mutating func append(_ text: String) {
        broke = false
        if text.isEmpty { return }
        // Text that is in no paragraph, as in a list without room between its entries
        if open == nil { begin(heading: 0) }
        let look = looks.reduce(into: Looks()) { $0.formUnion($1.1) }
        if let last = open?.pieces.indices.last, open?.pieces[last].looks == look, open?.pieces[last].link == links.last {
            open?.pieces[last].text += text
        } else {
            open?.pieces.append(ShownPiece(text: text, looks: look, link: links.last))
        }
    }

    /// Ends the paragraph that is open. The line end before a closing tag is no text.
    private mutating func finish() {
        broke = false
        guard var done = open else { return }
        open = nil
        if let last = done.pieces.indices.last {
            while done.pieces[last].text.last?.isWhitespace == true { done.pieces[last].text.removeLast() }
            if done.pieces[last].text.isEmpty { done.pieces.removeLast() }
        }
        if !done.pieces.isEmpty || done.mark != nil { paragraphs.append(done) }
    }
}
