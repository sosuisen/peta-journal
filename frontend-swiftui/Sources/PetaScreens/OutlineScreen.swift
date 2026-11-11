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

    /// `searching` opens the search box from the start.
    @MainActor
    public init(shell: Shell, look: StickyLook? = nil, searching: Bool = false) {
        self.shell = shell
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
        .onChange(of: shell.question) { _ in typed = "" }
        // The page that was searched for is here
        .onChange(of: shell.title) { _ in look.searching = false }
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
    /// How large the item is shown: 1 is the usual size.
    let zoom: Double

    var body: some View {
        HStack(alignment: .top) {
            // Without the bullet an item with no text cannot be seen
            ForEach(row.marks, id: \.self) { mark in
                Text(mark).foregroundStyle(.secondary).font(.system(size: NSFont.systemFontSize * zoom))
            }
            ItemText(
                item: row.item, isFirst: row.index == 0, request: shell.view.outline.focusRequest,
                order: shell.focus, shell: shell, cursors: cursors, zoom: zoom)
        }
        .padding(.leading, CGFloat(row.indentPx) * zoom)
    }
}
