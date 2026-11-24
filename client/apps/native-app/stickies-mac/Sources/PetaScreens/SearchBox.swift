import AppKit
import PetaBorder
import PetaTypes
import SwiftUI

/// The search box with what it found under it. An entry that is chosen opens its page.
struct SearchBox: View {
    @ObservedObject var shell: Shell
    /// Escape was pressed with nothing left to hide.
    let onClose: () -> Void

    var body: some View {
        let search = shell.view.search
        VStack(alignment: .leading, spacing: 4) {
            SearchField(shell: shell, query: search.query, onClose: onClose)
                .frame(height: 22)
            if search.showDropdown {
                // More entries than the window is high are scrolled: they would push the
                // screen out of the window. Fewer take only the room they need
                ViewThatFits(in: .vertical) {
                    entries(of: search)
                    ScrollViewReader { scroll in
                        ScrollView { entries(of: search) }
                            // The arrow keys may choose an entry that is out of sight
                            .onChange(of: search.selectedIndex) { chosen in
                                if let chosen { scroll.scrollTo(chosen) }
                            }
                    }
                }
                if search.foundNothing {
                    Text("Nothing found").font(.caption).foregroundStyle(.secondary)
                }
            }
        }
    }

    private func entries(of search: SearchView) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            ForEach(search.results, id: \.position) { entry in
                Button {
                    shell.pick(entry.position)
                } label: {
                    SearchEntryRow(entry: entry, isChosen: search.selectedIndex == entry.position)
                }
                .buttonStyle(.plain)
                .id(entry.position)
            }
        }
    }
}

extension SearchView {
    /// The search for what was typed is over, and found no page.
    var foundNothing: Bool {
        results.isEmpty && !searching && !query.isEmpty
    }
}

extension SearchEntry {
    /// The words around what was found, under the name of the entry. `nil` when there are
    /// none: the snippet of a page found by its name is the name again.
    var shownSnippet: String? {
        snippet.isEmpty || snippet == title ? nil : snippet
    }
}

private struct SearchEntryRow: View {
    let entry: SearchEntry
    let isChosen: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 1) {
            Text(entry.label)
            // The snippet of a page found by its name is the name again
            if let snippet = entry.shownSnippet {
                Text(snippet).font(.caption).foregroundStyle(.secondary).lineLimit(1)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.vertical, 2)
        .padding(.horizontal, 4)
        .background(isChosen ? Color.accentColor.opacity(0.25) : Color.clear)
        .contentShape(Rectangle())
    }
}

/// A text field of AppKit: one of SwiftUI does not give the arrow keys on macOS 13.
private struct SearchField: NSViewRepresentable {
    let shell: Shell
    let query: String
    let onClose: () -> Void

    func makeCoordinator() -> Hearer {
        Hearer(shell: shell, onClose: onClose)
    }

    func makeNSView(context: Context) -> NSTextField {
        let field = NSTextField()
        field.placeholderString = "Search pages"
        field.delegate = context.coordinator
        field.setAccessibilityIdentifier("search")
        // The box was opened to type into it
        context.coordinator.takeCursor(field)
        return field
    }

    func updateNSView(_ field: NSTextField, context: Context) {
        // The core empties the query when a page was chosen. Text that is still being
        // composed is not in the core yet
        let composing = (field.currentEditor() as? NSTextView)?.hasMarkedText() ?? false
        if !composing, field.stringValue != query { field.stringValue = query }
    }

    @MainActor
    final class Hearer: NSObject, NSTextFieldDelegate {
        private let shell: Shell
        private let onClose: () -> Void

        init(shell: Shell, onClose: @escaping () -> Void) {
            self.shell = shell
            self.onClose = onClose
        }

        private static let keys: [Selector: SearchKey] = [
            #selector(NSResponder.moveDown(_:)): .down,
            #selector(NSResponder.moveUp(_:)): .up,
            #selector(NSResponder.insertNewline(_:)): .enter,
            #selector(NSResponder.cancelOperation(_:)): .escape,
        ]

        func controlTextDidChange(_ notification: Notification) {
            guard let field = notification.object as? NSTextField else { return }
            if (field.currentEditor() as? NSTextView)?.hasMarkedText() ?? false { return }
            shell.search(field.stringValue)
        }

        func control(_ control: NSControl, textView: NSTextView, doCommandBy selector: Selector) -> Bool {
            guard let key = Hearer.keys[selector] else { return false }
            if key == .escape, !shell.view.search.showDropdown {
                onClose()
                return true
            }
            shell.search(key: key)
            return true
        }

        /// The field is not in its window yet when it is made.
        func takeCursor(_ field: NSTextField, tries: Int = 10) {
            DispatchQueue.main.async { [self] in
                if let window = field.window {
                    window.makeFirstResponder(field)
                } else if tries > 0 {
                    takeCursor(field, tries: tries - 1)
                }
            }
        }
    }
}
