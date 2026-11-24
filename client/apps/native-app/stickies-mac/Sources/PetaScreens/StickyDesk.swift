import AppKit
import Combine
import PetaBorder
import PetaCore
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
final class StickyWindow: NSWindow {
    var onTitleBarDoubleClick: (() -> Void)?
    /// Makes the menu of the sticky. A right click shows it, on the title bar and on an item
    /// that is not edited.
    var stickyMenu: (() -> NSMenu?)?

    /// A click of the right button, or of the left one while the Control key is down.
    private func asksForMenu(_ event: NSEvent) -> Bool {
        event.type == .rightMouseDown || (event.type == .leftMouseDown && event.modifierFlags.contains(.control))
    }

    override func sendEvent(_ event: NSEvent) {
        // Clicks in a quick row count on: 3, 4, ... macOS would zoom the window at a later one
        if event.type == .leftMouseDown, event.clickCount >= 2, isOnBareTitleBar(event.locationInWindow) {
            if event.clickCount.isMultiple(of: 2) { onTitleBarDoubleClick?() }
            return
        }
        if asksForMenu(event), let frame = contentView?.superview {
            if isOnBareTitleBar(event.locationInWindow), let menu = stickyMenu?() {
                NSMenu.popUpContextMenu(menu, with: event, for: frame)
                return
            }
            // Before the click reaches the item: a text view takes the cursor first, and the
            // item would turn into Markdown and show the menu of its text
            let place = frame.convert(event.locationInWindow, from: nil)
            if let item = contentView?.hitTest(place) as? ItemTextView, !item.isMarkdown,
                let menu = stickyMenu?()
            {
                NSMenu.popUpContextMenu(menu, with: event, for: item)
                return
            }
        }
        super.sendEvent(event)
    }

    /// The buttons that close, minimize and zoom the window.
    var windowButtons: [NSView] {
        let kinds: [NSWindow.ButtonType] = [.closeButton, .miniaturizeButton, .zoomButton]
        return kinds.compactMap { standardWindowButton($0) }
    }

    /// On the title bar, and not on a button or a tool of it. The name of the window is no
    /// button: it is text, which macOS counts as a control as well.
    private func isOnBareTitleBar(_ place: NSPoint) -> Bool {
        guard place.y > contentLayoutRect.maxY else { return false }
        let taken = windowButtons + titlebarAccessoryViewControllers.map(\.view)
        return !taken.contains { !$0.isHidden && $0.convert($0.bounds, to: nil).contains(place) }
    }
}

/// A view of SwiftUI that takes the click that gives its window the focus. Without this the
/// first click on a window without the focus only brings the window to the front, and the
/// button under the mouse is not pressed.
private final class FirstClickHostingView<Content: View>: NSHostingView<Content> {
    override func acceptsFirstMouse(for event: NSEvent?) -> Bool {
        true
    }
}

/// An entry of a menu that runs a closure.
private final class MenuEntry: NSMenuItem {
    private let run: () -> Void

    init(_ title: String, checked: Bool, run: @escaping () -> Void) {
        self.run = run
        super.init(title: title, action: #selector(chosen), keyEquivalent: "")
        target = self
        state = checked ? .on : .off
    }

    @available(*, unavailable)
    required init(coder: NSCoder) {
        fatalError("A menu entry is made in code")
    }

    @objc private func chosen() {
        run()
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
    /// The paper is not smaller than this, as on Windows (`client-stickies`).
    private static let smallest = NSSize(width: smallestSticky().width, height: smallestSticky().height)

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
            for sticky in kept { show(sticky) }
        }
    }

    /// Opens a new sticky that shows what `url` names, or no page yet. It comes a little down
    /// and to the right of `origin`, the window it was asked from, or of the newest sticky.
    @discardableResult
    public func add(url: String? = nil, nextTo origin: NSWindow? = nil) -> Sticky {
        show(nil, url: url, nextTo: origin)
    }

