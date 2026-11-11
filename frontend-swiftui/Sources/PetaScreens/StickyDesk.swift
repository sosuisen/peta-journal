import AppKit
import Combine
import PetaBorder
import SwiftUI

/// One sticky: a window that shows one page, with a core of its own.
@MainActor
public final class Sticky: Identifiable {
    public let id: String
    public let window: NSWindow
    public let shell: Shell
    fileprivate var hears: Set<AnyCancellable> = []

    fileprivate init(id: String, window: NSWindow, shell: Shell) {
        self.id = id
        self.window = window
        self.shell = shell
    }
}

/// The stickies that are open. A sticky is added with a button of any sticky, and is gone
/// when its window is closed. With a store, the stickies are kept for the next run of the app.
@MainActor
public final class StickyDesk: NSObject, NSWindowDelegate {
    public private(set) var stickies: [Sticky] = []
    private let server: String
    private let store: StickyStore?

    /// The stickies show pages of the journal server at `server`.
    public init(server: String, store: StickyStore? = nil) {
        self.server = server
        self.store = store
    }

    /// Opens the stickies that were kept, or a first one without a page.
    public func open() {
        let kept = store?.stickies ?? []
        if kept.isEmpty {
            add()
        } else {
            kept.forEach { show($0) }
        }
    }

    /// Opens a new sticky that shows what `url` names, or no page yet.
    @discardableResult
    public func add(url: String? = nil) -> Sticky {
        show(nil, url: url)
    }

    /// `kept` is a sticky of the run before, which comes back as it was.
    @discardableResult
    private func show(_ kept: StickySettings?, url: String? = nil) -> Sticky {
        let shell = Shell(server: server, url: kept?.url ?? url)
        let window = NSWindow(
            contentRect: NSRect(origin: .zero, size: NSSize(width: 480, height: 600)),
            styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
        // The desk keeps the window, and lets it go when it is closed
        window.isReleasedWhenClosed = false
        window.contentView = NSHostingView(
            rootView: OutlineScreen(shell: shell, onAdd: { [weak self] in self?.add() }))
        if let kept {
            window.setFrame(NSRect(x: kept.x, y: kept.y, width: kept.width, height: kept.height), display: false)
        } else {
            place(window)
        }
        let sticky = Sticky(id: kept?.id ?? UUID().uuidString, window: window, shell: shell)
        stickies.append(sticky)
        keep(sticky)
        // After the first keeping: the delegate hears the window move
        window.delegate = self
        shell.$url.dropFirst()
            .sink { [weak self, weak sticky] _ in
                // The new value is in the shell only after this call
                DispatchQueue.main.async { if let sticky { self?.keep(sticky) } }
            }
            .store(in: &sticky.hears)
        window.makeKeyAndOrderFront(nil)
        return sticky
    }

    private func keep(_ sticky: Sticky) {
        guard stickies.contains(where: { $0 === sticky }) else { return }
        let frame = sticky.window.frame
        store?.put(
            StickySettings(
                id: sticky.id, url: sticky.shell.url, x: frame.minX, y: frame.minY, width: frame.width,
                height: frame.height))
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

    private func sticky(of notification: Notification) -> Sticky? {
        stickies.first { $0.window === notification.object as? NSWindow }
    }

    public func windowDidMove(_ notification: Notification) {
        sticky(of: notification).map(keep)
    }

    public func windowDidResize(_ notification: Notification) {
        sticky(of: notification).map(keep)
    }

    /// A sticky that the user closed does not come back. The windows are not closed when the
    /// app is ended, so the stickies that are open then do come back.
    public func windowWillClose(_ notification: Notification) {
        guard let closed = sticky(of: notification) else { return }
        stickies.removeAll { $0 === closed }
        store?.remove(closed.id)
    }
}
