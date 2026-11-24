import PetaBorder
import SwiftUI

/// The parts of an item, or of an entry of a list or a quote in it, from top to bottom.
struct Blocks: View {
    let blocks: [RichBlock]
    let links: Links
    let image: (String) -> URL?

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            ForEach(Array(blocks.enumerated()), id: \.offset) { _, block in
                switch block {
                case .text(let paragraphs):
                    Paragraphs(paragraphs: paragraphs, links: links)
                case .code(let lines, let start):
                    CodeBlock(lines: lines, start: start)
                case .table(let rows):
                    TableBlock(rows: rows, links: links)
                case .image(let shown):
                    ImageBlock(image: shown, file: image)
                case .entries(let entries, _):
                    // The entries of a list, each after its mark
                    VStack(alignment: .leading, spacing: 2) {
                        ForEach(Array(entries.enumerated()), id: \.offset) { _, entry in
                            HStack(alignment: .top, spacing: 8) {
                                Text(entry.mark)
                                Blocks(blocks: entry.blocks, links: links, image: image)
                            }
                        }
                    }
                case .quote(let inside, _):
                    Blocks(blocks: inside, links: links, image: image).modifier(Quoted(depth: 1))
                }
            }
        }
    }
}

/// What is quoted has a line down its left side, one for each quote it is inside.
private struct Quoted: ViewModifier {
    let depth: Int

    func body(content: Content) -> some View {
        content
            .padding(.leading, CGFloat(depth) * 12)
            .overlay(alignment: .leading) {
                HStack(spacing: 9) {
                    ForEach(0..<depth, id: \.self) { _ in Rectangle().fill(Color(.systemGray3)).frame(width: 3) }
                }
            }
            .padding(.vertical, depth > 0 ? 2 : 0)
    }
}

/// Text with the decoration of its letters and its links.
struct Paragraphs: View {
    let paragraphs: [RichParagraph]
    let links: Links

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            ForEach(Array(paragraphs.enumerated()), id: \.offset) { _, paragraph in
                if paragraph.rule {
                    Divider().padding(.vertical, 4)
                } else {
                    HStack(alignment: .firstTextBaseline, spacing: 6) {
                        if let mark = paragraph.mark { Text(mark).frame(minWidth: 16, alignment: .leading) }
                        Text(letters(paragraph)).letters(measured(paragraph), start: paragraph.start)
                    }
                    // A paragraph of an entry that is not its first one stands under the text
                    .padding(
                        .leading,
                        CGFloat(max(Int(paragraph.depth) - 1, 0)) * 22
                            + (paragraph.depth > 0 && paragraph.mark == nil ? 22 : 0)
                    )
                    .modifier(Quoted(depth: Int(paragraph.quoted)))
                }
            }
        }
        .environment(
            \.openURL,
            OpenURLAction { address in
                if let link = RichLinks.link(of: address) { links.open(link) }
                return .handled
            })
    }

    private func letters(_ paragraph: RichParagraph) -> AttributedString {
        let size = Letters.plain * Letters.larger(heading: Int(paragraph.heading))
        var all = AttributedString()
        for run in paragraph.runs {
            var part = AttributedString(run.text)
            let looks = run.looks
            part.font = Font(Self.font(run.looks, heading: Int(paragraph.heading)))
            if looks.struck { part.strikethroughStyle = .single }
            if looks.underlined { part.underlineStyle = .single }
            if looks.marked { part.backgroundColor = Color.yellow.opacity(0.45) }
            if looks.code || looks.hidden { part.backgroundColor = Color(.systemGray5) }
            if looks.raised { part.baselineOffset = size * 0.35 }
            if looks.lowered { part.baselineOffset = -size * 0.15 }
            if let link = run.link { part.link = RichLinks.url(of: link) }
            all += part
        }
        return all
    }

    /// The letters of the paragraph with their fonts alone: what a touch is measured on.
    private func measured(_ paragraph: RichParagraph) -> NSAttributedString {
        let all = NSMutableAttributedString()
        for run in paragraph.runs {
            all.append(
                NSAttributedString(
                    string: run.text, attributes: [.font: Self.font(run.looks, heading: Int(paragraph.heading))]))
        }
        return all
    }

    /// The font of the letters of a run. The same font draws them and measures a touch.
    private static func font(_ looks: Looks, heading: Int) -> UIFont {
        let size = Letters.plain * Letters.larger(heading: heading)
        let points = looks.raised || looks.lowered ? size * 0.75 : size
        let weight: UIFont.Weight = heading > 0 || looks.bold ? .bold : .regular
        let font =
            looks.code
            ? UIFont.monospacedSystemFont(ofSize: points, weight: weight)
            : UIFont.systemFont(ofSize: points, weight: weight)
        guard looks.italic,
            let italic = font.fontDescriptor.withSymbolicTraits(font.fontDescriptor.symbolicTraits.union(.traitItalic))
        else {
            return font
        }
        return UIFont(descriptor: italic, size: points)
    }
}

/// The lines of the code as they were written. A long line is scrolled sideways, not broken.
private struct CodeBlock: View {
    let lines: String
    let start: UInt32

    var body: some View {
        let font = UIFont.monospacedSystemFont(ofSize: Letters.plain * 0.85, weight: .regular)
        ScrollView(.horizontal) {
            Text(lines).font(Font(font))
                .letters(NSAttributedString(string: lines, attributes: [.font: font]), start: start, wraps: false)
                .padding(8)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color(.systemGray6))
        .padding(.vertical, 4)
    }
}

/// The cells of a table, each in a box, in columns of the same width.
private struct TableBlock: View {
    let rows: [RichTableRow]
    let links: Links

    var body: some View {
        Grid(horizontalSpacing: 0, verticalSpacing: 0) {
            ForEach(Array(rows.enumerated()), id: \.offset) { _, row in
                GridRow {
                    ForEach(Array(row.cells.enumerated()), id: \.offset) { _, cell in
                        Paragraphs(paragraphs: [cell], links: links)
                            .font(.subheadline)
                            .fontWeight(row.header ? .bold : nil)
                            .padding(6)
                            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                            .border(Color(.systemGray4), width: 0.5)
                    }
                }
            }
        }
        .padding(.vertical, 4)
    }
}
