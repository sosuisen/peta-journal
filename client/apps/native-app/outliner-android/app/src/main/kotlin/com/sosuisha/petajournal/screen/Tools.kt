package com.sosuisha.petajournal.screen

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Alignment
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.disabled
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
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
            Tool(if (collapsed) "Expand" else "Collapse", { itemId?.let(shell::toggleCollapsed) }, enabled = parent) {
                if (collapsed) awayFromTheLine(it) else toTheLine(it)
            }
            Pad("Move block", BLOCK_MOVES, enabled = itemId != null, { arrow ->
                val id = itemId ?: return@Pad
                when (arrow) {
                    Arrow.Left -> shell.outdent(id)
                    Arrow.Right -> shell.indent(id)
                    Arrow.Up -> shell.moveUp(id)
                    Arrow.Down -> shell.moveDown(id)
                }
            }) { blockPad(it) }
            Pad("Move cursor", CURSOR_MOVES, enabled = itemId != null, { caret.move?.invoke(it) }) { cursorPad(it) }
            Tool("Task", { itemId?.let(shell::cycleTask) }, enabled = itemId != null) { checkedBox(it) }
            Tool("Undo", shell::undo) { turning(it, back = true) }
            Tool("Redo", shell::redo) { turning(it, back = false) }
            // The field loses the focus, and tells the core that the edit has ended
            TextButton({ focus.clearFocus() }, enabled = itemId != null, contentPadding = PaddingValues(horizontal = 6.dp)) { Text("Done") }
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
 * A tool with a drawing for its face. [name] says what it does, for a screen reader. The
 * face is drawn with [draw] in a square of 24 units.
 */
@Composable
private fun Tool(name: String, onTouch: () -> Unit, enabled: Boolean = true, draw: Face.(Color) -> Unit) {
    val ink = ink(enabled)
    TextButton(onTouch, Modifier.semantics { contentDescription = name }, enabled = enabled, contentPadding = PaddingValues(8.dp)) {
        Canvas(Modifier.size(28.dp)) { Face(this, size.width / 24f).draw(ink) }
    }
}

/**
 * A pad of four arrows. The whole pad takes touches: the lines from its corners cut it into
 * the parts of the arrows (see [aim]). A touch that is held goes on doing what the arrow does.
 * [moves] names what each arrow does, for a screen reader.
 */
@Composable
private fun Pad(
    name: String,
    moves: Map<Arrow, String>,
    enabled: Boolean,
    onArrow: (Arrow) -> Unit,
    draw: Face.(Color) -> Unit,
) {
    val ink = ink(enabled)
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
    Box(described.then(touches).size(width = 76.dp, height = 56.dp), contentAlignment = Alignment.Center) {
        Canvas(Modifier.size(44.dp)) { Face(this, size.width / 24f).draw(ink) }
    }
}

/** How long a touch is held before an arrow of a pad repeats. */
private const val HOLD_MS = 400L

/** How often an arrow of a pad repeats while the touch is held. */
private const val REPEAT_MS = 100L

/** Where the face of a tool is drawn. A place is given in units: the face is 24 units wide. */
private class Face(private val on: DrawScope, private val unit: Float) {
    private fun path(places: FloatArray, closed: Boolean): Path {
        val path = Path()
        path.moveTo(places[0] * unit, places[1] * unit)
        for (at in 2 until places.size step 2) path.lineTo(places[at] * unit, places[at + 1] * unit)
        if (closed) path.close()
        return path
    }

    /** A line through the places, each as x and y, one after the other. */
    fun line(ink: Color, vararg places: Float, width: Float = 2.6f) {
        on.drawPath(path(places, closed = false), ink, style = Stroke(width * unit, cap = StrokeCap.Round, join = StrokeJoin.Round))
    }

    /** A shape through the places, filled, or drawn as its outline when [filled] is false. */
    fun shape(ink: Color, vararg places: Float, filled: Boolean) {
        val path = path(places, closed = true)
        if (filled) on.drawPath(path, ink) else on.drawPath(path, ink, style = Stroke(1.6f * unit, join = StrokeJoin.Round))
    }

