import PetaBorder
import SwiftUI

/// What changes the item that is edited, in a row above the keyboard (ADR 031). A phone has
/// no Tab key and no key with Alt. A touch on a tool leaves the cursor in the field.
struct Tools: View {
    let itemId: String
    let shell: Shell

    /// How high the row of the tools is.
    static let height: CGFloat = 44

    var body: some View {
        HStack(spacing: 0) {
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 0) {
                    tool("Outdent", "decrease.indent") { shell.outdent(itemId) }
                    tool("Indent", "increase.indent") { shell.indent(itemId) }
                    tool("Move up", "arrow.up") { shell.moveUp(itemId) }
                    tool("Move down", "arrow.down") { shell.moveDown(itemId) }
                    tool("Task", "checkmark.square") { shell.cycleTask(itemId) }
                    tool("Delete item", "xmark") { shell.delete(itemId) }.foregroundStyle(.red)
                }
            }
            // The field lets go of the keyboard, and tells the core that the edit has ended
            Button("Done") {
                // First: a field that only loses the keyboard may be one that the list took
                // off the screen, and its edit goes on
                shell.finish(itemId)
                UIApplication.shared.sendAction(#selector(UIResponder.resignFirstResponder), to: nil, from: nil, for: nil)
            }
            .padding(.horizontal, 12)
        }
        .background(Color(.secondarySystemBackground))
    }

    /// A tool with a drawing for its face. `name` says what it does, for a screen reader.
    private func tool(_ name: String, _ face: String, _ does: @escaping () -> Void) -> some View {
        Button(action: does) {
            Image(systemName: face).font(.title3).frame(width: 48, height: 44)
        }
        .accessibilityLabel(name)
    }
}
