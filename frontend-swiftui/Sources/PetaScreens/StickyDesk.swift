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
    public let look: StickyLook
    fileprivate var hears: Set<AnyCancellable> = []

    fileprivate init(id: String, window: NSWindow, shell: Shell, look: StickyLook) {
        self.id = id
        self.window = window
        self.shell = shell
        self.look = look
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
        let look = StickyLook(color: kept?.color ?? StickySettings.yellow, alwaysOnTop: kept?.alwaysOnTop ?? false,
            zoom: kept?.zoom ?? 1)
        let window = NSWindow(
            contentRect: NSRect(origin: .zero, size: NSSize(width: 480, height: 600)),
            styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
        // The desk keeps the window, and lets it go when it is closed
        window.isReleasedWhenClosed = false
        // The paper is light in every color, and so the text on it is always dark
        window.appearance = NSAppearance(named: .aqua)
        window.titlebarAppearsTransparent = true
        window.backgroundColor = StickyLook.color(of: look.color)
        window.level = look.alwaysOnTop ? .floating : .normal
        window.contentView = NSHostingView(
            rootView: OutlineScreen(shell: shell, look: look, onAdd: { [weak self] in self?.add() }))
        if let kept {
            window.setFrame(NSRect(x: kept.x, y: kept.y, width: kept.width, height: kept.height), display: false)
        } else {
            place(window)
        }
        let sticky = Sticky(id: kept?.id ?? UUID().uuidString, window: window, shell: shell, look: look)
        stickies.append(sticky)
        keep(sticky)
        // After the first keeping: the delegate hears the window move
        window.delegate = self
        // A published value is told before it is in its place, so it is handed over
        shell.$url.dropFirst()
            .sink { [weak self, weak sticky] url in
                if let sticky { self?.keep(sticky, url: .some(url)) }
            }
            .store(in: &sticky.hears)
        look.$color.dropFirst()
            .sink { [weak self, weak sticky] color in
                guard let sticky else { return }
                sticky.window.backgroundColor = StickyLook.color(of: color)
                self?.keep(sticky, color: color)
            }
            .store(in: &sticky.hears)
        look.$alwaysOnTop.dropFirst()
            .sink { [weak self, weak sticky] onTop in
                guard let sticky else { return }
                sticky.window.level = onTop ? .floating : .normal
                self?.keep(sticky, alwaysOnTop: onTop)
            }
            .store(in: &sticky.hears)
        look.$zoom.dropFirst()
            .sink { [weak self, weak sticky] zoom in
                if let sticky { self?.keep(sticky, zoom: zoom) }
            }
            .store(in: &sticky.hears)
        window.makeKeyAndOrderFront(nil)
        return sticky
    }

    /// Keeps the sticky as it is now. The values handed over are those that are just changing.
    private func keep(_ sticky: Sticky, url: String?? = nil, color: String? = nil, alwaysOnTop: Bool? = nil,
        zoom: Double? = nil
    ) {
        guard stickies.contains(where: { $0 === sticky }) else { return }
        let frame = sticky.window.frame
        store?.put(
            StickySettings(
                id: sticky.id, url: url ?? sticky.shell.url, x: frame.minX, y: frame.minY, width: frame.width,
                height: frame.height, color: color ?? sticky.look.color,
                alwaysOnTop: alwaysOnTop ?? sticky.look.alwaysOnTop, zoom: zoom ?? sticky.look.zoom))
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
        sticky(of: notification).map { keep($0) }
    }

    public func windowDidResize(_ notification: Notification) {
        sticky(of: notification).map { keep($0) }
    }

    /// A sticky that the user closed does not come back. The windows are not closed when the
    /// app is ended, so the stickies that are open then do come back.
    public func windowWillClose(_ notification: Notification) {
        guard let closed = sticky(of: notification) else { return }
        stickies.removeAll { $0 === closed }
        store?.remove(closed.id)
    }
}
