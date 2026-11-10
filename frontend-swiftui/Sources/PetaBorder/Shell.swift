import Combine
import Foundation
@preconcurrency import PetaTypes

/// What one window shows, kept up to date with the core. The views draw `view` and tell this
/// what the user did. All of it runs on the main thread.
@MainActor
public final class Shell: ObservableObject {
    @Published public private(set) var view: ViewModel
    @Published public private(set) var title = ""
    private let border: Border

    /// Starts a core for the journal server at `server` and opens what `url` names.
    public init(server: String, url: String) {
        let inbox = Inbox()
        let border = Border(server: server) { requests in
            // The core calls from its own threads. The main queue keeps the requests in order
            DispatchQueue.main.async {
                MainActor.assumeIsolated { inbox.shell?.handle(requests) }
            }
        }
        self.border = border
        view = Shell.must { try border.view() }
        inbox.shell = self
        open(url)
    }

    /// Opens what the URL names. A URL that names nothing changes nothing.
    public func open(_ url: String) {
        try? border.open(url)
    }

    /// The user put the cursor in the item.
    public func startEditing(_ itemId: String) {
        send(.editRequested(itemId: itemId, cursor: 0))
    }

    /// The user changed the text of the item. The core saves it a moment later.
    public func typed(_ body: String, into itemId: String) {
        let nowMs = UInt64(Date().timeIntervalSince1970 * 1000)
        send(.bodyChanged(itemId: itemId, body: body, nowMs: nowMs))
    }

    private func send(_ event: Event) {
        Shell.must { try border.send(event) }
        // Not left to the render request. It comes a turn of the main queue later, and a text
        // field that reads its old text back after a key loses the key. Some events, such as
        // the start of an edit, send no render request at all
        view = Shell.must { try border.view() }
    }

    private func handle(_ requests: [Request]) {
        for request in requests {
            switch request.effect {
            case .render:
                view = Shell.must { try border.view() }
            case .navigate(let op):
                open(op.url)
            case .ui(let op):
                show(op)
            }
        }
    }

    private func show(_ op: UiOp) {
        switch op {
        case .setTitle(let title):
            self.title = title
        default:
            break
        }
    }

    /// Runs what crosses the border. The types on both sides are generated from one source, so
    /// a failure here is a build that mixes two versions.
    private static func must<T>(_ crossing: () throws -> T) -> T {
        do {
            return try crossing()
        } catch {
            fatalError("The border cannot be crossed: \(error)")
        }
    }
}

/// Where the requests of the core find the shell, which is made after the core.
private final class Inbox: @unchecked Sendable {
    weak var shell: Shell?
}
