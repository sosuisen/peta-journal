import PetaBorder
import PetaTypes
import SwiftUI

/// The items of the open page, one text field each.
public struct OutlineScreen: View {
    @ObservedObject private var shell: Shell
    /// The item with the cursor in its text.
    @FocusState private var cursor: String?
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
                ItemEditor(row: row, shell: shell, cursor: $cursor)
            }
        }
        .background(Color(nsColor: .windowBackgroundColor))
        .navigationTitle(shell.title)
        .frame(minWidth: 480, minHeight: 320)
        .onChange(of: shell.focus) { order in
            cursor = order?.itemId ?? shell.view.outline.rows.first?.item.id
        }
        .onChange(of: cursor) { itemId in
            if let itemId { shell.startEditing(itemId) }
        }
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

/// The text of one item. What is typed goes to the core, and the field shows what the core has.
private struct ItemEditor: View {
    let row: Row
    let shell: Shell
    let cursor: FocusState<String?>.Binding

    var body: some View {
        HStack(alignment: .firstTextBaseline) {
            // Without the bullet an item with no text cannot be seen
            Text("•").foregroundStyle(.secondary)
            TextField("", text: text)
                .textFieldStyle(.plain)
                .focused(cursor, equals: row.item.id)
        }
        .padding(.leading, CGFloat(row.indentPx))
    }

    private var text: Binding<String> {
        Binding(
            get: { row.item.body },
            set: { typed in
                // The field also sets its text when nothing was typed, such as on focus
                if typed != row.item.body { shell.typed(typed, into: row.item.id) }
            }
        )
    }
}
