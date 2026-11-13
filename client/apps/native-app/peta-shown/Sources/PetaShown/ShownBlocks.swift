import Foundation
import PetaTypes

/// One part of an item as it is shown, from top to bottom.
public indirect enum ShownBlock: Equatable {
    /// Text with the decoration of its letters and its links.
    case text([ShownParagraph])
    /// A block of code: its lines as they were written.
    case code(lines: String)
    case table(rows: [ShownTableRow])
    case image(ShownImage)
    /// A list that holds a part of its own kind, such as a code block: its entries apart.
    case entries([ShownEntry])
    /// A quote that holds a part of its own kind.
    case quote([ShownBlock])
}

public struct ShownTableRow: Equatable {
    /// True for the row of the titles of the columns.
    public let header: Bool
    /// What is in each cell: one paragraph each.
    public let cells: [ShownParagraph]
}

/// An image of a page.
public struct ShownImage: Equatable {
    /// Where its file is under the Markdown folder, such as `assets/picture.png`.
    public let path: String
    /// What stands for the image where it cannot be seen.
    public let alt: String
    /// The width that the page asks for, in points. `nil` when it asks for none.
    public let width: Int?
    /// The height that the page asks for, the same way.
    public let height: Int?

    public init(path: String, alt: String, width: Int?, height: Int?) {
        self.path = path
        self.alt = alt
        self.width = width
        self.height = height
    }
}

public struct ShownEntry: Equatable {
    /// What stands before the entry of a list: a bullet, its number or a checkbox.
    public let mark: String
    public let blocks: [ShownBlock]
}

/// An item as it is shown: with its decoration, not as Markdown. It is drawn from the AST that
/// the core gives with the row (ADR 041).
///
/// A code block and a table get parts of their own: one text cannot show them. An image of the
/// assets folder gets a part of its own, between the text before and after it in its
/// paragraph. An image of the web is not drawn: it becomes a link to its address.
///
/// A list or a quote that holds one of these parts cannot be one text. It is taken apart, and
/// what is in it is read the same way. A list or a quote that holds none stays in the text.
///
/// A link to a page and a tag have the URL of their page in the app as their link, such as
/// `/pages/notes`, as `links` writes it.
public func shownBlocks(ast: Node, links: PageLinks) -> [ShownBlock] {
    Reader(links: links).read(children(of: ast))
}

/// How the link of an item to a page and to a tag is written: as the URL of the page in the
/// app, such as `/pages/notes`. The core writes it, and each app has a core of its own.
public struct PageLinks {
    /// The URL of the page of a wikilink, from the page that the link names.
    let page: (String) -> String
    /// The URL of the page of a hashtag, from the tag without its `#`.
    let tag: (String) -> String

    public init(page: @escaping (String) -> String, tag: @escaping (String) -> String) {
        self.page = page
        self.tag = tag
    }
}

public enum ShownText {
    /// What the links of an item are relative to. A link to a page, such as `/pages/notes`,
    /// has no address of its own: it opens in the app.
    public static let base = URL(string: "peta-journal://app/")!
}

/// Where a link of an item leads.
public enum LinkTarget: Equatable {
    /// An address that the browser opens.
    case web(URL)
    /// The URL of a page, such as `/pages/notes`, as it is written and not encoded.
    case page(String)
    /// The path of a file of the assets folder, such as `assets/report.pdf`.
    case file(String)

    /// The target of a link of an item, made whole against `ShownText.base`. `nil` for any
    /// other link, such as one to a file of the PC: it opens nothing.
    public init?(of link: URL) {
        if link.scheme == "https" || link.scheme == "http" {
            self = .web(link)
            return
        }
        guard link.scheme == ShownText.base.scheme else { return nil }
        let path = link.path
        if path.hasPrefix("/pages/") {
            self = .page(path + (link.fragment.map { "#" + ($0.removingPercentEncoding ?? $0) } ?? ""))
        } else if let file = assetPath(path) {
            self = .file(file)
        } else {
            return nil
        }
    }
}

/// The path under the Markdown folder of the file that the address names, when the file is in
/// the assets folder: `assets/picture.png` for `../assets/picture.png`. `nil` for any other
/// address. The address may be encoded as a web address is.
public func assetPath(_ address: String) -> String? {
    let file = address.components(separatedBy: "#")[0]
    let path = String((file.removingPercentEncoding ?? file).drop { $0 == "." || $0 == "/" })
    return path.lowercased().hasPrefix(assets) ? path : nil
}