    /** A line that bends from [from] to [to], pulled towards [through]. */
    fun curve(ink: Color, from: Offset, through: Offset, to: Offset) {
        val path = Path()
        path.moveTo(from.x * unit, from.y * unit)
        path.quadraticTo(through.x * unit, through.y * unit, to.x * unit, to.y * unit)
        on.drawPath(path, ink, style = Stroke(2.6f * unit, cap = StrokeCap.Round))
    }

    /** A square with round corners, from the left top place and with the length of a side. */
    fun square(ink: Color, left: Float, top: Float, side: Float) {
        on.drawRoundRect(
            ink,
            topLeft = Offset(left * unit, top * unit),
            size = Size(side * unit, side * unit),
            cornerRadius = CornerRadius(2.5f * unit),
            style = Stroke(2.2f * unit),
        )
    }
}

/** A triangle at a side of a pad, pointing out of it. Filled or drawn as its outline. */
private fun Face.sideArrow(ink: Color, arrow: Arrow, filled: Boolean) {
    // The one to the right, turned for the others
    val right = floatArrayOf(19f, 8.5f, 23f, 12f, 19f, 15.5f)
    val turned = right.toList().chunked(2).flatMap { (x, y) ->
        when (arrow) {
            Arrow.Right -> listOf(x, y)
            Arrow.Left -> listOf(24f - x, y)
            Arrow.Down -> listOf(y, x)
            Arrow.Up -> listOf(y, 24f - x)
        }
    }
    shape(ink, *turned.toFloatArray(), filled = filled)
}

/** Filled arrows round the lines of an item: the item moves. */
private fun Face.blockPad(ink: Color) {
    for (arrow in Arrow.entries) sideArrow(ink, arrow, filled = true)
    line(ink, 8f, 8.5f, 16f, 8.5f, width = 1.8f)
    line(ink, 8f, 12f, 16f, 12f, width = 1.8f)
    line(ink, 8f, 15.5f, 14f, 15.5f, width = 1.8f)
}

/** Open arrows round the bar of a cursor: the cursor moves. */
private fun Face.cursorPad(ink: Color) {
    for (arrow in Arrow.entries) sideArrow(ink, arrow, filled = false)
    line(ink, 12f, 7.5f, 12f, 16.5f, width = 2f)
    line(ink, 10.5f, 7.5f, 13.5f, 7.5f, width = 2f)
    line(ink, 10.5f, 16.5f, 13.5f, 16.5f, width = 2f)
}

/** Arrows that meet at two lines in the middle: the children are hidden. */
private fun Face.toTheLine(ink: Color) {
    line(ink, 4f, 10.5f, 20f, 10.5f)
    line(ink, 4f, 13.5f, 20f, 13.5f)
    line(ink, 12f, 2.5f, 12f, 7.5f)
    line(ink, 9f, 5f, 12f, 7.5f, 15f, 5f)
    line(ink, 12f, 21.5f, 12f, 16.5f)
    line(ink, 9f, 19f, 12f, 16.5f, 15f, 19f)
}

/** Arrows from the middle out to two lines: the children are shown again. */
private fun Face.awayFromTheLine(ink: Color) {
    line(ink, 4f, 3f, 20f, 3f)
    line(ink, 4f, 21f, 20f, 21f)
    line(ink, 12f, 7f, 12f, 17f)
    line(ink, 9f, 9.5f, 12f, 7f, 15f, 9.5f)
    line(ink, 9f, 14.5f, 12f, 17f, 15f, 14.5f)
}

/** A bent arrow that turns back to the left for an undo, or on to the right for a redo. */
private fun Face.turning(ink: Color, back: Boolean) {
    fun x(at: Float) = if (back) at else 24f - at
    curve(ink, Offset(x(6f), 9f), Offset(x(22f), 6f), Offset(x(13f), 20f))
    line(ink, x(5f), 4f, x(6f), 9f, x(11f), 9.5f)
}

/** A box with a check mark: the item becomes a task, and the task moves on. */
private fun Face.checkedBox(ink: Color) {
    square(ink, 4f, 4f, 16f)
    line(ink, 8f, 12.5f, 11f, 15.5f, 16.5f, 9f)
}
