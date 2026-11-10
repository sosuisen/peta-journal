import PetaBorder
import PetaTypes
import SwiftUI

/// The items of the open page, one text field each.
public struct OutlineScreen: View {
    @ObservedObject private var shell: Shell
    @StateObject private var cursors = Cursors()
    /// What is typed into the prompt on screen.
    @State private var typed = ""

    public init(shell: Shell) {
        self.shell = shell
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // An empty window says nothing: the page and the state of the server are always shown
            VStack(alignment: .leading, spacing: 4) {
                Text(shell.title.isEmpty ? "No page" : shell.title).font(.title2)
                Text(status).font(.caption).foregroundStyle(.secondary)
            }
            .padding()
            List(shell.view.outline.rows, id: \.item.id) { row in
                ItemEditor(row: row, shell: shell, cursors: cursors)
                    .listRowSeparator(.hidden)
            }
        }
        .background(Color(nsColor: .windowBackgroundColor))
        .navigationTitle(shell.title)
        .frame(minWidth: 480, minHeight: 320)
        .onChange(of: shell.question) { _ in typed = "" }
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

    private var status: String {
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

    var body: some View {
        HStack(alignment: .top) {
            // Without the bullet an item with no text cannot be seen
            Text("•").foregroundStyle(.secondary)
            ItemText(
                item: row.item, isFirst: row.index == 0, request: shell.view.outline.focusRequest,
                order: shell.focus, shell: shell, cursors: cursors)
        }
        .padding(.leading, CGFloat(row.indentPx))
    }
}
