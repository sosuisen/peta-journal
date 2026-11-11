import AppKit
import PetaBorder
import PetaScreens
import SwiftUI

/// The app. It needs a running journal server, which `-server` on the command line chooses.
/// The stickies of the run before come back. `-stickies` names the file that keeps them.
/// With `-url` the app shows only that page instead, and keeps nothing.
@main
struct PetaJournalApp: App {
    @NSApplicationDelegateAdaptor(Stickies.self) private var stickies

    var body: some Scene {
        // The windows are those of the desk. This scene only gives the app its menus
        Settings { EmptyView() }
    }
}

@MainActor
final class Stickies: NSObject, NSApplicationDelegate {
    private var desk: StickyDesk?

    func applicationDidFinishLaunching(_ notification: Notification) {
        let defaults = UserDefaults.standard
        let server = defaults.string(forKey: "server") ?? "http://localhost:8087"
        if let url = defaults.string(forKey: "url") {
            let desk = StickyDesk(server: server)
            self.desk = desk
            desk.add(url: url)
        } else {
            let file = defaults.string(forKey: "stickies").map { URL(fileURLWithPath: $0) }
            let desk = StickyDesk(server: server, store: StickyStore(file: file ?? StickyStore.standardFile))
            self.desk = desk
            desk.open()
        }
        NSApp.activate(ignoringOtherApps: true)
    }

    /// Quitting closes the windows, and that is not the user closing the stickies.
    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        desk?.end()
        return .terminateNow
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }
}
