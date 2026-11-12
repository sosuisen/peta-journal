package net.sosuisen.petajournal

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import net.sosuisen.petajournal.core.FfiException
import net.sosuisen.petajournal.sync.findWithoutCase
import net.sosuisen.petajournal.types.Event
import net.sosuisen.petajournal.types.Request
import net.sosuisen.petajournal.types.SearchEntry
import net.sosuisen.petajournal.types.ShellEffect
import net.sosuisen.petajournal.types.UiOp
import net.sosuisen.petajournal.types.UiOutput
import net.sosuisen.petajournal.types.ViewModel
import java.io.File
import java.util.concurrent.CountDownLatch
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
    private val markdownDir: File,
    private val onMainThread: (() -> Unit) -> Unit,
) : AutoCloseable {
    /**
     * The file of an image of a page. [path] is its path under the Markdown folder, such as
     * `assets/picture.png`. `null` when no download has brought it yet.
     */
    fun image(path: String): File? = findWithoutCase(markdownDir, path)?.takeIf { it.isFile }

    /** The file that a link named and that is not here, until the next link is touched. */
    var missingFile: String? by mutableStateOf(null)
        private set

    /**
     * The user touched a link to a file of the assets folder. [path] is its path under the
     * Markdown folder. Answers with the file to open, or `null` when it is not here.
     */
    fun attached(path: String): File? {
        val file = image(path)
        missingFile = path.takeIf { file == null }
        return file
    }

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
     * Takes in the files of the Markdown folder, after a download has changed them, and writes
     * the pages that changed in the app. It runs on a thread of its own: the first sync of a
     * large folder takes long. [then] is called on that thread when the core is done, or at
     * once when the app has closed.
     */
    fun sync(then: () -> Unit = {}) {
        if (closed) return then()
        syncing = true
        worker.execute {
            border.sync()
            then()
            onMainThread {
                if (!closed) {
                    syncing = false
                    show()
                }
            }
        }
    }

    /**
     * Does what [sync] does, and returns when the core is done. It is for the thread of a sync
     * with the cloud, which uploads what the core wrote. It must not be the main thread.
     */
    fun syncAndWait() {
        val done = CountDownLatch(1)
        onMainThread { sync(done::countDown) }
        done.await()
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
        missingFile = null
        missing = try {
            border.open(url)
            null
        } catch (nothing: FfiException) {
            url.removePrefix(PAGES).substringBefore('#')
        }
        show()
    }

    /**
     * The focus request that a touch on an item made. The core asks for the cursor at the
     * start of the item then. A finger shows no place in the Markdown, so the screen puts the
     * cursor at the end instead, where the writing goes on.
     */
    var touched: ULong? = null
        private set

    /** The user touched an item: it is edited (ADR 031). The edit of another item ends. */
    fun edit(itemId: String) {
        view.outline.focusedItemId?.takeIf { it != itemId }?.let(::finish)
        send(Event.EditRequested(itemId, 0uL))
        touched = view.outline.focusRequest?.seq
    }

    /** The user changed the Markdown of the item. The core saves it a moment later. */
    fun type(itemId: String, body: String) = send(Event.BodyChanged(itemId, body, now()))

    /**
     * The user pressed Enter in the item. [cursor] counts the characters before it, as the
     * core counts them. The core splits the item there, and the edit goes on in the new one.
     */
    fun enter(itemId: String, cursor: Int) = send(Event.EnterPressed(itemId, cursor.toULong(), now()))

    /** The user left the editor of the item: what was typed is saved. */
    fun finish(itemId: String) = send(Event.EditFinished(itemId))

    private fun now(): ULong = System.currentTimeMillis().toULong()

    /** The user typed into the search field. */
    fun search(query: String) = send(Event.SearchQueryChanged(query))

    /** The user chose a search result: its page opens. */
    fun pick(entry: SearchEntry) = send(Event.SearchResultClicked(entry.position))

    private fun send(event: Event) {
        // A field that loses the focus while the screen is taken down still tells of it
        if (closed) return
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
                is ShellEffect.Ui -> ask(request.id, effect.value)
            }
        }
    }

    /** Does what the core asks of the screen, and answers when the core waits for an answer. */
    private fun ask(id: UInt, op: UiOp) {
        when (op) {
            // The row that is edited takes the focus by itself when it is drawn. The core
            // waits for this answer before it ends a move or a split
            is UiOp.FocusEditor -> {
                show()
                border.answer(id, UiOutput.Done)
            }
            // No dialog yet: what is hard to undo is not done
            is UiOp.Confirm, is UiOp.ConfirmDestructive -> border.answer(id, UiOutput.Bool(false))
            is UiOp.Prompt, is UiOp.ReadClipboardText -> border.answer(id, UiOutput.Text(null))
            // The others wait for no answer, such as the title to set and the item to scroll to
            else -> Unit
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
