package com.sosuisha.petajournal.shell

import com.sosuisha.petajournal.core.FfiException
import com.sosuisha.petajournal.core.PetaCore
import com.sosuisha.petajournal.core.PetaShell
import com.sosuisha.petajournal.types.Event
import com.sosuisha.petajournal.types.Request
import com.sosuisha.petajournal.types.Requests
import com.sosuisha.petajournal.types.UiOutput
import com.sosuisha.petajournal.types.ViewModel
import java.io.File

/**
 * The core behind its border of bytes (ADR 024). Events and answers go in and requests come
 * out, as the generated types. The bytes are bincode.
 *
 * The core has the journal inside (ADR 029): [dataDir] keeps its database, its index and its
 * settings, and [markdownDir] is the folder the files from Dropbox are put in. [onRequests]
 * gets what the core asks of the shell. The core may call it from any thread, and for the first
 * time before the constructor returns.
 */
class Border(dataDir: File, markdownDir: File, onRequests: (List<Request>) -> Unit) : AutoCloseable {
    // The types are generated from the Rust that wrote the bytes. Bytes that cannot be read
    // are a build that mixes two versions, and the exception ends the app
    val core = PetaCore(
        dataDir.path,
        markdownDir.path,
        PetaShell { bytes -> onRequests(Requests.bincodeDeserialize(bytes).value) },
    )

    /**
     * Takes in the files of the Markdown folder and deletes the pages of the files that have
     * gone. Answers with the number of files taken in. It can take long.
     */
    fun sync(): Int = core.sync().toInt()

    /**
     * Opens what the URL names, such as `/pages/notes`, and keeps it as the last screen.
     * Answers with the name of the page when the URL names nothing, or a page that is not
     * there, and with `null` when it opened.
     */
    fun open(url: String): String? = missing { core.open(url) }

    /** Opens what the URL names, and makes its page when it is not there. Answers as [open]. */
    fun openOrMake(url: String): String? = missing { core.openOrMake(url) }

    /** The URL of the screen opened last, even before the app was closed. `null` when none. */
    fun lastScreen(): String? = core.lastScreen()

    /** The page that the core could not open, as its error names it. */
    private inline fun missing(opening: () -> Unit): String? =
        try {
            opening()
            null
        } catch (nothing: FfiException) {
            nothing.message
        }

    /** Sends an event of the shell. */
    fun send(event: Event) = core.update(event.bincodeSerialize())

    /** Answers the request with the number [id]. Throws when the core does not wait for it. */
    fun answer(id: UInt, output: UiOutput) = core.resolve(id, output.bincodeSerialize())

    /** The whole view model, to draw after a render request. */
    fun view(): ViewModel = ViewModel.bincodeDeserialize(core.view())

    override fun close() = core.close()
}
