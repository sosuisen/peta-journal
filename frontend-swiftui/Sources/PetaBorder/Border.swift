import Foundation
import PetaCore
import PetaTypes

/// The core behind its border of bytes (ADR 024). Events and answers go in and requests come
/// out, as the generated types. The bytes are bincode.
public final class Border {
    private let core: PetaCore

    /// Starts the core for the journal server at `server`, such as `http://localhost:8080`.
    /// `onRequests` gets what the core asks of the shell. The core may call it from any thread,
    /// and for the first time before this returns.
    public init(server: String, onRequests: @escaping @Sendable ([Request]) -> Void) {
        core = PetaCore(server: server, shell: Listener(onRequests))
    }

    /// Opens what the URL names, such as `/pages/notes` or `/today`.
    public func open(_ url: String) throws {
        try core.open(url: url)
    }

    /// Sends an event of the shell.
    public func send(_ event: Event) throws {
        try core.update(event: Data(try event.bincodeSerialize()))
    }

    /// Answers the request with the number `id`. A request that only tells the shell something,
    /// such as a render, has no answer.
    public func answer(_ id: UInt32, with output: UiOutput) throws {
        try core.resolve(id: id, output: Data(try output.bincodeSerialize()))
    }

    /// The whole view model, to draw after a render request.
    public func view() throws -> ViewModel {
        try ViewModel.bincodeDeserialize(input: [UInt8](core.view()))
    }
}

/// Hears the core and reads its bytes.
private final class Listener: PetaShell {
    private let onRequests: @Sendable ([Request]) -> Void

    init(_ onRequests: @escaping @Sendable ([Request]) -> Void) {
        self.onRequests = onRequests
    }

    func processEffects(bytes: Data) {
        do {
            onRequests(try Requests.bincodeDeserialize(input: [UInt8](bytes)).value)
        } catch {
            // The types here are generated from the Rust that wrote the bytes, so this is a
            // build that mixes two versions
            fatalError("The core sent requests that cannot be read: \(error)")
        }
    }
}