    /// `kept` is a sticky of the run before, which comes back as it was.
    @discardableResult
    private func show(_ kept: StickySettings?, url: String? = nil, nextTo origin: NSWindow? = nil) -> Sticky {
        // The window of this sticky, for the stickies asked from it. The shell comes first
        weak var made: NSWindow?
        let shell = Shell(
            server: server, url: kept?.url ?? url,
            openSticky: { [weak self] in self?.add(url: $0, nextTo: made) })
        let look = StickyLook(
            color: kept?.color ?? StickySettings.yellow, alwaysOnTop: kept?.alwaysOnTop ?? false,
            zoom: kept?.zoom ?? 1)
        let window = StickyWindow(
            contentRect: NSRect(origin: .zero, size: NSSize(width: usualSticky().width, height: usualSticky().height)),
            styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
        made = window
        // The desk keeps the window, and lets it go when it is closed
        window.isReleasedWhenClosed = false
        // The paper is light in every color, and so the text on it is always dark
        window.appearance = NSAppearance(named: .aqua)
        window.titlebarAppearsTransparent = true
        paint(window, color: look.color)
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
        let buttons = FirstClickHostingView(
            rootView: StickyTools(look: look, shell: shell, onAdd: { [weak self] in self?.add(nextTo: made) }))
        buttons.frame.size = buttons.fittingSize
        tools.view = buttons
        window.addTitlebarAccessoryViewController(tools)
        if let kept {
            window.setFrame(NSRect(x: kept.x, y: kept.y, width: kept.width, height: kept.height), display: false)
        } else {
            place(window, nextTo: origin)
        }
        let sticky = Sticky(id: kept?.id ?? UUID().uuidString, window: window, shell: shell, look: look)
        stickies.append(sticky)
        window.onTitleBarDoubleClick = { [weak self, weak sticky] in
            if let sticky { self?.roll(sticky) }
        }
        window.stickyMenu = { [weak self, weak sticky] in
            sticky.flatMap { self?.menu(of: $0) }
        }
        if kept?.rolledUp == true { roll(sticky, animated: false) }
        keep(sticky)
        // After the first keeping: the delegate hears the window move
        window.delegate = self
        // A published value is told before it is in its place, so it is handed over
        shell.$url.dropFirst()
            .sink { [weak self, weak sticky] url in
                if let sticky { self?.keep(sticky, url: .some(url)) }
            }
            .store(in: &sticky.hears)
        // The core looks at the clock once a minute, but its wait does not count the time the
        // Mac slept: after a night asleep, the journal of today would show the old day a while
        NotificationCenter.default.publisher(for: .NSCalendarDayChanged)
            .merge(with: NSWorkspace.shared.notificationCenter.publisher(for: NSWorkspace.didWakeNotification))
            // The change of the day is told on any thread
            .receive(on: DispatchQueue.main)
            .sink { [weak shell] _ in MainActor.assumeIsolated { shell?.dayMayHaveChanged() } }
            .store(in: &sticky.hears)
        // The paper is not the content view, so its name does not reach the window by itself.
        // A window without a name is hard to find among the others
        shell.$title
            .sink { [weak window] title in window?.title = title.isEmpty ? "PetaStickies" : title }
            .store(in: &sticky.hears)
        look.$color.dropFirst()
            .sink { [weak self, weak sticky] color in
                guard let sticky else { return }
                self?.paint(sticky.window, color: color)
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
    private func keep(
        _ sticky: Sticky, url: String?? = nil, color: String? = nil, alwaysOnTop: Bool? = nil,
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
    /// stays where it is. The window moves there in steps, as the Stickies of macOS do.
    public func roll(_ sticky: Sticky, animated: Bool = true) {
        let window = sticky.window
        let frame = window.frame
        let anyHeight = NSSize(width: CGFloat.greatestFiniteMagnitude, height: .greatestFiniteMagnitude)
        let down = sticky.rolledUp
        let height: CGFloat
        if down {
            window.contentMaxSize = anyHeight
            height = sticky.fullHeight
        } else {
            sticky.fullHeight = frame.height
            sticky.rolledUp = true
            showTools(of: sticky)
            window.contentMinSize = NSSize(width: StickyDesk.smallest.width, height: 0)
            height = window.frameRect(forContentRect: .zero).height
        }
        // On the way the sticky counts as rolled up: what is kept then is its whole height,
        // not the height of a step
        window.setFrame(
            NSRect(x: frame.minX, y: frame.maxY - height, width: frame.width, height: height), display: true,
            animate: animated)
        if down {
            sticky.rolledUp = false
            window.contentMinSize = StickyDesk.smallest
            showTools(of: sticky)
        } else {
            // The width can still be changed, the height cannot
            window.contentMaxSize = NSSize(width: anyHeight.width, height: 0)
        }
        keep(sticky)
    }

    /// The menu that a right click on the title bar of the sticky shows, and one on an item
    /// that is not edited: the pin, and the colors. The pin and the color of the sticky are checked.
    public func menu(of sticky: Sticky) -> NSMenu {
        let look = sticky.look
        let menu = NSMenu()
        // The entries do not ask a responder whether they can be chosen: they always can
        menu.autoenablesItems = false
        menu.addItem(MenuEntry("Keep on Top", checked: look.alwaysOnTop) { look.alwaysOnTop.toggle() })
        menu.addItem(.separator())
        for color in StickyLook.colors {
            menu.addItem(MenuEntry(color.name, checked: look.color == color.hex) { look.color = color.hex })
        }
        return menu
    }

    /// Gives the window the color of its paper. Without paper the window is not opaque, and
    /// what is behind it shows through.
    private func paint(_ window: NSWindow, color: String) {
        window.backgroundColor = StickyLook.color(of: color)
        window.isOpaque = !StickyLook.isTransparent(color)
    }

    /// A little down and to the right of `origin`, or of the newest sticky, as new windows are
    /// everywhere on the Mac. Near the edge of the screen, AppKit keeps the window on it.
    private func place(_ window: NSWindow, nextTo origin: NSWindow?) {
        guard let before = origin ?? stickies.last?.window else {
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

    /// The search tool and the buttons of the window are for the sticky that is worked on. On
    /// the others they are only in the way of the name, and a sticky that is rolled up has no
    /// paper to work on. The plus stays.
    private func showTools(of sticky: Sticky) {
        let shown = sticky.hasFocus && !sticky.rolledUp
        // The views of the tools hide the search tool themselves, and keep the plus: a new
        // sticky is one click away from every sticky
        if sticky.look.toolsShown != shown { sticky.look.toolsShown = shown }
        // A sticky that is not worked on is only its name and the plus
        for button in (sticky.window as? StickyWindow)?.windowButtons ?? [] { button.isHidden = !shown }
    }

    /// The app is ending. The windows that macOS closes from here on were not closed by the
    /// user, and so their stickies stay kept.
    /// What was typed in every sticky and still waits for its save is saved now (#766).
    public func end() {
        ending = true
        for sticky in stickies { sticky.shell.close() }
    }

    /// What was typed in the sticky and still waits for its save is saved. A sticky that the
    /// user closed does not come back.
    public func windowWillClose(_ notification: Notification) {
        guard let closed = sticky(of: notification) else { return }
        closed.shell.close()
        guard !ending else { return }
        stickies.removeAll { $0 === closed }
        store?.remove(closed.id)
    }
}
