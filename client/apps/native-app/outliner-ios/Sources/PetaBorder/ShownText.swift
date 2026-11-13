import Foundation
import PetaCore
import PetaSync

/// One part of an item as it is shown, from top to bottom.
public indirect enum ShownBlock: Equatable {
    /// The body of an item whose AST cannot be read, as it is.
    case plain(String)
    /// Text with the decoration of its letters and its links, as HTML. The screen turns it
    /// into text with attributes: only a screen can.
    case text(html: String)
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
    /// What is in each cell, as HTML.
    public let cells: [String]
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

/// An item as it is shown: with its decoration, not as Markdown (ADR 025).
///
/// Rust makes HTML of the AST of the item: that gives the decoration of letters and the
/// links. The text that iOS makes of HTML loses the look of a code block and of a table, so
/// those two are read from the AST here and get parts of their own, as in the Android shell
/// (ADR 030).
///
/// An image of the assets folder gets a part of its own, between the text before and after it
/// in its paragraph. An image of the web is not drawn: it becomes a link to its address.
///
/// A list or a quote that holds one of these parts cannot be one text. It is taken apart, and
/// what is in it is read the same way. A list or a quote that holds none stays in the text.
///
/// A link to a page and a tag have the URL of their page as their `href`, such as
/// `/pages/notes`, encoded as a web address is. The page of a tag is under `tagsDir`. An item
/// whose AST cannot be read shows its body as it is.
public func shownBlocks(body: String, ast: String, tagsDir: String) -> [ShownBlock] {
    guard !ast.isEmpty, let document = try? JSONSerialization.jsonObject(with: Data(ast.utf8)) as? Node else {
        return [.plain(body)]
    }
    return Reader(tagsDir: tagsDir).read(document)
}

public enum ShownText {
    /// What the links of the HTML are relative to. A link to a page, such as `/pages/notes`,
    /// has no address of its own: the app has no server.
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

    /// The target of a link of the HTML, made whole against `ShownText.base`. `nil` for any
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
    return path.lowercased().hasPrefix(MarkdownFolder.assets) ? path : nil
}

private typealias Node = [String: Any]

private let imageType = "embed"
private let quotes: Set<String> = ["block_quote", "multiline_block_quote"]

private final class Reader {
    private let tagsDir: String
    private var blocks: [ShownBlock] = []
    /// The nodes since the last block of its own kind, which become one text together.
    private var others: [Node] = []

    init(tagsDir: String) {
        self.tagsDir = tagsDir
    }

    /// The parts of what is in the node: the document, an entry of a list, or a quote.
    func read(_ holder: Node) -> [ShownBlock] {
        for node in children(of: holder) {
            let type = string(node, "type")
            if type == "list", holdsOwnPart(node) {
                endText()
                blocks.append(.entries(entries(of: node)))
            } else if quotes.contains(type), holdsOwnPart(node) {
                endText()
                blocks.append(.quote(within(node)))
            } else {
                part(node, type)
            }
        }
        endText()
        return blocks
    }

    private func within(_ holder: Node) -> [ShownBlock] {
        Reader(tagsDir: tagsDir).read(holder)
    }

    private func entries(of list: Node) -> [ShownEntry] {
        let first = list["start"] as? Int ?? 1
        return children(of: list).enumerated().map { index, entry in
            let mark: String
            if string(entry, "type") == "task_item" {
                mark = entry["symbol"] == nil || entry["symbol"] is NSNull ? "☐" : "☑"
            } else if string(list, "list_type") == "ordered" {
                mark = "\(first + index)."
            } else {
                mark = "•"
            }
            return ShownEntry(mark: mark, blocks: within(entry))
        }
    }

    private func part(_ node: Node, _ type: String) {
        switch type {
        case "code_block":
            endText()
            var lines = Substring(string(node, "literal"))
            while lines.hasSuffix("\n") { lines = lines.dropLast() }
            blocks.append(.code(lines: String(lines)))
        case "table":
            endText()
            blocks.append(.table(rows: children(of: node).map(row)))
        case "paragraph":
            paragraph(node)
        default:
            others.append(node)
        }
    }

    /// A paragraph is cut at each image of the assets folder: the image stands on its own.
    private func paragraph(_ node: Node) {
        var run: [Node] = []
        func endRun() {
            if !run.isEmpty { others.append(["type": "paragraph", "children": run]) }
            run = []
        }
        for child in children(of: node) {
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
            string($0, "type") == "paragraph" && plainText($0).allSatisfy(\.isWhitespace)
        }
        if !blank { blocks.append(.text(html: html(of: others))) }
        others = []
    }

    private func row(_ row: Node) -> ShownTableRow {
        ShownTableRow(
            header: row["header"] as? Bool ?? false,
            // What is in a cell is part of a line, and the HTML is made of whole blocks
            cells: children(of: row).map { cell in
                html(of: [["type": "paragraph", "children": cell["children"] ?? [Node]()]])
            })
    }

    /// The HTML of the blocks.
    private func html(of nodes: [Node]) -> String {
        let document = webImagesAsLinks(["type": "document", "children": nodes])
        guard let json = try? JSONSerialization.data(withJSONObject: document) else { return "" }
        return itemHtml(ast: String(decoding: json, as: UTF8.self), tagsDir: tagsDir)
    }
}

/// True when a part of its own kind is somewhere under the node.
private func holdsOwnPart(_ node: Node) -> Bool {
    children(of: node).contains {
        ["code_block", "table"].contains(string($0, "type")) || assetImage($0) != nil || holdsOwnPart($0)
    }
}

/// The image that the node is, when its file is in the assets folder. A page writes the
/// address as `../assets/picture.png`, from the URL of the page in the web app.
private func assetImage(_ node: Node) -> ShownImage? {
    guard string(node, "type") == imageType, let path = assetPath(string(node, "url")) else { return nil }
    let props = node["props"] as? Node
    // A side that is not a number above zero is not asked for, as in the web app
    func side(_ name: String) -> Int? {
        (props?[name] as? String).flatMap(Int.init).flatMap { $0 > 0 ? $0 : nil }
    }
    return ShownImage(path: path, alt: plainText(node), width: side("w"), height: side("h"))
}

/// The text under the node, without its decoration.
private func plainText(_ node: Node) -> String {
    string(node, "value") + children(of: node).map(plainText).joined()
}

/// The same tree, with a link in the place of each image of the web. The text of the link is
/// what stands for the image, or the address when nothing does.
private func webImagesAsLinks(_ node: Node) -> Node {
    let url = string(node, "url")
    if string(node, "type") == imageType, url.hasPrefix("https://") || url.hasPrefix("http://") {
        let alt = plainText(node)
        let text: Node = ["type": "text", "value": alt.isEmpty ? url : alt]
        return ["type": "link", "url": url, "children": [text]]
    }
    guard node["children"] is [Node] else { return node }
    var same = node
    same["children"] = children(of: node).map(webImagesAsLinks)
    return same
}

private func children(of node: Node) -> [Node] {
    node["children"] as? [Node] ?? []
}

private func string(_ node: Node, _ name: String) -> String {
    node[name] as? String ?? ""
}
