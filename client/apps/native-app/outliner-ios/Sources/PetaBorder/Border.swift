import Foundation
import PetaCore
import PetaTypes

/// The core behind its border of bytes (ADR 024). Events and answers go in and requests come
/// out, as the generated types. The bytes are bincode.
///
/// The core has the journal inside (ADR 029). The core is made for many threads: the shell
/// takes in files on a thread of its own while the main thread draws.
final class Border: @unchecked Sendable {
    private let lock = NSLock()
    private var core: PetaCore?

    /// `dataDir` keeps the database, the index and the settings of the journal, and
    /// `markdownDir` is the folder the files from Dropbox are put in. `onRequests` gets what
    /// the core asks of the shell. The core may call it from any thread, and for the first
    /// time before this returns. Throws when the journal cannot be opened, as while another
    /// core still has it.
    init(dataDir: URL, markdownDir: URL, onRequests: @escaping @Sendable ([Request]) -> Void) throws {
        core = try PetaCore(dataDir: dataDir.path, markdownDir: markdownDir.path, shell: Listener(onRequests))
    }

    /// The core, for the sync to hear from it when a page changed (ADR 055).
    var petaCore: PetaCore { open }

    private var open: PetaCore {
        lock.withLock {
            guard let core else { preconditionFailure("the border is closed") }
            return core
        }
    }

    /// Takes in the files of the Markdown folder, deletes the pages of the files that have
    /// gone, and writes the pages that changed in the app. Answers with the number of files
    /// taken in. It can take long.
    @discardableResult
    func sync() -> Int {
        Int(open.sync())
    }


    /// Sends an event of the shell.
    func send(_ event: Event) throws {
        try open.update(event: Data(try event.bincodeSerialize()))
    }

    /// Hands a key to the core, which decides what it means (ADR 052). False when it means
    /// nothing to the outline there.
    func press(_ key: KeyPress) throws -> Bool {
        try open.press(key: Data(try key.bincodeSerialize()))
    }

    /// Answers the request with the number `id`. Throws when the core does not wait for it.
    func answer(_ id: UInt32, with output: UiOutput) throws {
        try open.resolve(id: id, output: Data(try output.bincodeSerialize()))
    }

    /// The whole view model, to draw after a render request.
    func view() throws -> ViewModel {
        try ViewModel.bincodeDeserialize(input: [UInt8](open.view()))
    }

    /// Lets go of the journal: its database and its index close.
    func close() {
        lock.withLock { core = nil }
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
