import PetaBorder
import PetaScreens
import SwiftUI

/// The app. It needs a running journal server. `-server` and `-url` on the command line choose
/// the server and the page to open. Without `-url` no page is open, and one is searched for.
@main
struct PetaJournalApp: App {
    @StateObject private var shell = Shell(
        server: UserDefaults.standard.string(forKey: "server") ?? "http://localhost:8087",
        url: UserDefaults.standard.string(forKey: "url")
    )

    var body: some Scene {
        WindowGroup {
            OutlineScreen(shell: shell)
        }
    }
}
