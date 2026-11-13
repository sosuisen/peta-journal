import PetaBorder
import PetaSync
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
        }
        .onChange(of: phase) { _, now in
            // The app syncs whenever it comes to the front, and when it goes to the back
            if now == .active { opened.model?.sync(inFront: true) }
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
            model = try JournalModel(
                root: root,
                // A test of the screen syncs nothing
                dropboxKey: named == nil ? info["DropboxKey"] as? String ?? "" : "",
                dropboxFolder: info["DropboxFolder"] as? String ?? "")
            failure = ""
        } catch {
            model = nil
            failure = String(describing: error)
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
