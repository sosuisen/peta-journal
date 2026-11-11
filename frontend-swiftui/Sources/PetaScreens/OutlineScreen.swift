import PetaBorder
import PetaTypes
import SwiftUI

/// The items of the open page, one text field each.
public struct OutlineScreen: View {
    @ObservedObject private var shell: Shell
    @ObservedObject private var look: StickyLook
    @StateObject private var cursors = Cursors()
    /// What is typed into the prompt on screen.
    @State private var typed = ""
    /// The search box is on screen.
    @State private var searching: Bool

    /// Opens another window, when the screen has a button for that.
    private let onAdd: (() -> Void)?

    /// `searching` opens the search box from the start. With `onAdd` the screen has a button
    /// that opens another window.
    @MainActor
    public init(
        shell: Shell, look: StickyLook? = nil, searching: Bool = false, onAdd: (() -> Void)? = nil
    ) {
        self.shell = shell
        self.look = look ?? StickyLook()
        self.onAdd = onAdd
        _searching = State(initialValue: searching)
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // An empty window says nothing: the page and the state of the server are always shown
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(shell.title.isEmpty ? "No page" : shell.title).font(.title2)
                    Text(status).font(.caption).foregroundStyle(.secondary)
                }
                Spacer()
                Button {
                    searching.toggle()
                } label: {
                    Image(systemName: "magnifyingglass")
                }
                .keyboardShortcut("f")
                .help("Search pages")
                Button {
                    look.alwaysOnTop.toggle()
                } label: {
                    Image(systemName: look.alwaysOnTop ? "pin.fill" : "pin")
                }
                .help(look.alwaysOnTop ? "Do not keep on top" : "Keep on top of other windows")
                Menu {
                    Button("Zoom In") { look.zoomIn() }.keyboardShortcut("+")
                    Button("Zoom Out") { look.zoomOut() }.keyboardShortcut("-")
                    Button("Actual Size") { look.resetZoom() }.keyboardShortcut("0")
                } label: {
                    Image(systemName: "textformat.size")
                }
                .menuStyle(.borderlessButton)
                .fixedSize()
                .help("Zoom: \(Int((look.zoom * 100).rounded()))%")
                Menu {
                    ForEach(StickyLook.colors, id: \.hex) { color in
                        Button(color.name) { look.color = color.hex }
                    }
                } label: {
                    Image(systemName: "paintpalette")
                }
                .menuStyle(.borderlessButton)
                .fixedSize()
                .help("Color")
                if let onAdd {
                    Button(action: onAdd) {
                        Image(systemName: "plus")
                    }
                    .keyboardShortcut("n")
                    .help("New sticky")
                }
            }
            .padding()
            // Without a page there is nothing to do but to search for one
            if searching || shell.url == nil {
                SearchBox(shell: shell) { searching = false }
                    .padding([.horizontal, .bottom])
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
        .frame(minWidth: 480, minHeight: 320)
        .onChange(of: shell.question) { _ in typed = "" }
        // The page that was searched for is here
        .onChange(of: shell.title) { _ in searching = false }
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
            case .prompt:
                TextField("", text: $typed)
                Button("OK") { shell.answer(text: typed) }.keyboardShortcut(.defaultAction)
                Button("Cancel", role: .cancel) { shell.answer(text: nil) }
            }
        }
    }

    private var rows: some View {
        List(shell.view.outline.rows, id: \.item.id) { row in
            ItemEditor(row: row, shell: shell, cursors: cursors, zoom: look.zoom)
                .listRowSeparator(.hidden)
                .listRowBackground(row.isSelected ? Color.accentColor.opacity(0.25) : Color.clear)
        }
        // The paper shows through the list
        .scrollContentBackground(.hidden)
    }

    private var status: String {
        if shell.url == nil { return "" }
        let items = shell.view.outline.rows.count
        // Not a switch: the optimizing compiler of Swift 6.4.2 crashes on one over this enum
        let connection = shell.view.connection
        if connection == .connected { return "Items: \(items)" }
        if connection == .connecting { return "Connecting to the journal server…" }
        return "No connection to the journal server"
    }
}

/// One item: its bullet and its text, as far to the right as its level says.
private struct ItemEditor: View {
    let row: Row
    @ObservedObject var shell: Shell
    let cursors: Cursors
    /// How large the item is shown: 1 is the usual size.
    let zoom: Double

    /// What stands before the text: the state of a task, or whether children are hidden.
    private var mark: String {
        if let task = row.task {
            if task == .done { return "☑" }
            return task == .running ? "◐" : "☐"
        }
        return row.isCollapsed ? "▸" : "•"
    }

    var body: some View {
        HStack(alignment: .top) {
            // Without the bullet an item with no text cannot be seen
            Text(mark).foregroundStyle(.secondary).font(.system(size: NSFont.systemFontSize * zoom))
            ItemText(
                item: row.item, isFirst: row.index == 0, request: shell.view.outline.focusRequest,
                order: shell.focus, shell: shell, cursors: cursors, zoom: zoom)
        }
        .padding(.leading, CGFloat(row.indentPx) * zoom)
    }
}
