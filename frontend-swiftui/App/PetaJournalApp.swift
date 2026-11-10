import AppKit
import PetaScreens
import SwiftUI

/// The app. It needs a running journal server. `-server` and `-url` on the command line choose
/// the server and the page of the first sticky. Without `-url` the sticky has no page, and one
/// is searched for.
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
        let desk = StickyDesk(
            server: UserDefaults.standard.string(forKey: "server") ?? "http://localhost:8087")
        self.desk = desk
        desk.add(url: UserDefaults.standard.string(forKey: "url"))
        NSApp.activate(ignoringOtherApps: true)
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }
}
