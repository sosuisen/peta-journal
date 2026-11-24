import PetaBorder
import PetaTypes
import SwiftUI

/// The items of the open page, one text field each.
public struct OutlineScreen: View {
    @ObservedObject private var shell: Shell
    @ObservedObject private var look: StickyLook
    @StateObject private var cursors = Cursors()
    @StateObject private var texts: ShownTexts
    @StateObject private var seen = SeenRows()

    /// `searching` opens the search box from the start.
    @MainActor
    public init(shell: Shell, look: StickyLook? = nil, searching: Bool = false) {
        self.shell = shell
        _texts = StateObject(wrappedValue: ShownTexts(shell: shell))
        self.look = look ?? StickyLook()
        if searching { self.look.searching = true }
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Items that do not come need a reason. The name of the page is in the title bar
            if !status.isEmpty {
                Text(status).font(.caption).foregroundStyle(.secondary).padding([.horizontal, .bottom])
            }
            // Without a page there is nothing to do but to search for one
            if look.searching || shell.url == nil {
                SearchBox(shell: shell) { look.searching = false }
                    .padding([.horizontal, .bottom])
                    // The title bar is right above
                    .padding(.top, 6)
                    // What was found comes before the items: they share the window otherwise,
                    // and only a few entries are seen
                    .layoutPriority(1)
            }
            if shell.url == nil {
                Text("Search for a page to show here").foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                rows
            }
        }
        .background(SelectionKeys(shell: shell, selecting: shell.view.outline.selectionMode))
        .background(Color(nsColor: StickyLook.color(of: look.color)))
        // The name of the window. A window without a name is hard to find among the others
        .navigationTitle(shell.title.isEmpty ? "PetaStickies" : shell.title)
        // The page that was searched for is here. Not by the title: that of a journal changes
        // when it is scrolled
        .onChange(of: shell.url) { _ in look.searching = false }
        .alert(
            shell.question?.message ?? "",
            isPresented: Binding(get: { shell.question != nil }, set: { _ in }),
            presenting: shell.question
        ) { question in
            switch question.kind {
            case .confirm:
                Button("OK") { shell.answer(yes: true) }.keyboardShortcut(.defaultAction)
                Button("Cancel", role: .cancel) { shell.answer(yes: false) }
            case .confirmDestructive:
                // No key says yes here: Return and Escape both cancel
                Button("OK", role: .destructive) { shell.answer(yes: true) }
                Button("Cancel", role: .cancel) { shell.answer(yes: false) }
                    .keyboardShortcut(.defaultAction)
            case .notice:
                Button("OK") { shell.answer(yes: true) }.keyboardShortcut(.defaultAction)
            }
        }
    }

    private var rows: some View {
        List(shell.view.outline.rows, id: \.item.id) { row in
            ItemEditor(row: row, shell: shell, cursors: cursors, texts: texts, zoom: look.zoom)
                .listRowSeparator(.hidden)
                .listRowBackground(row.isSelected ? Color.accentColor.opacity(0.25) : Color.clear)
                // A journal tells the day at the top of the window by the rows that are drawn
                .onAppear { seen.changed(row.item.id, shown: true, in: shell) }
                .onDisappear { seen.changed(row.item.id, shown: false, in: shell) }
        }
        // The paper shows through the list
        .scrollContentBackground(.hidden)
    }

    /// What is wrong with the server, or nothing.
    private var status: String {
        if shell.url == nil { return "" }
        // Not a switch: the optimizing compiler of Swift 6.4.2 crashes on one over this enum
        let connection = shell.view.connection
        if connection == .connected { return "" }
        if connection == .connecting { return "Connecting to the journal server…" }
        return "No connection to the journal server"
    }
}

