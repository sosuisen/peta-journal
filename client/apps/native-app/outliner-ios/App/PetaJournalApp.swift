import PetaBorder
import PetaDropbox
import PetaTypes
import SwiftUI

@main
struct PetaJournalApp: App {
    @Environment(\.scenePhase) private var phase
    @StateObject private var opened = Opened()

    var body: some Scene {
        WindowGroup {
            Group {
                if let model = opened.model {
                    Journal(model: model)
                } else {
                    Text("The journal cannot be opened: \(opened.failure)").padding()
                }
            }
            .onOpenURL { opened.model?.cameBack($0) }
            // Midnight, or a clock that was set. The core looks at the clock once a minute too
            .onReceive(NotificationCenter.default.publisher(for: UIApplication.significantTimeChangeNotification)) {
                _ in opened.model?.shell.dayMayHaveChanged()
            }
        }
        .onChange(of: phase) { _, now in
            // The app syncs whenever it comes to the front, and when it goes to the back
            if now == .active { opened.model?.sync(inFront: true) }
            // The wait of the core does not count the time in the back, and the new day would
            // come late
            if now == .active { opened.model?.shell.dayMayHaveChanged() }
            if now == .background { opened.model?.syncInTheBack() }
        }
    }
}

/// The model, or why there is none.
@MainActor
private final class Opened: ObservableObject {
    let model: JournalModel?
    let failure: String

    init() {
        let named = ProcessInfo.processInfo.environment["PETA_ROOT"].map { URL(fileURLWithPath: $0) }
        let support = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        let root = named ?? support.appendingPathComponent("PetaJournal")
        let info = Bundle.main.infoDictionary ?? [:]
        do {
            let account = DropboxAccount(
                // A test of the screen syncs nothing
                key: named == nil ? info["DropboxKey"] as? String ?? "" : "",
                folder: info["DropboxFolder"] as? String ?? "")
            let made = try JournalModel(root: root, account: account)
            Opened.prepareForTest(made)
            model = made
            failure = ""
        } catch {
            model = nil
            failure = String(describing: error)
        }
    }

    /// What a test of the screen asks of the app when it starts it. `PETA_OPEN` is the URL of a
    /// page to open at once, as `/pages/notes`: the test does not type into the search field
    /// for it.
    private static func prepareForTest(_ model: JournalModel) {
        guard let url = ProcessInfo.processInfo.environment["PETA_OPEN"] else { return }
        // Only after the files of the test are taken in: until then, the page is not there.
        // The take-in of the start runs first, on the same thread
        let shell = model.shell
        shell.sync {
            DispatchQueue.main.async { MainActor.assumeIsolated { shell.follow(.page(url: url)) } }
        }
    }
}

/// The screen with the sync it shows.
private struct Journal: View {
    let model: JournalModel

    var body: some View {
        if let cloud = model.cloud {
            Synced(model: model, cloud: cloud)
        } else {
            JournalScreen(shell: model.shell)
        }
    }
}

private struct Synced: View {
    let model: JournalModel
    @ObservedObject var cloud: CloudSync

    var body: some View {
        JournalScreen(
            shell: model.shell,
            cloud: cloud.state,
            syncInFront: cloud.inFront,
            onSignIn: model.signIn,
            onSync: { model.sync() },
            onSignOut: model.signOut)
    }
}
