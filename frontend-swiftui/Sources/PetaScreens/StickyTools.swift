import SwiftUI

/// The tools of a sticky, which stand in the title bar of its window.
struct StickyTools: View {
    @ObservedObject var look: StickyLook
    /// Opens another sticky.
    let onAdd: () -> Void

    var body: some View {
        HStack(spacing: 6) {
            Button {
                look.searching.toggle()
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
            Button(action: onAdd) {
                Image(systemName: "plus")
            }
            .keyboardShortcut("n")
            .help("New sticky")
        }
        .buttonStyle(.borderless)
        .padding(.horizontal, 8)
    }
}
