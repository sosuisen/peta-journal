package com.sosuisha.petajournal.screen

import androidx.annotation.DrawableRes
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.sosuisha.petajournal.R
import com.sosuisha.petajournal.shell.Shell
import com.sosuisha.petajournal.types.Row as OutlineRow
import com.sosuisha.petajournal.types.SelectionKey
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * The tools at the bottom of the screen, while an item is edited or selected (ADR 031). A phone has
 * no Tab key, no key with Alt and no arrow keys. Two pads of four arrows take their place: one
 * moves the item, the other the cursor. While items are selected, the second pad chooses the item
 * above or below instead. The tools stand in two rows: the pads take both. [target] is the row the
 * tools act on: the one that is edited, or the first that is selected. [rows] are all the rows of
 * the page. A touch on a tool leaves the focus in the field.
 */
@Composable
internal fun Tools(target: OutlineRow, rows: List<OutlineRow>, shell: Shell, onShrink: () -> Unit) {
    val focus = LocalFocusManager.current
    val caret = LocalCaret.current
    val itemId = target.item.id
    val editing = target.isEditing
    // Never scrolled: the pads are always where the thumb looks for them
    Surface(tonalElevation = 3.dp) {
        Row(
            Modifier.fillMaxWidth().height(TOOLS_HEIGHT),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column {
                val collapsed = target.isCollapsed
                val fold = if (collapsed) R.drawable.ic_expand else R.drawable.ic_collapse
                Tool(
                    if (collapsed) "Expand" else "Collapse",
                    fold,
                    { shell.toggleCollapsed(itemId) },
                    enabled = target.hasChildren,
                )
                Tool("Task", R.drawable.ic_task, { shell.cycleTask(itemId) })
            }
            Pad("Move block", R.drawable.ic_move_block, BLOCK_MOVES, Modifier.weight(1f)) { arrow ->
                if (editing) move(shell, itemId, arrow) else moveSelected(shell, arrow)
            }
            if (editing) {
                Pad("Move cursor", R.drawable.ic_move_cursor, CURSOR_MOVES, Modifier.weight(1f)) {
                    caret.move?.invoke(it)
                }
            } else {
                Pad(
                    "Move selection",
                    R.drawable.ic_move_selection,
                    SELECTION_MOVES,
                    Modifier.weight(1f),
                ) { arrow ->
                    neighbour(rows, arrow)?.let { shell.select(it.item.id) }
                }
            }
            if (!editing) {
                Column {
                    Tool("Copy", R.drawable.ic_copy, shell::copySelected)
                    Tool("Cut", R.drawable.ic_cut, shell::cutSelected)
                }
            }
            Column {
                Tool("Redo", R.drawable.ic_redo, shell::redo)
                Tool("Undo", R.drawable.ic_undo, shell::undo)
            }
            Column {
                Tool("Shrink tools", R.drawable.ic_shrink_tools, onShrink)
                if (editing) {
                    // The field loses the focus and ends the edit. The item stays selected
                    Tool(
                        "Hide keyboard",
                        R.drawable.ic_hide_keyboard,
                        {
                            focus.clearFocus()
                            shell.select(itemId)
                        },
                    )
                } else {
                    Tool("Show keyboard", R.drawable.ic_show_keyboard, { shell.edit(itemId) })
                }
            }
        }
    }
}

/**
 * The tools made small: a round button at the bottom right, over the page. A touch makes the tools
 * full again.
 */
@Composable
internal fun ShrunkTools(onExpand: () -> Unit, modifier: Modifier = Modifier) {
    FloatingActionButton(
        onExpand,
        modifier.padding(12.dp).semantics { contentDescription = "Expand tools" },
        shape = CircleShape,
    ) {
        Icon(
            painterResource(R.drawable.ic_expand_tools),
            contentDescription = null,
            Modifier.size(28.dp),
            tint = ink(enabled = true),
        )
    }
}

private fun move(shell: Shell, itemId: String, arrow: Arrow) =
    when (arrow) {
        Arrow.Left -> shell.outdent(itemId)
        Arrow.Right -> shell.indent(itemId)
        Arrow.Up -> shell.moveUp(itemId)
        Arrow.Down -> shell.moveDown(itemId)
    }

