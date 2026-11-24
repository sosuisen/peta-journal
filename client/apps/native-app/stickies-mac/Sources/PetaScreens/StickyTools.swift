import PetaBorder
import SwiftUI

/// The tools of a sticky, which stand in the title bar of its window. The pin and the colors
/// are not here: they are in the menu of the title bar, and leave the bar to the name.
///
/// The search tool and the arrows of the history are for the sticky that is worked on. The
/// plus is always there.
struct StickyTools: View {
    @ObservedObject var look: StickyLook
    @ObservedObject var shell: Shell
    /// Opens another sticky.
    let onAdd: () -> Void

    var body: some View {
        HStack(spacing: 6) {
            // The keys of the history in the browsers and the Finder of macOS (ADR 035)
            Button(action: shell.goBack) {
                Image(systemName: "chevron.left")
            }
            .keyboardShortcut("[")
            .help("Back")
            .opacity(look.toolsShown ? 1 : 0)
            .disabled(!Self.usable(shown: look.toolsShown, possible: shell.view.history.canGoBack))
            Button(action: shell.goForward) {
                Image(systemName: "chevron.right")
            }
            .keyboardShortcut("]")
            .help("Forward")
            .opacity(look.toolsShown ? 1 : 0)
            .disabled(!Self.usable(shown: look.toolsShown, possible: shell.view.history.canGoForward))
            Button {
                look.searching.toggle()
            } label: {
                Image(systemName: "magnifyingglass")
            }
            .keyboardShortcut("f")
            .help("Search pages")
            // The tool keeps its place while it is not shown: the plus beside it does not move
            .opacity(look.toolsShown ? 1 : 0)
            .disabled(!look.toolsShown)
            // A new sticky is one click away from every sticky: one without the focus, and
            // one that is rolled up. So this button is always shown
            Button(action: onAdd) {
                Image(systemName: "plus")
            }
            .keyboardShortcut("n")
            .help("New sticky")
        }
        .buttonStyle(.borderless)
        // The zoom has keys only. Buttons that are not seen give the keys to the window
        .background {
            Group {
                Button("Zoom In") { look.zoomIn() }.keyboardShortcut("+")
                Button("Zoom Out") { look.zoomOut() }.keyboardShortcut("-")
                Button("Actual Size") { look.resetZoom() }.keyboardShortcut("0")
            }
            .opacity(0)
            .frame(width: 0, height: 0)
        }
        .padding(.horizontal, 8)
    }

    /// An arrow of the history can be clicked. Only while it is shown: a sticky without the
    /// focus keeps the place of its tools, and a click there must not go back.
    static func usable(shown: Bool, possible: Bool) -> Bool {
        shown && possible
    }
}
