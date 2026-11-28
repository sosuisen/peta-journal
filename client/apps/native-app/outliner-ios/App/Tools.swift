import PetaBorder
import PetaTypes
import SwiftUI

/// The tools at the bottom of the screen, while an item is edited or selected (ADR 031). A phone
/// has no Tab key, no key with Alt and no arrow keys. Two pads of four arrows take their place:
/// one moves the item, the other the cursor. While items are selected, the second pad chooses
/// the item above or below instead. The tools stand in two rows: the pads take both. `target`
/// is the row the tools act on: the one that is edited, or the first that is selected. A touch
/// on a tool leaves the cursor in the field.
struct Tools: View {
    let target: Row
    let shell: Shell
    let onShrink: () -> Void

    /// How high the tools are: two rows of tools.
    static let height: CGFloat = 112

    var body: some View {
        let itemId = target.item.id
        let editing = target.isEditing
        // Never scrolled: the pads are always where the thumb looks for them
        HStack(spacing: 0) {
            VStack(spacing: 0) {
                let folded = target.isCollapsed
                Tool(
                    name: folded ? "Expand" : "Collapse", icon: folded ? "expand" : "collapse",
                    enabled: target.hasChildren
                ) {
                    shell.toggleCollapsed(itemId)
                }
                Tool(name: "Task", icon: "task") { shell.cycleTask(itemId) }
            }
            ArrowPad(name: "Move block", icon: "move_block", moves: Tools.blockMoves, onArrow: moveBlock)
            if editing {
                ArrowPad(name: "Move cursor", icon: "move_cursor", moves: Tools.cursorMoves) { ItemEditor.move($0) }
            } else {
                ArrowPad(name: "Move selection", icon: "move_selection", moves: Tools.selectionMoves) { arrow in
                    if let next = Tools.neighbour(of: shell.view.outline.rows, arrow) { shell.select(next.item.id) }
                }
            }
            if !editing {
                VStack(spacing: 0) {
                    Tool(name: "Copy", icon: "copy", does: shell.copySelected)
                    Tool(name: "Cut", icon: "cut", does: shell.cutSelected)
                }
            }
            VStack(spacing: 0) {
                Tool(name: "Redo", icon: "redo", does: shell.redo)
                Tool(name: "Undo", icon: "undo", does: shell.undo)
            }
            VStack(spacing: 0) {
                Tool(name: "Shrink tools", icon: "shrink_tools", does: onShrink)
                if editing {
                    // The field lets go of the keyboard and the edit ends. The item stays selected
                    Tool(name: "Hide keyboard", icon: "hide_keyboard") {
                        // First: a field that only loses the keyboard may be one that the list
                        // took off the screen, and its edit goes on
                        shell.finish(itemId)
                        shell.select(itemId)
                        UIApplication.shared.sendAction(
                            #selector(UIResponder.resignFirstResponder), to: nil, from: nil, for: nil)
                    }
                } else {
                    Tool(name: "Show keyboard", icon: "show_keyboard") { shell.edit(itemId) }
                }
            }
        }
        .frame(height: Tools.height)
        .background(Color(.secondarySystemBackground).ignoresSafeArea())
    }

    /// The pad that moves the item: the item that is edited, or the selected ones. It reads the
    /// rows when it is touched, as a held touch goes on while they change.
    private func moveBlock(_ arrow: Arrow) {
        if let edited = shell.view.outline.rows.first(where: \.isEditing)?.item.id {
            switch arrow {
            case .left: shell.outdent(edited)
            case .right: shell.indent(edited)
            case .up: shell.moveUp(edited)
            case .down: shell.moveDown(edited)
            }
        } else {
            switch arrow {
            case .left: shell.moveSelected(.outdent)
            case .right: shell.moveSelected(.indent)
            case .up: shell.moveSelected(.moveUp)
            case .down: shell.moveSelected(.moveDown)
            }
        }
    }

    /// The row to select for an arrow of the pad of the selection: the row above the selected
    /// ones, or the row below them. Left and right have none: the pad has no arrows there.
    static func neighbour(of rows: [Row], _ arrow: Arrow) -> Row? {
        switch arrow {
        case .up:
            guard let first = rows.firstIndex(where: \.isSelected), first > 0 else { return nil }
            return rows[first - 1]
        case .down:
            guard let last = rows.lastIndex(where: \.isSelected), last + 1 < rows.count else { return nil }
            return rows[last + 1]
        case .left, .right:
            return nil
        }
    }

    /// What each arrow of the pad that moves the item does, as a screen reader says it.
    private static let blockMoves: [(Arrow, String)] = [
        (.left, "Outdent"), (.up, "Move up"), (.down, "Move down"), (.right, "Indent"),
    ]

    /// What the arrows of the pad that chooses the selected item do, as a screen reader says it.
    private static let selectionMoves: [(Arrow, String)] = [
        (.up, "Select the item above"), (.down, "Select the item below"),
    ]

    /// What each arrow of the pad that moves the cursor does, as a screen reader says it.
    private static let cursorMoves: [(Arrow, String)] = [
        (.left, "Cursor left"), (.up, "Cursor up"), (.down, "Cursor down"), (.right, "Cursor right"),
    ]
}

/// The tools made small: a round button at the bottom right, over the page. A touch makes the
/// tools full again.
struct ShrunkTools: View {
    let onExpand: () -> Void

    var body: some View {
        Button(action: onExpand) {
            Image("expand_tools").resizable().frame(width: 28, height: 28)
                .frame(width: 56, height: 56)
                .background(Circle().fill(Color(.secondarySystemBackground)))
                .shadow(radius: 3)
        }
        .accessibilityLabel("Expand tools")
        .padding(12)
    }
}

/// A tool with an icon of `client/assets/icons` for its face. `name` says what it does, for a
/// screen reader.
private struct Tool: View {
    let name: String
    let icon: String
    var enabled = true
    let does: () -> Void

    var body: some View {
        Button(action: does) {
            Image(icon).resizable().frame(width: 28, height: 28).frame(width: 48, height: Tools.height / 2)
        }
        .disabled(!enabled)
        .accessibilityLabel(name)
    }
}

/// A pad of four arrows. The whole pad takes touches: the lines from its corners cut it into
/// the parts of the arrows (`Pad.aim`). A touch that is held goes on doing what the arrow does.
/// `icon` is its face. `moves` names what each arrow does, for a screen reader.
private struct ArrowPad: View {
    let name: String
    let icon: String
    let moves: [(Arrow, String)]
    let onArrow: (Arrow) -> Void

    /// What the held touch does again and again, until the touch ends.
    @State private var held: Task<Void, Never>?
    /// True from the start of a touch to its end: a touch starts once, where it went down.
    @State private var touching = false

    var body: some View {
        GeometryReader { space in
            Image(icon).resizable().frame(width: 88, height: 88)
                .foregroundStyle(Color.accentColor)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .contentShape(Rectangle())
                .gesture(
                    DragGesture(minimumDistance: 0)
                        .onChanged { touch in
                            guard !touching else { return }
                            touching = true
                            guard let arrow = Pad.aim(at: touch.startLocation, in: space.size) else { return }
                            held = Pad.held { onArrow(arrow) }
                        }
                        .onEnded { _ in
                            touching = false
                            held?.cancel()
                            held = nil
                        }
                )
        }
        .accessibilityElement()
        .accessibilityLabel(name)
        .accessibilityAddTraits(.isButton)
        .accessibilityActions {
            ForEach(moves, id: \.1) { arrow, said in
                Button(said) { onArrow(arrow) }
            }
        }
    }
}
