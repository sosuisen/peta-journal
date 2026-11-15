package com.sosuisha.petajournal.shell

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.sosuisha.petajournal.core.closedAsTyped
import com.sosuisha.petajournal.sync.findWithoutCase
import com.sosuisha.petajournal.types.Event
import com.sosuisha.petajournal.types.Request
import com.sosuisha.petajournal.types.SearchEntry
import com.sosuisha.petajournal.types.SearchKind
import com.sosuisha.petajournal.types.ShellEffect
import com.sosuisha.petajournal.types.UiOp
import com.sosuisha.petajournal.types.UiOutput
import com.sosuisha.petajournal.types.ViewModel
import java.io.File
import java.time.LocalDate
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors

/**
 * What the app shows, kept up to date with the core. The screen draws [view] and tells this
 * what the user did. All of it runs on the main thread, but the taking in of files.
 *
 * [onMainThread] runs what it is given on the main thread, later and in order. The core calls
 * from its own threads. [onEdit] is called each time the user changes a page: the app syncs
 * a moment after the last change (ADR 031).
 */
class Shell(
    dataDir: File,
    private val markdownDir: File,
    private val onMainThread: (() -> Unit) -> Unit,
    private val onEdit: () -> Unit = {},
) : AutoCloseable {
    // The main thread keeps the requests in order. The first ones may come before the border
    // is there, and are still handled after this constructor
    private val border = Border(dataDir, markdownDir) { requests -> onMainThread { handle(requests) } }

    /** Takes in the files, one sync after the other, and closes the core after the last. */
    private val worker = Executors.newSingleThreadExecutor()
    private var closed = false

    // What the screen draws

    var view: ViewModel by mutableStateOf(border.view())
        private set

    /** True while files are taken in. The pages of an earlier sync can be read meanwhile. */
    var syncing: Boolean by mutableStateOf(false)
        private set

    /** The page that a link named and that is not there, until another page opens. */
    var missing: String? by mutableStateOf(null)
        private set

    /** The file that a link named and that is not here, until the next link is touched. */
    var missingFile: String? by mutableStateOf(null)
        private set

    /** What the core asks the user before it does what is hard to undo, or null. */
    var question: Question? by mutableStateOf(null)
        private set

    /**
     * True when [restore] opened no screen and was not to make the journal of today. A new
     * app waits for the sign-in: a journal made before the first download would be a second
     * file for a day that Dropbox may hold already.
     */
    var unopened: Boolean by mutableStateOf(false)
        private set

    /**
     * The focus request that a touch beside the text of an item made. The core asks for the
     * cursor at the start of the item then. Such a touch shows no place in the Markdown, so the
     * screen puts the cursor at the end instead, where the writing goes on.
     */
    var touched: ULong? = null
        private set

    // The screen that is open

    /**
     * The user touched a link to a page. [url] is the URL of the page, such as `/pages/notes`.
     * A link to a page that is not there opens nothing, and [missing] names the page: a touch
     * by mistake must not put an empty page into Dropbox (ADR 031).
     */
    fun follow(url: String) = open(url, make = false)

    /** The user wants the page that [missing] names: it is made and opens. */
    fun make() {
        val page = missing ?: return
        open(PAGES + page, make = true)
        onEdit()
    }

    /** The user wants the journal of today: it is made when it is not there, and opens. */
    fun today() {
        open(TODAY, make = true)
        onEdit()
    }

    /**
     * Opens the screen that the app showed when it was used last. The journal of today opens
     * when there is none, as in a new app, and when its page has gone since. Without [make],
     * the journal of today is not made, and no screen opens then.
     */
    fun restore(make: Boolean = true) {
        val before = border.lastScreen()
        if (before != null) open(before, make = false)
        val none = before == null || missing != null
        if (none && make) today()
        unopened = none && !make
    }

    private fun open(url: String, make: Boolean) {
        missingFile = null
        missing = if (make) border.openOrMake(url) else border.open(url)
        if (missing == null) unopened = false
        show()
    }

    /**
     * Back to the page that was open before, as the core keeps the history (ADR 035). Nothing
     * happens when `view.history.canGoBack` is false.
     */
    fun back() = turn(Event.Back(date()))

    /** Forward to the page that was open before [back]. */
    fun forward() = turn(Event.Forward(date()))

    private fun turn(event: Event) {
        // The pages of the history opened before, so what the last one told is gone
        missing = null
        missingFile = null
        send(event)
    }

    // The search

    /** The user typed into the search field. */
    fun search(query: String) = send(Event.SearchQueryChanged(query))

    /**
     * The user chose a search result: its page opens. The results for the journal of today
     * and for a new page make their page.
     */
    fun pick(entry: SearchEntry) {
        send(Event.SearchResultClicked(entry.position))
        if (entry.kind == SearchKind.TODAY || entry.kind == SearchKind.CREATE) onEdit()
    }

    // The edit of an item

    /**
     * The user touched an item: it is edited (ADR 031). The edit of another item ends.
     * [letter] is how many letters of the shown text of the item stand before the touch, as
     * `RichParagraph.start` counts them: the cursor goes to that place in the Markdown. Without
     * it the cursor is at the end, where the writing goes on.
     */
    fun edit(itemId: String, letter: Int? = null) {
        view.outline.focusedItemId?.takeIf { it != itemId }?.let(::finish)
        if (letter != null) {
            send(Event.ShownTextTouched(itemId, letter.toULong()))
            touched = null
        } else {
            send(Event.EditRequested(itemId, 0uL))
            touched = view.outline.focusRequest?.seq
        }
    }

    /** The user changed the Markdown of the item. The core saves it a moment later. */
    fun type(itemId: String, body: String) = change(Event.BodyChanged(itemId, body, now()))

    /**
     * The user changed the Markdown of the item, and the cursor is [cursor] UTF-16 units into
     * [body]. An opening just typed gets its closing (ADR 050). An opening that is left after a
     * deletion does not, or the closing would come back at every delete.
     */
    fun type(itemId: String, body: String, cursor: Int) {
        val before = view.outline.rows.firstOrNull { it.item.id == itemId }?.item?.body
        val added = body.length > (before?.length ?: 0)
        val closed = if (added) closedAsTyped(body, cursor.toULong()) else null
        type(itemId, closed ?: body)
    }

    /**
     * The user pressed Enter in the item. [cursor] counts the characters before it, as the
     * core counts them. The core splits the item there, and the edit goes on in the new one.
     */
    fun enter(itemId: String, cursor: Int) = change(Event.EnterPressed(itemId, cursor.toULong(), now()))

    /**
     * The user pressed Backspace at the start of the item. The core joins the item to the one
     * before it, and the edit goes on there, where the two texts meet. Nothing happens in the
     * first item of a page.
     */
    fun backspaceAtStart(itemId: String) = change(Event.BackspaceAtStart(itemId))

    /**
     * The cursor leaves the item for the one above it, at the place [cursor] in characters,
     * as the core counts them. A place past the end of that item puts the cursor at its end.
     * Nothing happens in the first item of a page.
     */
    fun navigateUp(itemId: String, cursor: Long) = send(Event.NavigateUp(itemId, cursor.toULong()))

    /**
     * The cursor leaves the item for the one below it, at the place [cursor] in characters.
     * Nothing happens in the last item of a page.
     */
    fun navigateDown(itemId: String, cursor: Long) = send(Event.NavigateDown(itemId, cursor.toULong()))

    /** The user left the editor of the item: what was typed is saved. */
    fun finish(itemId: String) = send(Event.EditFinished(itemId))

    private fun now(): ULong = System.currentTimeMillis().toULong()

    /** The local date, `YYYY-MM-DD`, for a journal of today that the history opens. */
    private fun date(): String = LocalDate.now().toString()

    /** Makes the item a child of the item above it. */
    fun indent(itemId: String) = change(Event.Indent(itemId))

    /** Moves the item one level up, out of its parent. */
    fun outdent(itemId: String) = change(Event.Outdent(itemId))

    /** Swaps the item with the one above it on its level. */
    fun moveUp(itemId: String) = change(Event.MoveUp(itemId))

    /** Swaps the item with the one below it on its level. */
    fun moveDown(itemId: String) = change(Event.MoveDown(itemId))

    /** Moves the task on: no task, not started, running, done, and no task again. */
    fun cycleTask(itemId: String) = change(Event.TaskCycle(itemId))

    /** Deletes the item. */
    fun delete(itemId: String) = change(Event.DeleteItem(itemId))

    // The files of the assets folder

    /**
     * The file of an image of a page. [path] is its path under the Markdown folder, such as
     * `assets/picture.png`. `null` when no download has brought it yet.
     */
    fun image(path: String): File? = findWithoutCase(markdownDir, path)?.takeIf { it.isFile }

    /**
     * The user touched a link to a file of the assets folder. [path] is its path under the
     * Markdown folder. Answers with the file to open, or `null` when it is not here.
     */
    fun attached(path: String): File? {
        val file = image(path)
        missingFile = path.takeIf { file == null }
        return file
    }

    // The questions of the core

    /** The user answered [question]: the core goes on with what it asked about, or leaves it. */
    fun answer(yes: Boolean) {
        val asked = question ?: return
        question = null
        border.answer(asked.id, UiOutput.Bool(yes))
        show()
    }

    /** Shows a question of the core. The core asks the next one only after this is answered. */
    private fun put(next: Question) {
        question = next
    }

    // The files of the Markdown folder

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
     *
     * A new app that waited for the sign-in opens its first screen after it: the download
     * has brought the journal of today, if Dropbox holds one.
     */
    fun syncAndWait() {
        val done = CountDownLatch(1)
        onMainThread { sync(done::countDown) }
        done.await()
        onMainThread { if (unopened && !closed) restore() }
    }

    /** Lets go of the journal. A sync that runs ends first, on its own thread. */
    override fun close() {
        closed = true
        worker.execute { border.close() }
        worker.shutdown()
    }

    // The events and the requests of the core

    /** Sends an event that changes a page. */
    private fun change(event: Event) {
        send(event)
        onEdit()
    }

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
                    // The core names what it wants open, as after a search result was chosen.
                    // The user asked for it, so a page that is not there is made
                    open(effect.value.url, make = true)
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
            is UiOp.Confirm -> put(Question(op.value, destructive = false, id))
            is UiOp.ConfirmDestructive -> put(Question(op.value, destructive = true, id))
            is UiOp.Prompt, is UiOp.ReadClipboardText -> border.answer(id, UiOutput.Text(null))
            // The others wait for no answer, such as the title to set and the item to scroll to
            else -> Unit
        }
    }

    companion object {
        /** What the URL of a page starts with, before the path of the page. */
        const val PAGES = "/pages/"

        /** The URL of the journal of today, whatever the date is. */
        const val TODAY = "/today"
    }
}

/**
 * A question of the core with the answers yes and no. [destructive] says that yes loses
 * something, so no is the answer to choose by default.
 */
class Question(val text: String, val destructive: Boolean, internal val id: UInt)
