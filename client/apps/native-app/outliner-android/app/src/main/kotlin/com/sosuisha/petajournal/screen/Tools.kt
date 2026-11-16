package com.sosuisha.petajournal.screen

import androidx.annotation.DrawableRes
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Alignment
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.disabled
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.sosuisha.petajournal.R
import com.sosuisha.petajournal.shell.Shell
import com.sosuisha.petajournal.types.Row as OutlineRow
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * The row of tools at the bottom of the screen (ADR 031). A phone has no Tab key, no key with
 * Alt and no arrow keys. Two pads of four arrows take their place: one moves the item, the
 * other the cursor. [edited] is the row of the item that is edited, and `null` when none is:
 * the tools that change an item cannot be touched then. A touch on a tool leaves the focus in
 * the field.
 */
@Composable
internal fun Tools(edited: OutlineRow?, shell: Shell) {
    val focus = LocalFocusManager.current
    val caret = LocalCaret.current
    val itemId = edited?.item?.id
    // Never scrolled: the pads are always where the thumb looks for them
    Surface(tonalElevation = 3.dp) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceEvenly, verticalAlignment = Alignment.CenterVertically) {
            val parent = edited?.hasChildren == true
            val collapsed = edited?.isCollapsed == true
            val fold = if (collapsed) R.drawable.ic_expand else R.drawable.ic_collapse
            Tool(if (collapsed) "Expand" else "Collapse", fold, { itemId?.let(shell::toggleCollapsed) }, enabled = parent)
            Pad("Move block", R.drawable.ic_move_block, BLOCK_MOVES, enabled = itemId != null, { arrow ->
                val id = itemId ?: return@Pad
                when (arrow) {
                    Arrow.Left -> shell.outdent(id)
                    Arrow.Right -> shell.indent(id)
                    Arrow.Up -> shell.moveUp(id)
                    Arrow.Down -> shell.moveDown(id)
                }
            })
            Pad("Move cursor", R.drawable.ic_move_cursor, CURSOR_MOVES, enabled = itemId != null, { caret.move?.invoke(it) })
            Tool("Task", R.drawable.ic_task, { itemId?.let(shell::cycleTask) }, enabled = itemId != null)
            Tool("Undo", R.drawable.ic_undo, shell::undo)
            Tool("Redo", R.drawable.ic_redo, shell::redo)
            // The field loses the focus, and tells the core that the edit has ended
            TextButton({ focus.clearFocus() }, enabled = itemId != null, contentPadding = PaddingValues(horizontal = 6.dp)) { Text("Done", maxLines = 1, softWrap = false) }
        }
    }
}

/** What each arrow of the pad that moves the item does, as a screen reader says it. */
private val BLOCK_MOVES = mapOf(
    Arrow.Left to "Outdent",
    Arrow.Up to "Move up",
    Arrow.Down to "Move down",
    Arrow.Right to "Indent",
)

/** What each arrow of the pad that moves the cursor does, as a screen reader says it. */
private val CURSOR_MOVES = mapOf(
    Arrow.Left to "Cursor left",
    Arrow.Up to "Cursor up",
    Arrow.Down to "Cursor down",
    Arrow.Right to "Cursor right",
)

/** The colour of the face of a tool, faint when it cannot be touched. */
@Composable
private fun ink(enabled: Boolean): Color =
    if (enabled) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface.copy(alpha = 0.38f)

/**
 * A tool with an icon for its face. [name] says what it does, for a screen reader. [icon] is
 * one of the icons of `client/assets/icons`.
 */
@Composable
private fun Tool(name: String, @DrawableRes icon: Int, onTouch: () -> Unit, enabled: Boolean = true) {
    TextButton(onTouch, Modifier.semantics { contentDescription = name }, enabled = enabled, contentPadding = PaddingValues(8.dp)) {
        Icon(painterResource(icon), contentDescription = null, Modifier.size(28.dp), tint = ink(enabled))
    }
}

/**
 * A pad of four arrows. The whole pad takes touches: the lines from its corners cut it into
 * the parts of the arrows (see [aim]). A touch that is held goes on doing what the arrow does.
 * [icon] is its face. [moves] names what each arrow does, for a screen reader.
 */
@Composable
private fun Pad(
    name: String,
    @DrawableRes icon: Int,
    moves: Map<Arrow, String>,
    enabled: Boolean,
    onArrow: (Arrow) -> Unit,
) {
    val scope = rememberCoroutineScope()
    // The pointer input lives on while the item that is edited changes
    val act by rememberUpdatedState(onArrow)
    val touches = Modifier.pointerInput(enabled) {
        if (!enabled) return@pointerInput
        awaitEachGesture {
            val down = awaitFirstDown()
            val arrow = aim(down.position.x, down.position.y, size.width.toFloat(), size.height.toFloat())
                ?: return@awaitEachGesture
            down.consume()
            val held = scope.launch {
                act(arrow)
                delay(HOLD_MS)
                while (true) {
                    act(arrow)
                    delay(REPEAT_MS)
                }
            }
            waitForUpOrCancellation()
            held.cancel()
        }
    }
    val described = Modifier.semantics {
        contentDescription = name
        if (enabled) {
            customActions = moves.map { (arrow, said) -> CustomAccessibilityAction(said) { act(arrow); true } }
        } else {
            disabled()
        }
    }
    Box(described.then(touches).size(width = 68.dp, height = 56.dp), contentAlignment = Alignment.Center) {
        Icon(painterResource(icon), contentDescription = null, Modifier.size(44.dp), tint = ink(enabled))
    }
}

/** How long a touch is held before an arrow of a pad repeats. */
private const val HOLD_MS = 400L

/** How often an arrow of a pad repeats while the touch is held. */
private const val REPEAT_MS = 100L
