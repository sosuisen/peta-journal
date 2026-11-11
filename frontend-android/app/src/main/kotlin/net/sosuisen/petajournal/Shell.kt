package net.sosuisen.petajournal

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import net.sosuisen.petajournal.core.FfiException
import net.sosuisen.petajournal.types.Event
import net.sosuisen.petajournal.types.Request
import net.sosuisen.petajournal.types.SearchEntry
import net.sosuisen.petajournal.types.ShellEffect
import net.sosuisen.petajournal.types.ViewModel
import java.io.File
import java.util.concurrent.Executors

/**
 * What the app shows, kept up to date with the core. The screen draws [view] and tells this
 * what the user did. All of it runs on the main thread, but the taking in of files.
 *
 * [onMainThread] runs what it is given on the main thread, later and in order. The core calls
 * from its own threads.
 */
class Shell(
    dataDir: File,
    markdownDir: File,
    private val onMainThread: (() -> Unit) -> Unit,
) : AutoCloseable {
    // The main thread keeps the requests in order. The first ones may come before the border
    // is there, and are still handled after this constructor
    private val border = Border(dataDir, markdownDir) { requests -> onMainThread { handle(requests) } }

    /** Takes in the files, one sync after the other, and closes the core after the last. */
    private val worker = Executors.newSingleThreadExecutor()
    private var closed = false

    var view: ViewModel by mutableStateOf(border.view())
        private set

    /** True while files are taken in. The pages of an earlier sync can be read meanwhile. */
    var syncing: Boolean by mutableStateOf(false)
        private set

    /**
     * Takes in the files of the Markdown folder, after a download has changed them. It runs
     * on a thread of its own: the first sync of a large folder takes long.
     */
    fun sync() {
        syncing = true
        worker.execute {
            border.sync()
            onMainThread {
                if (!closed) {
                    syncing = false
                    show()
                }
            }
        }
    }

    /** The page that a link named and that is not there, until another page opens. */
    var missing: String? by mutableStateOf(null)
        private set

    /**
     * The user touched a link to a page. [url] is the URL of the page, such as `/pages/notes`.
     * A link to a page that is not there opens nothing: the first version makes no page.
     */
    fun follow(url: String) = open(url)

    private fun open(url: String) {
        missing = try {
            border.open(url)
            null
        } catch (nothing: FfiException) {
            url.removePrefix(PAGES).substringBefore('#')
        }
        show()
    }

    /** The user typed into the search field. */
    fun search(query: String) = send(Event.SearchQueryChanged(query))

    /** The user chose a search result: its page opens. */
    fun pick(entry: SearchEntry) = send(Event.SearchResultClicked(entry.position))

    private fun send(event: Event) {
        border.send(event)
        // Not left to the render request. It comes a turn of the main thread later, and a
        // text field that reads its old text back after a key loses the key
        show()
    }

    private fun show() {
        view = border.view()
    }

    private fun handle(requests: List<Request>) {
        // The app was closed while the requests waited for the main thread
        if (closed) return
        for (request in requests) {
            when (val effect = request.effect) {
                is ShellEffect.Render -> show()
                is ShellEffect.Navigate -> {
                    // The core names what it wants open, as after a search result was chosen
                    open(effect.value.url)
                }
                // What the first screen is asked waits for no answer: the title to set, and
                // the item to scroll to
                is ShellEffect.Ui -> Unit
            }
        }
    }

    /** Lets go of the journal. A sync that runs ends first, on its own thread. */
    override fun close() {
        closed = true
        worker.execute { border.close() }
        worker.shutdown()
    }

    companion object {
        /** What the URL of a page starts with, before the path of the page. */
        const val PAGES = "/pages/"
    }
}
