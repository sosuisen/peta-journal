import AppKit
import PetaBorder
import SwiftUI

/// One sticky: a window that shows one page, with a core of its own.
@MainActor
public final class Sticky: Identifiable {
    public let id: String
    public let window: NSWindow
    public let shell: Shell

    fileprivate init(id: String, window: NSWindow, shell: Shell) {
        self.id = id
        self.window = window
        self.shell = shell
    }
}

/// The stickies that are open. A sticky is added with a button of any sticky, and is gone
/// when its window is closed.
@MainActor
public final class StickyDesk: NSObject, NSWindowDelegate {
    public private(set) var stickies: [Sticky] = []
    private let server: String

    /// The stickies show pages of the journal server at `server`.
    public init(server: String) {
        self.server = server
    }

    /// Opens a new sticky that shows what `url` names, or no page yet.
    @discardableResult
    public func add(url: String? = nil) -> Sticky {
        let shell = Shell(server: server, url: url)
        let window = NSWindow(
            contentRect: NSRect(origin: .zero, size: NSSize(width: 480, height: 600)),
            styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
        // The desk keeps the window, and lets it go when it is closed
        window.isReleasedWhenClosed = false
        window.contentView = NSHostingView(
            rootView: OutlineScreen(shell: shell, onAdd: { [weak self] in self?.add() }))
        window.delegate = self
        let sticky = Sticky(id: UUID().uuidString, window: window, shell: shell)
        place(window)
        stickies.append(sticky)
        window.makeKeyAndOrderFront(nil)
        return sticky
    }

    /// A little down and to the right of the newest sticky, as new windows are everywhere
    /// on the Mac.
    private func place(_ window: NSWindow) {
        guard let before = stickies.last?.window else {
            window.center()
            return
        }
        let corner = NSPoint(x: before.frame.minX, y: before.frame.maxY)
        window.setFrameTopLeftPoint(window.cascadeTopLeft(from: corner))
        // The first call only moves the window to the corner it was given
        window.setFrameTopLeftPoint(window.cascadeTopLeft(from: corner))
    }

    public func windowWillClose(_ notification: Notification) {
        guard let closed = notification.object as? NSWindow else { return }
        stickies.removeAll { $0.window === closed }
    }
}