private fun moveSelected(shell: Shell, arrow: Arrow) =
    shell.moveSelected(
        when (arrow) {
            Arrow.Left -> SelectionKey.OUTDENT
            Arrow.Right -> SelectionKey.INDENT
            Arrow.Up -> SelectionKey.MOVEUP
            Arrow.Down -> SelectionKey.MOVEDOWN
        },
    )

/**
 * The row to select for an arrow of the pad of the selection: the row above the selected ones, or
 * the row below them. Left and right have none: the pad has no arrows there.
 */
internal fun neighbour(rows: List<OutlineRow>, arrow: Arrow): OutlineRow? =
    when (arrow) {
        Arrow.Up -> rows.getOrNull(rows.indexOfFirst { it.isSelected } - 1)
        Arrow.Down ->
            rows.indexOfLast { it.isSelected }.takeIf { it >= 0 }?.let { rows.getOrNull(it + 1) }
        Arrow.Left,
        Arrow.Right -> null
    }

/** How high the tools are: two rows of tools. */
private val TOOLS_HEIGHT = 112.dp

/** What each arrow of the pad that moves the item does, as a screen reader says it. */
private val BLOCK_MOVES =
    mapOf(
        Arrow.Left to "Outdent",
        Arrow.Up to "Move up",
        Arrow.Down to "Move down",
        Arrow.Right to "Indent",
    )

/** What the arrows of the pad that chooses the selected item do, as a screen reader says it. */
private val SELECTION_MOVES =
    mapOf(
        Arrow.Up to "Select the item above",
        Arrow.Down to "Select the item below",
    )

/** What each arrow of the pad that moves the cursor does, as a screen reader says it. */
private val CURSOR_MOVES =
    mapOf(
        Arrow.Left to "Cursor left",
        Arrow.Up to "Cursor up",
        Arrow.Down to "Cursor down",
        Arrow.Right to "Cursor right",
    )

/** The colour of the face of a tool, faint when it cannot be touched. */
@Composable
private fun ink(enabled: Boolean): Color =
    if (enabled) MaterialTheme.colorScheme.primary
    else MaterialTheme.colorScheme.onSurface.copy(alpha = 0.38f)

/**
 * A tool with an icon for its face. [name] says what it does, for a screen reader. [icon] is one of
 * the icons of `client/assets/icons`.
 */
@Composable
private fun Tool(
    name: String,
    @DrawableRes icon: Int,
    onTouch: () -> Unit,
    enabled: Boolean = true,
) {
    TextButton(
        onTouch,
        Modifier.semantics { contentDescription = name },
        enabled = enabled,
        contentPadding = PaddingValues(8.dp),
    ) {
        Icon(
            painterResource(icon),
            contentDescription = null,
            Modifier.size(28.dp),
            tint = ink(enabled),
        )
    }
}

/**
 * A pad of four arrows. The whole pad takes touches: the lines from its corners cut it into the
 * parts of the arrows (see [aim]). A touch that is held goes on doing what the arrow does. [icon]
 * is its face. [moves] names what each arrow does, for a screen reader.
 */
@Composable
private fun Pad(
    name: String,
    @DrawableRes icon: Int,
    moves: Map<Arrow, String>,
    modifier: Modifier,
    onArrow: (Arrow) -> Unit,
) {
    val scope = rememberCoroutineScope()
    // The pointer input lives on while the item that is edited changes
    val act by rememberUpdatedState(onArrow)
    val touches =
        Modifier.pointerInput(Unit) {
            awaitEachGesture {
                val down = awaitFirstDown()
                val arrow =
                    aim(
                        down.position.x,
                        down.position.y,
                        size.width.toFloat(),
                        size.height.toFloat(),
                    ) ?: return@awaitEachGesture
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
        customActions = moves.map { (arrow, said) ->
            CustomAccessibilityAction(said) {
                act(arrow)
                true
            }
        }
    }
    Box(
        modifier.then(described).then(touches).fillMaxHeight(),
        contentAlignment = Alignment.Center,
    ) {
        Icon(
            painterResource(icon),
            contentDescription = null,
            Modifier.size(88.dp),
            tint = ink(enabled = true),
        )
    }
}

/** How long a touch is held before an arrow of a pad repeats. */
private const val HOLD_MS = 400L

/** How often an arrow of a pad repeats while the touch is held. */
private const val REPEAT_MS = 100L
