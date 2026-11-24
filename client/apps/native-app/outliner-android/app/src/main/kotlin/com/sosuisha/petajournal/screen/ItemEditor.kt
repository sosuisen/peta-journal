package com.sosuisha.petajournal.screen

import android.content.ClipboardManager
import android.content.ContentResolver
import android.content.Context
import android.net.Uri
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.provider.OpenableColumns
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputConnectionWrapper
import android.view.inputmethod.InputContentInfo
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.contextmenu.builder.item
import androidx.compose.foundation.text.contextmenu.data.TextContextMenuKeys
import androidx.compose.foundation.text.contextmenu.modifier.appendTextContextMenuComponents
import androidx.compose.foundation.text.contextmenu.modifier.filterTextContextMenuComponents
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.isCtrlPressed
import androidx.compose.ui.input.key.isShiftPressed
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.platform.InterceptPlatformTextInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.PlatformTextInputInterceptor
import androidx.compose.ui.platform.PlatformTextInputMethodRequest
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.pasteText
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import com.sosuisha.petajournal.shell.PastedImage
import com.sosuisha.petajournal.shell.Shell
import com.sosuisha.petajournal.types.EditorKey
import com.sosuisha.petajournal.types.EditorState
import com.sosuisha.petajournal.types.Row
import kotlin.concurrent.thread

/**
 * The Markdown of the item that is edited (ADR 031). The core has the text: this field shows it,
 * and tells the core of every change.
 */
