import AppKit
import PetaCore
import PetaTypes

/// The text of an item as it is shown: with its decoration, not as Markdown (ADR 025).
///
/// The shell does not read the AST. Rust makes HTML of it, with `ast_to_html` of
/// flatmarkdown-ast2html, which takes every kind of node. macOS turns the HTML into text with
/// attributes. That has to happen on the main thread, and outside the drawing of SwiftUI:
/// macOS runs work of its own in the middle of it.
@MainActor
public enum ShownText {
    /// Wikilinks and hashtags name a page or a tag, and have no address. As links of the text
    /// they get an address of this kind, to tell them from links that lead out of the journal:
    /// the host says whether it is a page or a tag.
    private static let pages = URL(string: "peta-link://page/")!
    private static let tags = "tag"

    /// The text of the item. An item whose AST cannot be read shows its body as it is: nothing
    /// that was written is left out.
    public static func of(_ item: Item, zoom: Double = 1) -> NSAttributedString {
        // The AST of what was just typed comes from the server a moment later
        if item.ast.isEmpty { return plain(item.body, zoom: zoom) }
        guard let html = try? itemHtml(ast: item.ast) else { return plain(item.body, zoom: zoom) }
        return text(ofHTML: html, zoom: zoom)
    }

    /// What a link of the shown text leads to. Nothing for a link that stays in the item, as
    /// the one of a footnote.
    public static func target(of link: URL) -> LinkTarget? {
        guard link.scheme == pages.scheme else { return .out(link) }
        let name = String(link.path.dropFirst())
        if name.isEmpty { return nil }
        return link.host == tags ? .tag(name) : .page(name)
    }

    static func text(ofHTML html: String, zoom: Double) -> NSAttributedString {
        if html.isEmpty { return NSAttributedString() }
        let size = NSFont.systemFontSize * zoom
        let page = "<meta charset=\"utf-8\"><style>\(css(size: size))</style>" + withWhatMacOSDrops(html)
        let read = NSMutableAttributedString(
            html: Data(page.utf8),
            options: [.characterEncoding: String.Encoding.utf8.rawValue, .baseURL: pages],
            documentAttributes: nil)
        // macOS reads any text as HTML, so this is not expected to happen
        guard let text = read else { return plain(html, zoom: zoom) }
        // Every block ends its line, the last one too
        while text.string.hasSuffix("\n") {
            text.deleteCharacters(in: NSRange(location: text.length - 1, length: 1))
        }
        return text
    }

    private static func plain(_ text: String, zoom: Double) -> NSAttributedString {
        NSAttributedString(
            string: text,
            attributes: [.font: NSFont.systemFont(ofSize: NSFont.systemFontSize * zoom)])
    }

    /// How the text looks. This is the look of this shell: it is not kept the same as the
    /// stylesheet of the web frontend. macOS takes what CSS says about the letters, and drops
    /// what it says about boxes (borders, padding, round corners).
    private static func css(size: Double) -> String {
        """
        body { font: \(size)px -apple-system; }
        p, ul, ol, blockquote, pre, table, h1, h2, h3, h4, h5, h6 { margin: 0; }
        h1 { font-size: 1.6em; } h2 { font-size: 1.4em; } h3 { font-size: 1.2em; }
        h4, h5, h6 { font-size: 1em; }
        code, pre { font-family: Menlo; }
        a { color: #0a58ca; }
        mark { background: #fff3a3; }
        blockquote { color: #555555; }
        .alert-title { font-weight: 600; }
        .spoiler { background: #dddddd; }
        """
    }

    /// The HTML with text in the place of what macOS leaves out when it reads HTML: the
    /// checkbox of a task list, the line between two parts, and an image. The HTML of
    /// `ast_to_html` has one form for each, so the text is put in by its form.
    private static func withWhatMacOSDrops(_ html: String) -> String {
        var kept = html
        // Not left out by macOS, but a hashtag would look like a wikilink: both have only a name
        kept = kept.replacingOccurrences(
            of: "<a href=\"([^\"]*)\" class=\"hashtag\">", with: "<a href=\"//\(tags)/$1\">",
            options: .regularExpression)
        kept = kept.replacingOccurrences(of: "<input type=\"checkbox\" checked disabled>", with: "☑ ")
        kept = kept.replacingOccurrences(of: "<input type=\"checkbox\" disabled>", with: "☐ ")
        kept = kept.replacingOccurrences(of: "<hr />", with: "<p>────────</p>")
        // An image is shown as the words for it, and without any as its file. The image itself
        // is not shown yet
        kept = kept.replacingOccurrences(
            of: "<img src=\"([^\"]*)\" alt=\"\"[^>]*/>", with: "[$1]", options: .regularExpression)
        kept = kept.replacingOccurrences(
            of: "<img src=\"[^\"]*\" alt=\"([^\"]*)\"[^>]*/>", with: "[$1]", options: .regularExpression)
        return kept
    }
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

/// The shown texts of a screen that were made already. To make one takes two or three
/// milliseconds, and it must not be done while SwiftUI draws: a row that has no text yet asks
/// for it here, and is told when it is there.
@MainActor
public final class ShownTexts: ObservableObject {
    private struct Made {
        let ast: String
        let body: String
        let zoom: Double
        let text: NSAttributedString
    }

    private var made: [String: Made] = [:]
    /// Counts up whenever the rows are to be drawn again. A row is handed this number: SwiftUI
    /// draws a row again only when what it is given changes.
    public private(set) var changes = 0
    private var wanted: [String: (item: Item, zoom: Double)] = [:]

    public init() {}

    /// The text of the item as it is now, when it was made already.
    public func text(of item: Item, zoom: Double) -> NSAttributedString? {
        guard let made = made[item.id], made.ast == item.ast, made.body == item.body, made.zoom == zoom else {
            return nil
        }
        return made.text
    }

    /// Makes the text of the item soon, outside the drawing, and tells that it changed.
    public func want(_ item: Item, zoom: Double) {
        let first = wanted.isEmpty
        wanted[item.id] = (item, zoom)
        guard first else { return }
        // All the rows of one drawing are made together, and told once
        DispatchQueue.main.async { [self] in
            objectWillChange.send()
            changes += 1
            for (id, asked) in wanted {
                made[id] = Made(
                    ast: asked.item.ast, body: asked.item.body, zoom: asked.zoom,
                    text: ShownText.of(asked.item, zoom: asked.zoom))
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
