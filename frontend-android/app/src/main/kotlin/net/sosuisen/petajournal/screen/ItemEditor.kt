package net.sosuisen.petajournal.screen

import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.TextRange
import net.sosuisen.petajournal.shell.Shell
import net.sosuisen.petajournal.types.Row

/**
 * The Markdown of the item that is edited (ADR 031). The core has the text: this field shows
 * it, and tells the core of every change.
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
        if (field.composition == null && field.text != body) {
            field = TextFieldValue(body, TextRange(field.selection.start.coerceAtMost(body.length)))
        }
    }
    val asked = shell.view.outline.focusRequest?.takeIf { it.itemId == id }
    LaunchedEffect(asked?.seq) {
        val cursor = asked?.cursor
        if (asked != null && asked.seq != shell.touched && cursor != null) {
            field = field.copy(selection = TextRange(units(field.text, cursor.toInt())))
        }
        focus.requestFocus()
    }
    BasicTextField(
        value = field,
        onValueChange = { next ->
            val enter = enterAt(field, next)
            when {
                // The core splits the item: the field takes no line break. The field tells of
                // a change that was not taken once more, and that is not a second Enter
                enter != null -> if (next.text != refused) {
                    refused = next.text
                    shell.enter(id, characters(field.text, enter))
                }
                else -> {
                    refused = null
                    field = next
                    if (next.composition == null && next.text != body) shell.type(id, next.text)
                }
            }
        },
        textStyle = MaterialTheme.typography.bodyLarge.copy(color = MaterialTheme.colorScheme.onSurface),
        cursorBrush = SolidColor(MaterialTheme.colorScheme.primary),
        modifier = modifier
            .focusRequester(focus)
            .onFocusChanged {
                if (focused && !it.isFocused) shell.finish(id)
                focused = it.isFocused
            }
            .testTag(EDITOR_TAG),
    )
}

/**
 * Where Enter was pressed, as the place in the text before the key, when [next] is [before]
 * with one line break typed at the cursor. `null` for any other change.
 */
private fun enterAt(before: TextFieldValue, next: TextFieldValue): Int? {
    val at = next.selection.start - 1
    val typed = next.composition == null && next.selection.collapsed && next.text.getOrNull(at) == '\n'
    return at.takeIf { typed && next.text.removeRange(at, at + 1) == before.text }
}

/** The core counts a cursor in characters, and a field of Android counts in UTF-16 units. */
private fun characters(text: String, units: Int): Int = text.codePointCount(0, units)

private fun units(text: String, characters: Int): Int =
    text.offsetByCodePoints(0, characters.coerceIn(0, text.codePointCount(0, text.length)))