@Composable
internal fun ItemEditor(row: Row, shell: Shell, modifier: Modifier) {
    val id = row.item.id
    val body = row.item.body
    var field by remember(id) { mutableStateOf(TextFieldValue(body, TextRange(body.length))) }
    val focus = remember { FocusRequester() }
    var focused by remember { mutableStateOf(false) }
    // Enter in an empty child moves it out and keeps the edit there: the next Enter counts again
    var refused by remember(id, row.item.level) { mutableStateOf<String?>(null) }

    // What the core holds wins, as after an undo. Not while the keyboard composes a word: the
    // core has not heard of that word yet, and the field would lose it
    LaunchedEffect(body) {
        // Not `body`: on a slow phone a held Backspace may come in before this runs, and the
        // field would get back the letter that it just deleted (#918)
        val held = shell.bodyOf(id) ?: body
        if (field.composition == null && field.text != held) {
            field = TextFieldValue(held, TextRange(field.selection.start.coerceAtMost(held.length)))
        }
    }
    val asked = shell.view.outline.focusRequest?.takeIf { it.itemId == id }
    LaunchedEffect(asked?.seq) {
        val cursor = asked?.cursor
        if (asked != null && cursor != null) {
            // A place past the end, as after a move to the end of the item above, is the end.
            // Not `toInt()` alone: it turns a large place into a negative one, the start
            val place = cursor.coerceAtMost(Int.MAX_VALUE.toULong()).toInt()
            field = field.copy(selection = TextRange(units(field.text, place)))
        }
        focus.requestFocus()
    }
    // The lines of the text as the field draws them, for the arrows up and down
    var lines by remember { mutableStateOf<TextLayoutResult?>(null) }
    val caret = LocalCaret.current
    DisposableEffect(caret, id) {
        val move: (Arrow) -> Unit = { arrow ->
            // A word that the keyboard composes ends here, and the core has not heard of it yet
            if (field.composition != null) shell.type(id, field.text)
            val text = field.text
            val from =
                if (arrow == Arrow.Left || arrow == Arrow.Up) field.selection.min
                else field.selection.max
            when (val place = moved(arrow, text, field.selection, lines)) {
                null -> shell.press(arrow.key, id, cursorAt(from, text))
                else -> field = TextFieldValue(text, TextRange(place))
            }
        }
        caret.move = move
        onDispose { if (caret.move === move) caret.move = null }
    }
    val joinsAtStart = { field.selection == TextRange.Zero && field.composition == null }
    val clipboard = LocalContext.current.let { context -> remember(context) { Clipboard(context) } }
    // The core pastes, so that a list becomes items and what the app copied comes back as the
    // same items. Chosen text gives way to the pasted text in the field, as in any field
    val pasteText = { text: String ->
        val chosen = field.selection
        if (chosen.collapsed) {
            // A word that the keyboard composes ends here, and the core has not heard of it yet
            if (field.composition != null) shell.type(id, field.text)
            shell.paste(id, text, chosen.start)
        } else {
            val typed = field.text.replaceRange(chosen.min, chosen.max, text)
            field = TextFieldValue(typed, TextRange(chosen.min + text.length))
            shell.type(id, typed, chosen.min + text.length)
        }
    }
    // An image goes to the core as a file, which it saves among the assets (ADR 071). Its
    // content may come from another app, so it is read away from the main thread
    val context = LocalContext.current
    val pasteImage = { image: Uri, type: String? ->
        // A word that the keyboard composes ends here, and the core has not heard of it yet
        if (field.composition != null) shell.type(id, field.text)
        val cursor = field.selection.min
        val (resolver, cache) = context.contentResolver to context.cacheDir
        thread {
            val file =
                resolver.openInputStream(image)?.let {
                    PastedImage.write(it, cache, resolver.nameOf(image), type)
                }
            if (file != null)
                Handler(Looper.getMainLooper()).post { shell.pasteImage(id, file, cursor) }
        }
        Unit
    }
    val paste = {
        clipboard.image()?.let { (image, type) -> pasteImage(image, type) }
            ?: clipboard.text()?.let(pasteText)
    }
    // The keyboard puts in what it shows of the clipboard as typed text. What the app copied
    // goes through the core, so that it comes back as the same items
    val pastes = { text: CharSequence ->
        (text.toString() == shell.copied).also { if (it) pasteText(text.toString()) }
    }
    // The keyboard on the screen sends no key for Backspace. It asks the field to delete the
    // character before the cursor. At the start there is none: the field changes nothing and
    // tells of nothing. So the screen stands between the keyboard and the field, and hears it
    val keyboard =
        remember(id) {
            PlatformTextInputInterceptor { request, next ->
                next.startInputMethod(
                    PlatformTextInputMethodRequest { attributes ->
                        val connection = request.createInputConnection(attributes)
                        // The keyboard offers the images it has, such as those of its clipboard
                        attributes.contentMimeTypes = arrayOf("image/*")
                        KeyboardOnTheScreen(connection, joinsAtStart, pastes, pasteImage) {
                            shell.press(EditorKey.BACKSPACE, id, cursorAt(0, field.text))
                        }
                    },
                )
            }
        }
    InterceptPlatformTextInput(keyboard) {
        BasicTextField(
            value = field,
            onValueChange = { next ->
                val enter = enterAt(field, next)
                when {
                    // The core splits the item: the field takes no line break. The field tells of
                    // a change that was not taken once more, and that is not a second Enter
                    enter != null ->
                        if (next.text != refused) {
                            refused = next.text
                            shell.press(EditorKey.ENTER, id, cursorAt(enter, field.text))
                        }
                    else -> {
                        refused = null
                        field = next
                        when (telling(next, body)) {
                            Telling.Composing -> shell.composing()
                            Telling.Typed -> shell.type(id, next.text, next.selection.start)
                            null -> {}
                        }
                    }
                }
            },
            textStyle =
                MaterialTheme.typography.bodyLarge.copy(
                    color = MaterialTheme.colorScheme.onSurface,
                ),
            cursorBrush = SolidColor(MaterialTheme.colorScheme.primary),
            onTextLayout = { lines = it },
            modifier =
                modifier
                    // A keyboard with real keys sends the key. The core joins the item to the one
                    // before
                    .onPreviewKeyEvent { pressed ->
                        val joins = pressed.key == Key.Backspace && joinsAtStart()
                        if (joins && pressed.type == KeyEventType.KeyDown)
                            shell.press(EditorKey.BACKSPACE, id, cursorAt(0, field.text))
                        joins
                    }
                    // Ctrl+V of a keyboard with real keys, Paste of the menu of the text, and the
                    // paste of a service of accessibility all paste through the core
                    .onPreviewKeyEvent { pressed ->
                        val ctrlV =
                            pressed.key == Key.V && pressed.isCtrlPressed && !pressed.isShiftPressed
                        if (ctrlV && pressed.type == KeyEventType.KeyDown) paste()
                        ctrlV
                    }
                    // The filter also runs on the items that the app adds: its own Paste has a key
                    // of its own
                    .filterTextContextMenuComponents { it.key != TextContextMenuKeys.PasteKey }
                    .appendTextContextMenuComponents {
                        if (clipboard.hasText()) {
                            item(CorePasteKey, label = clipboard.pasteLabel) {
                                paste()
                                close()
                            }
                        }
                    }
                    .semantics {
                        pasteText {
                            paste()
                            true
                        }
                    }
                    .focusRequester(focus)
                    .onFocusChanged {
                        if (focused && !it.isFocused) shell.finish(id)
                        focused = it.isFocused
                    }
                    .testTag(EDITOR_TAG),
        )
    }
}

