package com.sosuisha.petajournal.shell

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.sosuisha.petajournal.core.PetaCore
import com.sosuisha.petajournal.core.PetaScreen
import com.sosuisha.petajournal.types.EditPlace
import com.sosuisha.petajournal.types.EditorKey
import com.sosuisha.petajournal.types.EditorState
import com.sosuisha.petajournal.types.Event
import com.sosuisha.petajournal.types.KeyPress
import com.sosuisha.petajournal.types.Request
import com.sosuisha.petajournal.types.SearchEntry
import com.sosuisha.petajournal.types.SelectionKey
import com.sosuisha.petajournal.types.ShellEffect
import com.sosuisha.petajournal.types.Typing
import com.sosuisha.petajournal.types.UiOp
import com.sosuisha.petajournal.types.UiOutput
import com.sosuisha.petajournal.types.ViewModel
import java.io.File
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors

/**
 * What the app shows, kept up to date with the core. The screen draws [view] and tells this what
 * the user did. All of it runs on the main thread, but the taking in of files.
 *
 * [onMainThread] runs what it is given on the main thread, later and in order. The core calls from
 * its own threads. The core tells the sync when a page changed (ADR 055). [copy] puts text on the
 * clipboard of the system.
 */
class Shell(
    dataDir: File,
    markdownDir: File,
    private val onMainThread: (() -> Unit) -> Unit,
    private val copy: (String) -> Unit = {},
) : AutoCloseable {
    // The main thread keeps the requests in order. The first ones may come before the border
    // is there, and are still handled after this constructor
    private val border =
        Border(dataDir, markdownDir) { requests -> onMainThread { handle(requests) } }

    /**
     * The Markdown that the app put on the clipboard last, `null` before the first copy. The
     * keyboard on the screen may put it in again from what it shows of the clipboard.
     */
    var copied: String? = null
        private set

    /** The core, for the sync to hear from it when a page changed (ADR 055). */
    val core: PetaCore
        get() = border.core

    /** Takes in the files, one sync after the other, and closes the core after the last. */
    private val worker = Executors.newSingleThreadExecutor()
    private var closed = false

    // What the screen draws

    var view: ViewModel by mutableStateOf(firstView(border.changes()))
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
     * True when [restore] opened no screen and was not to make the journal of today. A new app
     * waits for the sign-in: a journal made before the first download would be a second file for a
     * day that Dropbox may hold already.
     */
    var unopened: Boolean by mutableStateOf(false)
        private set

    // The screen that is open. The core decides what opens (ADR 056)

    /**
     * The user touched a link to a page. [url] is the URL of the page, such as `/pages/notes`. A
     * link to a page that is not there opens nothing, and [missing] names the page: a touch by
     * mistake must not put an empty page into Dropbox (ADR 031).
     */
    fun follow(url: String) = told { core.follow(url) }

    /** The user wants the page that [missing] names: it is made and opens. */
    fun make() = told { core.make() }

    /** The user wants the journal of today: it is made when it is not there, and opens. */
    fun today() = told { core.today() }

    /**
     * Opens the screen that the app showed when it was used last. The journal of today opens when
     * there is none, as in a new app, and when its page has gone since. Without [make], the journal
     * of today is not made, and no screen opens then.
     */
    fun restore(make: Boolean = true) = told { core.restore(make) }

    /**
     * Back to the page that was open before, as the core keeps the history (ADR 035). Nothing
     * happens when `view.history.canGoBack` is false.
     */
    fun back() = told { core.back() }

    /** Forward to the page that was open before [back]. */
    fun forward() = told { core.forward() }

    /** Shows what the core told after it was asked to open a screen. */
    private inline fun told(asking: () -> PetaScreen) {
        // A field that loses the focus while the screen is taken down still tells of it
        if (closed) return
        val screen = asking()
        missing = screen.missing
        missingFile = screen.missingFile
        unopened = screen.unopened
        show()
    }

    // The search

    /** The user typed into the search field. */
    fun search(query: String) = send(Event.SearchQueryChanged(query))

    /**
     * The user chose a search result: its page opens. The results for the journal of today and for
     * a new page make their page.
     */
    fun pick(entry: SearchEntry) {
        send(Event.SearchResultClicked(entry.position))
    }

    // The edit of an item

    /**
     * The user touched an item: it is edited (ADR 031). The core ends the edit of another item and
     * a selection (ADR 057). [letter] is how many letters of the shown text of the item stand
     * before the touch, as `RichParagraph.start` counts them: the cursor goes to that place in the
     * Markdown. Without it the cursor is at the end, where the writing goes on.
     */
    fun edit(itemId: String, letter: Int? = null) {
        val place = letter?.let { EditPlace.ShownLetter(it.toULong()) } ?: EditPlace.Unplaced
        send(Event.EditRequested(itemId, place))
    }

    /** The user changed the Markdown of the item. The core saves it a moment later. */
    fun type(itemId: String, body: String) = typed(Typing.Changed(itemId, body))

    /**
     * The user typed into the item, and the cursor is [cursor] UTF-16 units into [body]. The core
     * closes an opening just typed (ADR 057), and saves the body a moment later.
     */
    fun type(itemId: String, body: String, cursor: Int) =
        typed(Typing.Typed(itemId, body, cursor.coerceAtLeast(0).toULong()))

    /**
     * Told when the keyboard composes a word in an item. The core does not hear of the word until
     * it is committed. The sync with the cloud waits for the end of the input (#884).
     */
    var onComposing: () -> Unit = {}

    /** The keyboard composes a word in an item: the user has not ended the input. */
    fun composing() = onComposing()

    /** The core adds the time, and counts the cursor in its own characters. */
    private fun typed(typing: Typing) {
        // As `send`: a field that loses the focus while the screen is taken down still tells
        if (closed) return
        border.typed(typing)
        show()
    }

    /**
     * The user pressed a key in the editor of the item: Enter, Backspace, or an arrow of the cursor
     * pad. The core decides what it means (ADR 052): Enter splits the item, Backspace at the start
     * joins it to the item before, and an arrow at the edge of the text leaves the item. False when
     * the key means nothing to the outline there. [editor] counts in UTF-16 units, as the fields of
     * Android do.
     */
    fun press(key: EditorKey, itemId: String, editor: EditorState): Boolean {
        if (closed || !border.press(KeyPress.Named(itemId, key, editor))) return false
        show()
        return true
    }

    /**
     * The user pasted [text] into the edited item, with the cursor [cursor] UTF-16 units into its
     * Markdown. The core cuts the text into items (ADR 011): its first line goes on after the
     * cursor, and each line after it is an item below. What the app copied comes back as the same
     * items.
     */
    fun paste(itemId: String, text: String, cursor: Int) {
        send(Event.PasteText(itemId, text, "", placeIn(itemId, cursor), false))
    }

    /**
     * The user pasted the image in [file] into the edited item, with the cursor [cursor] UTF-16
     * units into its Markdown. The core saves the image among the assets, deletes [file], and puts
     * a link to the image at the cursor (ADR 071).
     */
    fun pasteImage(itemId: String, file: File, cursor: Int) {
        if (closed) return
        core.pasteImage(itemId, file.path, placeIn(itemId, cursor))
        show()
    }

    /**
     * The place [cursor] UTF-16 units into the Markdown of the item, in the characters of the core.
     */
    private fun placeIn(itemId: String, cursor: Int): ULong {
        val body = bodyOf(itemId).orEmpty()
        return body.codePointCount(0, cursor.coerceIn(0, body.length)).toULong()
    }

    /**
     * The Markdown of the item as the core holds it now, or `null` when the screen shows no such
     * item.
     */
    fun bodyOf(itemId: String): String? =
        view.outline.rows.firstOrNull { it.item.id == itemId }?.item?.body

    /** The user left the editor of the item: what was typed is saved. */
    fun finish(itemId: String) = send(Event.EditFinished(itemId))

    /** Makes the item a child of the item above it. */
    fun indent(itemId: String) = send(Event.Indent(itemId))

    /** Moves the item one level up, out of its parent. */
    fun outdent(itemId: String) = send(Event.Outdent(itemId))

    /** Swaps the item with the one above it on its level. */
    fun moveUp(itemId: String) = send(Event.MoveUp(itemId))

    /** Swaps the item with the one below it on its level. */
    fun moveDown(itemId: String) = send(Event.MoveDown(itemId))

    /** Moves the task on: no task, not started, running, done, and no task again. */
    fun cycleTask(itemId: String) = send(Event.TaskCycle(itemId))

    /** Hides the children of the item, or shows them again. */
    fun toggleCollapsed(itemId: String) = send(Event.CollapseToggle(itemId))

    /**
     * Selects the item with everything under it (ADR 031). The edit of the item has ended: the
     * keyboard is down, and the tools act on the selected items.
     */
    fun select(itemId: String) = send(Event.ShiftSelect(itemId))

    /** The selection ends: no item is selected. */
    fun unselect() = send(Event.SelectionKey(SelectionKey.ESCAPE))

    /** Moves or indents the selected items, as the keys do on a desktop. */
    fun moveSelected(key: SelectionKey) = send(Event.SelectionKey(key))

    /**
     * Puts the selected items on the clipboard, as Markdown. The core keeps how they stood, so that
     * they are pasted back as the same items.
     */
    fun copySelected() = send(Event.SelectionKey(SelectionKey.COPY))

    /** Puts the selected items on the clipboard, as [copySelected] does, and deletes them. */
    fun cutSelected() = send(Event.SelectionKey(SelectionKey.CUT))

    /**
     * The day may have changed while the core could not look at the clock, as when the app was in
     * the back: the journal of today opens the new day at once (#796).
     */
    fun dayMayHaveChanged() = send(Event.DateMayHaveChanged)

    /** Takes back the last change of the page. */
    fun undo() = send(Event.Undo)

    /** Does again the change that [undo] took back. */
    fun redo() = send(Event.Redo)

    // The files of the assets folder

    /**
     * The file of an image of a page. [path] is its path under the Markdown folder, such as
     * `assets/picture.png`. `null` when no download has brought it yet.
     */
    fun image(path: String): File? = core.image(path)?.let(::File)

    /**
     * The user touched a link to a file of the assets folder. [path] is its path under the Markdown
     * folder. Answers with the file to open, or `null` when it is not here: then [missingFile]
     * names it.
     */
    fun attached(path: String): File? {
        val file = core.attached(path)?.let(::File)
        missingFile = core.screen().missingFile
        return file
    }

    // The questions of the core

    /** What the core asked or told while [question] was shown, oldest first. */
    private val later = ArrayDeque<Question>()

    /**
     * The user answered [question]: the core goes on with what it asked about, or leaves it. A
     * notice waits for no answer: the core would take one as a mistake.
     */
    fun answer(yes: Boolean) {
        val asked = question ?: return
        question = later.removeFirstOrNull()
        if (asked.waits) border.answer(asked.id, UiOutput.Bool(yes))
        show()
    }

    /**
     * Shows a question or a notice of the core. The core asks the next question only after this one
     * is answered, but a notice comes at any time: it waits behind what is shown, as the question
     * it took the place of would wait for ever for its answer.
     */
    private fun put(next: Question) {
        if (question == null) question = next else later.addLast(next)
    }

    // The files of the Markdown folder

    /**
     * Takes in the files of the Markdown folder, after a download has changed them, and writes the
     * pages that changed in the app. It runs on a thread of its own: the first sync of a large
     * folder takes long. [then] is called on that thread when the core is done, or at once when the
     * app has closed.
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
     * Does what [sync] does, and returns when the core is done. It is for the thread of a sync with
     * the cloud, which uploads what the core wrote. It must not be the main thread.
     *
     * A new app that waited for the sign-in opens its first screen after it: the download has
     * brought the journal of today, if Dropbox holds one.
     */
    fun syncAndWait() {
        val done = CountDownLatch(1)
        onMainThread { sync(done::countDown) }
        done.await()
        onMainThread { told { core.openFirstScreen() } }
    }

    /**
     * The app goes to the back, where Android may stop it. The edit ends, and the text typed and
     * still waiting for its save is saved before this returns: the moment later, when it would be
     * saved, may never come (#766). The sync after this writes it to its file.
     */
    fun wentToTheBack() {
        if (closed) return
        border.leave()
    }

    /** Lets go of the journal. A sync that runs ends first, on its own thread. */
    override fun close() {
        closed = true
        worker.execute { border.close() }
        worker.shutdown()
    }

    // The events and the requests of the core

    private fun send(event: Event) {
        // A field that loses the focus while the screen is taken down still tells of it
        if (closed) return
        border.send(event)
        // Not left to the render request. It comes a turn of the main thread later, and a
        // text field that reads its old text back after a key loses the key
        show()
    }

    private fun show() {
        view = view.with(border.changes())
    }

    private fun handle(requests: List<Request>) {
        // The app was closed while the requests waited for the main thread
        if (closed) return
        for (request in requests) {
            when (val effect = request.effect) {
                is ShellEffect.Render -> show()
                // The core names what it wants open, as after a search result was chosen
                is ShellEffect.Navigate -> told { core.navigated(effect.value.url) }
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
            // Such as why a page of that name cannot be made
            is UiOp.Alert -> put(Question(op.value, destructive = false, id, waits = false))
            is UiOp.CopyMarkdown -> {
                copied = op.markdown
                copy(op.markdown)
            }
            is UiOp.Prompt,
            is UiOp.ReadClipboardText -> border.answer(id, UiOutput.Text(null))
            // The others wait for no answer, such as the title to set and the item to scroll to
            else -> Unit
        }
    }
}

/**
 * A question of the core with the answers yes and no. [destructive] says that yes loses something,
 * so no is the answer to choose by default. Without [waits] it is a notice: the core waits for no
 * answer, and only OK closes it.
 */
class Question(
    val text: String,
    val destructive: Boolean,
    internal val id: UInt,
    val waits: Boolean = true,
)
