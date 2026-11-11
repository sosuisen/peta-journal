import PetaBorder
import PetaTypes
import SwiftUI

/// The items of the open page, one text field each.
public struct OutlineScreen: View {
    @ObservedObject private var shell: Shell
    @StateObject private var cursors = Cursors()
    /// What is typed into the prompt on screen.
    @State private var typed = ""
    /// The search box is on screen.
    @State private var searching: Bool

    /// `searching` opens the search box from the start.
    public init(shell: Shell, searching: Bool = false) {
        self.shell = shell
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
            }
            .padding()
            // Without a page there is nothing to do but to search for one
            if searching || shell.url == nil {
                SearchBox(shell: shell) { searching = false }
                    .padding([.horizontal, .bottom])
            }
            if shell.url == nil {
                Text("Search for a page to show here").foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                rows
            }
        }
        .background(SelectionKeys(shell: shell, selecting: shell.view.outline.selectionMode))
        .background(Color(nsColor: .windowBackgroundColor))
        .navigationTitle(shell.title)
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
            ItemEditor(row: row, shell: shell, cursors: cursors)
                .listRowSeparator(.hidden)
                .listRowBackground(row.isSelected ? Color.accentColor.opacity(0.25) : nil)
        }
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
            Text(mark).foregroundStyle(.secondary)
            ItemText(
                item: row.item, isFirst: row.index == 0, request: shell.view.outline.focusRequest,
                order: shell.focus, shell: shell, cursors: cursors)
        }
        .padding(.leading, CGFloat(row.indentPx))
    }
}
