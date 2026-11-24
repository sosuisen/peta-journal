package com.sosuisha.petajournal.shell

import com.sosuisha.petajournal.core.PetaCore
import com.sosuisha.petajournal.core.PetaShell
import com.sosuisha.petajournal.types.Changes
import com.sosuisha.petajournal.types.Event
import com.sosuisha.petajournal.types.KeyPress
import com.sosuisha.petajournal.types.Request
import com.sosuisha.petajournal.types.Requests
import com.sosuisha.petajournal.types.Typing
import com.sosuisha.petajournal.types.UiOutput
import java.io.File

/**
 * The core behind its border of bytes (ADR 024). Events and answers go in and requests come out, as
 * the generated types. The bytes are bincode.
 *
 * The core has the journal inside (ADR 029): [dataDir] keeps its database, its index and its
 * settings, and [markdownDir] is the folder the files from Dropbox are put in. [onRequests] gets
 * what the core asks of the shell. The core may call it from any thread, and for the first time
 * before the constructor returns.
 */
class Border(dataDir: File, markdownDir: File, onRequests: (List<Request>) -> Unit) :
    AutoCloseable {
    // The types are generated from the Rust that wrote the bytes. Bytes that cannot be read
    // are a build that mixes two versions, and the exception ends the app
    val core =
        PetaCore(
            dataDir.path,
            markdownDir.path,
            PetaShell { bytes -> onRequests(Requests.bincodeDeserialize(bytes).value) },
        )

    /**
     * Takes in the files of the Markdown folder and deletes the pages of the files that have gone.
     * Answers with the number of files taken in. It can take long.
     */
    fun sync(): Int = core.sync().toInt()

    /**
     * The app goes to the back: the edit ends, and the text typed and still waiting for its save is
     * saved before this returns (#766).
     */
    fun leave() = core.leave()

    /** Sends an event of the shell. */
    fun send(event: Event) = core.update(event.bincodeSerialize())

    /**
     * Hands a key to the core, which decides what it means (ADR 052). False when it means nothing
     * to the outline there.
     */
    /**
     * Hands a change of the Markdown of an item in its text field to the core, which adds the time.
     */
    fun typed(typing: Typing) = core.typed(typing.bincodeSerialize())

    fun press(key: KeyPress): Boolean = core.press(key.bincodeSerialize())

    /** Answers the request with the number [id]. Throws when the core does not wait for it. */
    fun answer(id: UInt, output: UiOutput) = core.resolve(id, output.bincodeSerialize())

    /**
     * What changed on the screen since the last call: the changes to the rows of the outline, and
     * the panels that changed (ADR 076). The first call gives a reset of the rows and every panel.
     * Call it from the main thread only.
     */
    fun changes(): Changes = Changes.bincodeDeserialize(core.changes())

    override fun close() = core.close()
}
