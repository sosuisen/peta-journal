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
    /// Wikilinks and hashtags name a page by its path, not by an address. As links of the text
    /// they get this address, to tell them from links that lead out of the journal.
    private static let pages = URL(string: "peta-page://page/")!

    /// The text of the item. An item whose AST cannot be read shows its body as it is: nothing
    /// that was written is left out.
    public static func of(_ item: Item, zoom: Double = 1) -> NSAttributedString {
        guard let html = try? itemHtml(ast: item.ast) else { return plain(item.body, zoom: zoom) }
        return text(ofHTML: html, zoom: zoom)
    }

    /// The path of the page that a link of the text names, when it is a wikilink or a hashtag.
    public static func page(_ link: URL) -> String? {
        guard link.scheme == pages.scheme, link.host == pages.host else { return nil }
        return String(link.path.dropFirst())
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
