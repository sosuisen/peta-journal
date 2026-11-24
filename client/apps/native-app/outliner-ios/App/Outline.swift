import PetaBorder
import PetaTypes
import SwiftUI

/// The sizes of the letters of an item.
enum Letters {
    /// The size of plain text, which follows the size the user chose for the device.
    static var plain: CGFloat { UIFont.preferredFont(forTextStyle: .body).pointSize }

    /// `count` ems of plain text, as the core gives an indent (#795).
    static func ems(_ count: Float) -> CGFloat {
        plain * CGFloat(count)
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

    var body: some View {
        let view = shell.view
        let focus = view.outline.focusRequest
        ScrollViewReader { list in
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 0) {
                    Text(title(view)).font(.title2).padding(.vertical, 8)
                    ForEach(view.outline.rows, id: \.item.id) { row in
                        OutlineRow(row: row, shell: shell)
                            .id(row.item.id)
                    }
                }
                .padding(.horizontal, 12)
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
            // The field of the edited item left the screen with the keyboard, as when the
            // screen is turned and the item goes out of what is seen. The keyboard keeps its
            // height then, and nothing else brings the item back (#792)
            .onReceive(NotificationCenter.default.publisher(for: .editedFieldLeft)) { _ in
                guard let edited = shell.view.outline.rows.first(where: \.isEditing)?.item.id else { return }
                list.scrollTo(edited)
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
    let shell: Shell
    /// Where the texts of the item stand, for a touch to find its letter.
    @State private var targets: [LetterTarget] = []
    /// Where the images of the item stand, for a touch to open one large.
    @State private var images: [ImageTarget] = []
    /// The path of the image that a touch opened large.
    @State private var opened: String?

    /// How wide and how high the bullet of an item is.
    private static let bullet: CGFloat = 7

    /// How wide the disc behind the bullet of a folded item is, and how faint. The web draws a
    /// dot of 5px on a disc of 13px, in `#888` and `#ddd`: the disc here keeps that size to the
    /// bullet, and that faintness.
    private static let folded: CGFloat = 18
    private static let foldedFaintness = 0.13

    var body: some View {
        let id = row.item.id
        let blocks = row.isEditing ? [] : row.bodyModel.blocks
        // The bullet and the mark stand beside the middle of the first line. An item that
        // starts with a heading has a higher first line, as the core says (#795). The editor
        // has letters of plain size
        let first = Letters.line(Letters.plain * (row.isEditing ? 1 : CGFloat(row.firstLineScale)))
        VStack(alignment: .leading, spacing: 0) {
            if let date = row.dateHeader {
                // As large as the date of today at the top: each is the title of a day
                Text(date).font(.title2).padding(.top, 12)
            }
            HStack(alignment: .top, spacing: 0) {
                ZStack {
                    // The bullet is drawn, not a letter: the place of a letter in its line
                    // differs from font to font
                    if row.showBullet {
                        if row.isCollapsed {
                            // A folded item has a large faint disc behind its bullet, as on the web
                            Circle().frame(width: OutlineRow.bullet, height: OutlineRow.bullet)
                                .background {
                                    Circle().fill(Color.primary.opacity(OutlineRow.foldedFaintness))
                                        .frame(width: OutlineRow.folded, height: OutlineRow.folded)
                                }
                                .accessibilityElement()
                                .accessibilityLabel("Folded")
                                .accessibilityIdentifier("folded")
                        } else {
                            Circle().frame(width: OutlineRow.bullet, height: OutlineRow.bullet)
                                .accessibilityIdentifier("bullet")
                        }
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
                    Blocks(blocks: blocks, links: Links(shell: shell), image: shell.image)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .coordinateSpace(name: Touches.space)
                        .onPreferenceChange(LetterTargets.self) { targets = $0 }
                        .onPreferenceChange(ImageTargets.self) { images = $0 }
                        .environment(\.openedImage, $opened)
                        // A touch on a link or an image opens it, and any other touch edits the
                        // item with the cursor where it was touched
                        .contentShape(Rectangle())
                        .onTapGesture(coordinateSpace: .named(Touches.space)) { point in
                            if let image = Touches.image(at: point, in: images) {
                                opened = image
                            } else {
                                shell.edit(id, letter: Touches.letter(at: point, in: targets))
                            }
                        }
                }
            }
            .padding(.leading, CGFloat(row.indentPx))
            .padding(.vertical, 3)
            // A selected item is coloured behind, so that the user sees what the tools act on
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(row.isSelected ? Color.accentColor.opacity(0.18) : Color.clear)
            .accessibilityElement(children: .contain)
            .accessibilityIdentifier(row.isSelected ? "selected" : "")
        }
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

    /// The user touched a link of the text. The core decides how it opens (ADR 083).
    @MainActor
    func open(_ link: RichLink) {
        shell.follow(link)
    }
}