/// One item: its bullet and its text, as far to the right as its level says.
private struct ItemEditor: View {
    let row: Row
    @ObservedObject var shell: Shell
    let cursors: Cursors
    /// The row is drawn again when the text with its decoration was made.
    @ObservedObject var texts: ShownTexts
    /// How large the item is shown: 1 is the usual size.
    let zoom: Double

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // The first item of a day of a journal, below another day: a line between the
            // days, and the date of this one
            if let date = row.dateHeader {
                Divider()
                Text(date).font(.system(size: NSFont.systemFontSize * 1.3 * zoom, weight: .bold))
                    .padding(.vertical, 8)
            }
            text
        }
        .padding(.leading, CGFloat(row.indentPx) * zoom)
    }

    private var text: some View {
        HStack(alignment: .top) {
            // Without the bullet an item with no text cannot be seen
            ForEach(Array(row.marks.enumerated()), id: \.offset) { place, mark in
                let size = NSFont.systemFontSize * zoom
                let line = NSLayoutManager().defaultLineHeight(for: .systemFont(ofSize: size))
                // The first line of a heading is higher. The marks keep their size and stand
                // in the middle of it, as in the web frontend
                let first = NSLayoutManager().defaultLineHeight(
                    for: .boldSystemFont(ofSize: size * row.firstLineScale))
                if place == 0 {
                    Bullet(mark: mark, line: line, first: first)
                } else {
                    // The mark after the bullet is the state of a task: a click steps it
                    TaskBox(mark: mark, line: line, first: first)
                        .onTapGesture { shell.clickedTask(of: row.item.id) }
                }
            }
            ItemText(
                item: row.item, rich: row.bodyModel, isEditing: row.isEditing,
                shown: texts.text(of: row.item.id, rich: row.bodyModel, zoom: zoom),
                texts: texts, changes: texts.changes, heading: row.editedHeading, isFirst: row.position == 0,
                request: shell.view.outline.focusRequest, order: shell.view.outline.focusOrder, shell: shell,
                cursors: cursors,
                zoom: zoom)
        }
    }
}

/// The bullet of an item: a dot, with a pale circle around it while its children are hidden.
/// It is drawn here as the web frontend draws it, and not written as a letter: a letter does
/// not stand in the middle of its line.
private struct Bullet: View {
    let mark: String
    /// How high a line of the usual text is: the bullet is as large as that says.
    let line: CGFloat
    /// How high the first line of the item is. The bullet stands in the middle of it.
    let first: CGFloat

    var body: some View {
        let side = (line * 0.85).rounded()
        ZStack {
            if mark == "◉" {
                Circle().opacity(0.3)
            }
            Circle().frame(width: side * 0.42, height: side * 0.42)
        }
        .foregroundStyle(.secondary)
        .frame(width: side, height: side)
        .frame(height: max(line, first))
    }
}

/// The box that shows the state of a task: empty, with a line while the task runs, or checked.
/// It is drawn here and not written as a letter: the three letters come from different fonts
/// and are of different sizes, and none stands in the middle of its line.
private struct TaskBox: View {
    let mark: String
    /// How high a line of the usual text is: the box is as large as that says.
    let line: CGFloat
    /// How high the first line of the item is. The box stands in the middle of it.
    let first: CGFloat

    var body: some View {
        // Half as large again as the letter was
        let side = (line * 0.85).rounded()
        let stroke = max(1, side / 11)
        ZStack {
            RoundedRectangle(cornerRadius: side / 6).stroke(lineWidth: stroke)
            Path { path in
                if mark == "⊟" {
                    path.move(to: CGPoint(x: side * 0.25, y: side * 0.5))
                    path.addLine(to: CGPoint(x: side * 0.75, y: side * 0.5))
                } else if mark == "☑" {
                    path.move(to: CGPoint(x: side * 0.22, y: side * 0.52))
                    path.addLine(to: CGPoint(x: side * 0.42, y: side * 0.72))
                    path.addLine(to: CGPoint(x: side * 0.78, y: side * 0.28))
                }
            }
            .stroke(style: StrokeStyle(lineWidth: stroke * 1.4, lineCap: .round, lineJoin: .round))
        }
        .foregroundStyle(.secondary)
        .frame(width: side, height: side)
        .frame(height: max(line, first))
        .contentShape(Rectangle())
    }
}

/// The rows that the list has drawn. SwiftUI tells of each row when it comes and when it
/// goes; the shell is told once of all that changed in one turn of the main queue.
@MainActor
private final class SeenRows: ObservableObject {
    private var ids: Set<String> = []
    private var waits = false
    private var told: ClosedRange<Int>?

    func changed(_ id: String, shown: Bool, in shell: Shell) {
        if shown { ids.insert(id) } else { ids.remove(id) }
        guard !waits else { return }
        waits = true
        DispatchQueue.main.async { [self] in
            waits = false
            // Only a journal does something with it, and every time costs a view model
            guard case .journal = shell.view.screen else { return }
            let places = shell.view.outline.rows.filter { ids.contains($0.item.id) }.map { Int($0.position) }
            guard let first = places.min(), let last = places.max(), told != first...last else { return }
            told = first...last
            shell.scrolled(top: first, shown: first...last)
        }
    }
}