/**
 * The keyboard on the screen as the field hears it, with two changes. A Backspace at the start is
 * not for the field: [atStart] says whether the cursor is at the start, with no text chosen and no
 * word being composed, and [join] is called then. Text that the keyboard puts in goes to [pastes]
 * first, which tells whether it pasted the text. An image that the keyboard puts in goes to
 * [pastesImage]. The field hears nothing of what was taken.
 */
private class KeyboardOnTheScreen(
    keyboard: InputConnection,
    private val atStart: () -> Boolean,
    private val pastes: (CharSequence) -> Boolean,
    private val pastesImage: (Uri, String?) -> Unit,
    private val join: () -> Unit,
) : InputConnectionWrapper(keyboard, false) {
    override fun commitContent(content: InputContentInfo, flags: Int, opts: Bundle?): Boolean {
        val description = content.description
        if (!description.hasMimeType("image/*")) return super.commitContent(content, flags, opts)
        // The keyboard lends the image only for as long as it is asked to
        if (flags and INPUT_CONTENT_GRANT_READ_URI_PERMISSION != 0) content.requestPermission()
        pastesImage(content.contentUri, description.getMimeType(0))
        return true
    }

    override fun deleteSurroundingText(before: Int, after: Int): Boolean =
        joined(before, after) || super.deleteSurroundingText(before, after)

    override fun deleteSurroundingTextInCodePoints(before: Int, after: Int): Boolean =
        joined(before, after) || super.deleteSurroundingTextInCodePoints(before, after)

    override fun commitText(text: CharSequence?, newCursorPosition: Int): Boolean =
        (text != null && pastes(text)) || super.commitText(text, newCursorPosition)

    private fun joined(before: Int, after: Int): Boolean {
        val backspace = before > 0 && after == 0 && atStart()
        if (backspace) join()
        return backspace
    }
}

/** The key of the Paste of the menu of the text that pastes through the core. */
private object CorePasteKey

/** The clipboard of the system, as the field pastes from it. */
private class Clipboard(private val context: Context) {
    private val manager = context.getSystemService(ClipboardManager::class.java)

    /** The word of the system for Paste, as the menu of any field shows it. */
    val pasteLabel: String = context.getString(android.R.string.paste)

    /**
     * Whether there is something to paste. It does not read the clipboard, which the system tells
     * the user of.
     */
    fun hasText(): Boolean = manager.hasPrimaryClip()

    fun text(): String? =
        manager.primaryClip
            ?.takeIf { it.itemCount > 0 }
            ?.getItemAt(0)
            ?.coerceToText(context)
            ?.toString()

