import PetaBorder
import PetaTypes
import SwiftUI

/// The sizes of the letters of an item.
enum Letters {
    /// The size of plain text, which follows the size the user chose for the device.
    static var plain: CGFloat { UIFont.preferredFont(forTextStyle: .body).pointSize }

    /// How many times the size of plain text the letters of a heading are.
    static func larger(heading: Int) -> CGFloat {
        switch heading {
        case 1: return 1.6
        case 2: return 1.4
        case 3: return 1.2
        case 4: return 1.1
        default: return 1
        }
    }

    /// How high a line of letters of the size is.
    static func line(_ size: CGFloat) -> CGFloat {
        UIFont.systemFont(ofSize: size).lineHeight
    }
}

/// The page that is open: its title, and its items one under the other.
struct Outline: View {
    @ObservedObject var shell: Shell
    /// What the keyboard and the tools cover at the bottom of the page.
    let covered: CGFloat
    let onFile: (URL) -> Void

    var body: some View {
        let view = shell.view
        let focus = view.outline.focusRequest
        ScrollViewReader { list in
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 0) {
                    Text(title(view)).font(.title2).padding(.vertical, 8)
                    ForEach(view.outline.rows, id: \.item.id) { row in
                        OutlineRow(row: row, tagsDir: view.config.tagsDir, shell: shell, onFile: onFile)
                            .id(row.item.id)
                    }
                }
                .padding(.horizontal, 12)
                // Room for the callout of the sync under the last item
                .padding(.bottom, 60)
            }
            // A drag of the page puts the keyboard away, and so ends the edit
            .scrollDismissesKeyboard(.interactively)
            // The item that the core moved the edit to may be off the screen, as a new item
            // at the end of a long page
            .onChange(of: focus?.seq) {
                if let focus { list.scrollTo(focus.itemId) }
            }
            // The keyboard and the tools cover the bottom of the page. The item that is
            // edited may be under them then, and it is brought to what is seen
            .onChange(of: covered) {
                guard let edited = shell.view.outline.rows.first(where: \.isEditing)?.item.id else { return }
                withAnimation { list.scrollTo(edited) }
            }
        }
    }

    private func title(_ view: ViewModel) -> String {
        switch view.screen {
        case .page(let page): return page?.title ?? ""
        case .journal(let journal): return journal.displayDate
        default: return ""
        }
    }
}

/// One item of the outline: its bullet, the mark of its task, and its text. The item that is
/// edited shows its Markdown in a field (ADR 031).
struct OutlineRow: View {
    let row: Row
    let tagsDir: String
    let shell: Shell
    let onFile: (URL) -> Void

    /// How wide and how high the bullet of an item is.
    private static let bullet: CGFloat = 7

    var body: some View {
        let id = row.item.id
        let blocks = row.isEditing ? [] : shownBlocks(ast: row.ast, tagsDir: tagsDir)
        // The bullet and the mark stand beside the middle of the first line. An item that
        // starts with a heading has a higher first line
        let first = Letters.line(Letters.plain * firstLarger(blocks))
        VStack(alignment: .leading, spacing: 0) {
            if let date = row.dateHeader {
                Text(date).font(.headline).padding(.top, 12)
            }
            HStack(alignment: .top, spacing: 0) {
                ZStack {
                    // The bullet is drawn, not a letter: the place of a letter in its line
                    // differs from font to font
                    if row.showBullet {
                        Circle().frame(width: OutlineRow.bullet, height: OutlineRow.bullet)
                            .accessibilityIdentifier("bullet")
                    }
                }
                .frame(width: 22, height: first, alignment: .leading)
                if let task = row.task {
                    TaskMark(state: task, height: first) { shell.cycleTask(id) }
                }
                if row.isEditing {
                    ItemEditor(row: row, shell: shell)
                } else if row.item.body.isEmpty {
                    // An item without text is as high as a line, to be touched and written
                    // into: a new journal has one such item and nothing else
                    Color.clear
                        .frame(maxWidth: .infinity, minHeight: first)
                        .contentShape(Rectangle())
                        .onTapGesture { shell.edit(id) }
                        .accessibilityElement()
                        .accessibilityLabel("Empty item")
                        .accessibilityAddTraits(.isButton)
                } else {
                    Blocks(blocks: blocks, links: Links(shell: shell, onFile: onFile), image: shell.image)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        // A touch on a link or an image opens it, and any other touch edits the item
                        .contentShape(Rectangle())
                        .onTapGesture { shell.edit(id) }
                }
            }
            .padding(.leading, CGFloat(row.indentPx))
            .padding(.vertical, 3)
        }
    }

    /// How many times larger than plain text the letters of the first line are.
    private func firstLarger(_ blocks: [ShownBlock]) -> CGFloat {
        guard case .text(let paragraphs) = blocks.first, let paragraph = paragraphs.first else { return 1 }
        return Letters.larger(heading: paragraph.heading)
    }
}

/// Where a task stands, as a checkbox. A touch moves the task on: not started, running, done,
/// and then no task.
private struct TaskMark: View {
    let state: TaskState
    /// As high as the first line of the text, so the box is beside it.
    let height: CGFloat
    let onTouch: () -> Void

    var body: some View {
        Button(action: onTouch) {
            Image(systemName: face).font(.system(size: Letters.plain * 1.1))
        }
        .buttonStyle(.plain)
        .foregroundStyle(state == .notStarted ? Color.secondary : Color.accentColor)
        .frame(width: 28, height: height, alignment: .leading)
        .accessibilityLabel(name)
    }

    private var face: String {
        switch state {
        case .notStarted: return "square"
        case .running: return "minus.square.fill"
        case .done: return "checkmark.square.fill"
        }
    }

    private var name: String {
        switch state {
        case .notStarted: return "Task not started"
        case .running: return "Task running"
        case .done: return "Task done"
        }
    }
}

/// Where the links of an item lead.
struct Links {
    let shell: Shell
    let onFile: (URL) -> Void

    /// The user touched a link of the text. `address` is made whole against `ShownText.base`.
    @MainActor
    func open(_ address: URL) {
        switch LinkTarget(of: address) {
        case .web(let url): UIApplication.shared.open(url)
        case .page(let url): shell.follow(url)
        case .file(let path): if let file = shell.attached(path) { onFile(file) }
        // Any other link, such as one to a file of the PC, opens nothing
        case nil: break
        }
    }
}