/// The folder of the files of the pages, such as images, under the Markdown folder. The sync
/// of the iOS app names it too (`MarkdownFolder.assets`): it knows nothing of what is shown.
private let assets = "assets/"


private final class Reader {
    private let links: PageLinks
    private var blocks: [ShownBlock] = []
    /// The nodes since the last block of its own kind, which become one text together.
    private var others: [Node] = []

    init(links: PageLinks) {
        self.links = links
    }

    /// The parts of these nodes: those of the document, of an entry of a list, or of a quote.
    func read(_ nodes: [Node]) -> [ShownBlock] {
        for node in nodes {
            switch node {
            case .list(let listType, let start, _, _, let entries) where holdsOwnPart(node):
                endText()
                blocks.append(.entries(self.entries(entries, ordered: listType == .ordered, from: Int(start))))
            case .blockQuote(let inside) where holdsOwnPart(node),
                .multilineBlockQuote(let inside) where holdsOwnPart(node):
                endText()
                blocks.append(.quote(within(inside)))
            case .codeBlock(_, _, let literal):
                endText()
                var lines = Substring(literal)
                while lines.hasSuffix("\n") { lines = lines.dropLast() }
                blocks.append(.code(lines: String(lines)))
            case .table(_, _, _, let rows):
                endText()
                blocks.append(.table(rows: rows.map(row)))
            case .paragraph(let inline):
                paragraph(inline)
            default:
                others.append(node)
            }
        }
        endText()
        return blocks
    }

    private func within(_ nodes: [Node]) -> [ShownBlock] {
        Reader(links: links).read(nodes)
    }

    private func entries(_ entries: [Node], ordered: Bool, from first: Int) -> [ShownEntry] {
        entries.enumerated().map { index, entry in
            ShownEntry(mark: mark(of: entry, ordered: ordered, number: first + index), blocks: within(children(of: entry)))
        }
    }

    /// A paragraph is cut at each image of the assets folder: the image stands on its own.
    private func paragraph(_ inline: [Node]) {
        var run: [Node] = []
        func endRun() {
            if !run.isEmpty { others.append(.paragraph(children: run)) }
            run = []
        }
        for child in inline {
            if let image = assetImage(child) {
                endRun()
                endText()
                blocks.append(.image(image))
            } else {
                run.append(child)
            }
        }
        endRun()
    }

    private func endText() {
        if others.isEmpty { return }
        // Between two images there may be only a space
        let blank = others.allSatisfy {
            if case .paragraph = $0 { return plainText($0).allSatisfy(\.isWhitespace) }
            return false
        }
        if !blank { blocks.append(.text(shownParagraphs(of: others, links: links))) }
        others = []
    }

    private func row(_ row: Node) -> ShownTableRow {
        guard case .tableRow(let header, let cells) = row else { return ShownTableRow(header: false, cells: []) }
        // What is in a cell is part of a line: it is one paragraph
        return ShownTableRow(
            header: header,
            cells: cells.map { cell in
                shownParagraphs(of: [.paragraph(children: children(of: cell))], links: links).first
                    ?? ShownParagraph()
            })
    }
}

/// What stands before an entry of a list: a checkbox for a task, its number in a numbered
/// list, and a bullet otherwise.
func mark(of entry: Node, ordered: Bool, number: Int) -> String {
    if case .taskItem(let symbol, _) = entry { return symbol == nil ? "☐" : "☑" }
    return ordered ? "\(number)." : "•"
}

/// True when a part of its own kind is somewhere under the node.
private func holdsOwnPart(_ node: Node) -> Bool {
    children(of: node).contains { child in
        switch child {
        case .codeBlock, .table: return true
        default: return assetImage(child) != nil || holdsOwnPart(child)
        }
    }
}

/// The image that the node is, when its file is in the assets folder. A page writes the
/// address as `../assets/picture.png`, from the URL of the page in the web app.
private func assetImage(_ node: Node) -> ShownImage? {
    guard case .embed(let url, _, let props, _) = node, let path = assetPath(url) else { return nil }
    // A side that is not a number above zero is not asked for, as in the web app
    func side(_ name: String) -> Int? {
        guard case .text(let value) = props[name], let number = Int(value), number > 0 else { return nil }
        return number
    }
    return ShownImage(path: path, alt: plainText(node), width: side("w"), height: side("h"))
}
