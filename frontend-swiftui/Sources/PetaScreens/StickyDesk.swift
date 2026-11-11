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
    /// Only the title bar of the window is shown.
    public fileprivate(set) var rolledUp = false
    /// The height of the window before it was rolled up.
    fileprivate var fullHeight: CGFloat = 0
    /// The window is the one that is worked in.
    fileprivate var hasFocus = false
    fileprivate var hears: Set<AnyCancellable> = []

    fileprivate init(id: String, window: NSWindow, shell: Shell, look: StickyLook) {
        self.id = id
        self.window = window
        self.shell = shell
        self.look = look
    }
}

/// The window of a sticky. It hears the double click on its title bar, which macOS would
/// answer by zooming or minimizing the window.
private final class StickyWindow: NSWindow {
    var onTitleBarDoubleClick: (() -> Void)?

    override func sendEvent(_ event: NSEvent) {
        if event.type == .leftMouseDown, event.clickCount == 2, isOnBareTitleBar(event.locationInWindow) {
            onTitleBarDoubleClick?()
            return
        }
        super.sendEvent(event)
    }

    /// On the title bar, and not on a button of it.
    private func isOnBareTitleBar(_ place: NSPoint) -> Bool {
        guard place.y > contentLayoutRect.maxY, let frame = contentView?.superview else { return false }
        if titlebarAccessoryViewControllers.contains(where: { !$0.view.isHidden && $0.view.convert($0.view.bounds, to: nil).contains(place) }) {
            return false
        }
        return !(frame.hitTest(frame.convert(place, from: nil)) is NSControl)
    }
}

/// The stickies that are open. A sticky is added with a button of any sticky, and is gone
/// when its window is closed. With a store, the stickies are kept for the next run of the app.
@MainActor
public final class StickyDesk: NSObject, NSWindowDelegate {
    public private(set) var stickies: [Sticky] = []
    private let server: String
    private let store: StickyStore?
    /// The app is ending.
    private var ending = false
    /// The paper is not smaller than this.
    private static let smallest = NSSize(width: 480, height: 320)

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
        let window = StickyWindow(
            contentRect: NSRect(origin: .zero, size: NSSize(width: 480, height: 600)),
            styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
        // The desk keeps the window, and lets it go when it is closed
        window.isReleasedWhenClosed = false
        // The paper is light in every color, and so the text on it is always dark
        window.appearance = NSAppearance(named: .aqua)
        window.titlebarAppearsTransparent = true
        window.backgroundColor = StickyLook.color(of: look.color)
        window.level = look.alwaysOnTop ? .floating : .normal
        let paper = NSHostingView(rootView: OutlineScreen(shell: shell, look: look))
        // The paper lies in a plain view. As the content view itself, it would not let the
        // window be smaller than what is on it, and so not be rolled up
        paper.sizingOptions = []
        let under = NSView(frame: NSRect(origin: .zero, size: window.contentLayoutRect.size))
        paper.frame = under.bounds
        paper.autoresizingMask = [.width, .height]
        under.addSubview(paper)
        window.contentView = under
        window.contentMinSize = StickyDesk.smallest
        let tools = NSTitlebarAccessoryViewController()
        tools.layoutAttribute = .trailing
        let buttons = NSHostingView(rootView: StickyTools(look: look, onAdd: { [weak self] in self?.add() }))
        buttons.frame.size = buttons.fittingSize
        tools.view = buttons
        window.addTitlebarAccessoryViewController(tools)
        if let kept {
            window.setFrame(NSRect(x: kept.x, y: kept.y, width: kept.width, height: kept.height), display: false)
        } else {
            place(window)
        }
        let sticky = Sticky(id: kept?.id ?? UUID().uuidString, window: window, shell: shell, look: look)
        stickies.append(sticky)
        window.onTitleBarDoubleClick = { [weak self, weak sticky] in
            if let sticky { self?.roll(sticky) }
        }
        if kept?.rolledUp == true { roll(sticky) }
        keep(sticky)
        // After the first keeping: the delegate hears the window move
        window.delegate = self
        // A published value is told before it is in its place, so it is handed over
        shell.$url.dropFirst()
            .sink { [weak self, weak sticky] url in
                if let sticky { self?.keep(sticky, url: .some(url)) }
            }
            .store(in: &sticky.hears)
        // The paper is not the content view, so its name does not reach the window by itself.
        // A window without a name is hard to find among the others
        shell.$title
            .sink { [weak window] title in window?.title = title.isEmpty ? "PetaStickies" : title }
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
        sticky.hasFocus = window.isKeyWindow
        showTools(of: sticky)
        return sticky
    }

    /// Keeps the sticky as it is now. The values handed over are those that are just changing.
    private func keep(_ sticky: Sticky, url: String?? = nil, color: String? = nil, alwaysOnTop: Bool? = nil,
        zoom: Double? = nil
    ) {
        guard stickies.contains(where: { $0 === sticky }) else { return }
        let frame = sticky.window.frame
        // What is kept is the whole sticky, also while only its title bar is shown
        let height = sticky.rolledUp ? sticky.fullHeight : frame.height
        store?.put(
            StickySettings(
                id: sticky.id, url: url ?? sticky.shell.url, x: frame.minX, y: frame.maxY - height,
                width: frame.width, height: height, color: color ?? sticky.look.color,
                alwaysOnTop: alwaysOnTop ?? sticky.look.alwaysOnTop, zoom: zoom ?? sticky.look.zoom,
                rolledUp: sticky.rolledUp))
    }

    /// Rolls the sticky up to its title bar, or down again to the height it had. The top edge
    /// stays where it is.
    public func roll(_ sticky: Sticky) {
        let window = sticky.window
        let frame = window.frame
        let height: CGFloat
        if sticky.rolledUp {
            sticky.rolledUp = false
            window.contentMinSize = StickyDesk.smallest
            window.contentMaxSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: .greatestFiniteMagnitude)
            height = sticky.fullHeight
        } else {
            sticky.fullHeight = frame.height
            sticky.rolledUp = true
            // The width can still be changed, the height cannot
            window.contentMinSize = NSSize(width: StickyDesk.smallest.width, height: 0)
            window.contentMaxSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: 0)
            height = window.frameRect(forContentRect: .zero).height
        }
        window.setFrame(
            NSRect(x: frame.minX, y: frame.maxY - height, width: frame.width, height: height), display: true)
        showTools(of: sticky)
        keep(sticky)
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

    public func windowDidBecomeKey(_ notification: Notification) {
        focus(true, of: notification)
    }

    public func windowDidResignKey(_ notification: Notification) {
        focus(false, of: notification)
    }

    private func focus(_ has: Bool, of notification: Notification) {
        guard let sticky = sticky(of: notification) else { return }
        sticky.hasFocus = has
        showTools(of: sticky)
    }

    /// The tools are for the sticky that is worked on. On the others they are only in the way
    /// of the paper, and a sticky that is rolled up has no paper to work on.
    private func showTools(of sticky: Sticky) {
        let shown = sticky.hasFocus && !sticky.rolledUp
        // The view, not its controller: hiding the controller leaves the view on screen
        // until the title bar is laid out again
        sticky.window.titlebarAccessoryViewControllers.forEach { $0.view.isHidden = !shown }
    }

    /// The app is ending. The windows that macOS closes from here on were not closed by the
    /// user, and so their stickies stay kept.
    public func end() {
        ending = true
    }

    /// A sticky that the user closed does not come back.
    public func windowWillClose(_ notification: Notification) {
        guard !ending, let closed = sticky(of: notification) else { return }
        stickies.removeAll { $0 === closed }
        store?.remove(closed.id)
    }
}