    /**
     * The image on the clipboard and its media type, such as `image/png`, or `null` when it holds
     * none. The type is that of the clip: the app that lent the image may tell this app nothing.
     */
    fun image(): Pair<Uri, String>? {
        val clip = manager.primaryClip?.takeIf { it.itemCount > 0 } ?: return null
        val description = clip.description
        val type =
            (0 until description.mimeTypeCount).map(description::getMimeType).firstOrNull {
                it.startsWith("image/")
            }
        val uri = clip.getItemAt(0).uri
        return if (type != null && uri != null) uri to type else null
    }
}

/** What the editor tells the shell of a change of its field. */
internal enum class Telling {
    /** The keyboard composes a word. The core hears of it once it is committed. */
    Composing,

    /** The text changed, and the core saves it. */
    Typed,
}

/** What to tell the shell of [next], when the core holds [body]. `null` when the text stays. */
internal fun telling(next: TextFieldValue, body: String): Telling? =
    when {
        next.composition != null -> Telling.Composing
        next.text != body -> Telling.Typed
        else -> null
    }

/** The name that the content at [uri] came with, or `null` when its app tells none. */
private fun ContentResolver.nameOf(uri: Uri): String? = runCatching {
    query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { row ->
        if (row.moveToFirst()) row.getString(0) else null
    }
}
    .getOrNull()

/**
 * Where Enter was pressed, as the place in the text before the key, when [next] is [before] with
 * one line break typed at the cursor. `null` for any other change.
 */
private fun enterAt(before: TextFieldValue, next: TextFieldValue): Int? {
    val at = next.selection.start - 1
    val typed =
        next.composition == null && next.selection.collapsed && next.text.getOrNull(at) == '\n'
    return at.takeIf { typed && next.text.removeRange(at, at + 1) == before.text }
}

/**
 * Where the cursor goes in [text] for [arrow], from [selection], in UTF-16 units. `null` when it
 * leaves the item: at the start for Left, on the first line for Up, at the end for Right, and on
 * the last line for Down. Chosen text gives way to a cursor at its start or its end.
 */
private fun moved(
    arrow: Arrow,
    text: String,
    selection: TextRange,
    lines: TextLayoutResult?,
): Int? {
    if (!selection.collapsed && (arrow == Arrow.Left || arrow == Arrow.Right)) {
        return if (arrow == Arrow.Left) selection.min else selection.max
    }
    val at = if (arrow == Arrow.Left || arrow == Arrow.Up) selection.min else selection.max
    return when (arrow) {
        Arrow.Left -> if (at == 0) null else text.offsetByCodePoints(at, -1)
        Arrow.Right -> if (at == text.length) null else text.offsetByCodePoints(at, 1)
        Arrow.Up -> lines?.onLine(at, -1)
        Arrow.Down -> lines?.onLine(at, 1)
    }
}

/**
 * The place on the line [by] lines away from the line of [offset], under the same spot of the
 * screen. `null` when there is no such line.
 */
private fun TextLayoutResult.onLine(offset: Int, by: Int): Int? {
    val line = getLineForOffset(offset) + by
    if (line !in 0 until lineCount) return null
    val x = getHorizontalPosition(offset, usePrimaryDirection = true)
    val y = (getLineTop(line) + getLineBottom(line)) / 2
    return getOffsetForPosition(Offset(x, y))
}

/** The key of the arrow, for the core, which moves the cursor to the next item (ADR 052). */
private val Arrow.key: EditorKey
    get() =
        when (this) {
            Arrow.Left -> EditorKey.LEFT
            Arrow.Up -> EditorKey.UP
            Arrow.Right -> EditorKey.RIGHT
            Arrow.Down -> EditorKey.DOWN
        }

/**
 * A field with a cursor [at] UTF-16 units into [text], and nothing chosen. The cursor is on the
 * first and the last line: the field only hands a key to the core at the edge of its text.
 */
private fun cursorAt(at: Int, text: String) =
    EditorState(text, at.toULong(), at.toULong(), onFirstLine = true, onLastLine = true)

/** The place in UTF-16 units, as a field of Android counts, of a place in characters. */
private fun units(text: String, characters: Int): Int =
    text.offsetByCodePoints(0, characters.coerceIn(0, text.codePointCount(0, text.length)))
