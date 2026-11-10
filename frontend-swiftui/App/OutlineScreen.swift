import PetaBorder
import PetaTypes
import SwiftUI

/// The items of the open page, one text field each.
struct OutlineScreen: View {
    @ObservedObject var shell: Shell

    var body: some View {
        List(shell.view.outline.rows, id: \.item.id) { row in
            ItemEditor(row: row, shell: shell)
        }
        .navigationTitle(shell.title)
        .frame(minWidth: 480, minHeight: 320)
    }
}

/// The text of one item. What is typed goes to the core, and the field shows what the core has.
private struct ItemEditor: View {
    let row: Row
    let shell: Shell
    @FocusState private var hasCursor: Bool

    var body: some View {
        TextField("", text: text)
            .textFieldStyle(.plain)
            .padding(.leading, CGFloat(row.indentPx))
            .focused($hasCursor)
            .onChange(of: hasCursor) { hasCursor in
                if hasCursor { shell.startEditing(row.item.id) }
            }
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
