package com.sosuisha.petajournal.screen

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
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
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.sosuisha.petajournal.shell.Shell

/**
 * The rows at the bottom of the screen (ADR 031). Above, while an item is edited, the tools
 * that change it. Below, always, the arrows that move the cursor. A phone has no Tab key, no
 * key with Alt and no arrow keys. [itemId] is the item that is edited, and `null` when none
 * is: the arrows cannot be touched then. A touch on a tool leaves the focus in the field.
 */
@Composable
internal fun Tools(itemId: String?, shell: Shell) {
    val focus = LocalFocusManager.current
    val caret = LocalCaret.current
    Surface(tonalElevation = 3.dp) {
        Column {
            if (itemId != null) {
                Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), verticalAlignment = Alignment.CenterVertically) {
                    val ink = MaterialTheme.colorScheme.primary
                    Tool("Outdent", ink, { shell.outdent(itemId) }) { toTheBar(it, left = true) }
                    Tool("Indent", ink, { shell.indent(itemId) }) { toTheBar(it, left = false) }
                    Tool("Move up", ink, { shell.moveUp(itemId) }) { itemMoves(it, up = true) }
                    Tool("Move down", ink, { shell.moveDown(itemId) }) { itemMoves(it, up = false) }
                    Tool("Task", ink, { shell.cycleTask(itemId) }) { checkedBox(it) }
                    Tool("Delete item", MaterialTheme.colorScheme.error, { shell.delete(itemId) }) { cross(it) }
                    Spacer(Modifier.weight(1f))
                    // The field loses the focus, and tells the core that the edit has ended
                    TextButton({ focus.clearFocus() }) { Text("Done") }
                }
            }
            // Never scrolled: the arrows are always where the thumb looks for them
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.Center) {
                val edited = itemId != null
                val ink = if (edited) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface.copy(alpha = 0.38f)
                for ((arrow, name) in ARROWS) {
                    Tool(name, ink, { caret.move?.invoke(arrow) }, enabled = edited) { pointing(it, arrow) }
                }
            }
        }
    }
}

/** The arrow buttons, in their order on the screen, with the names a screen reader says. */
private val ARROWS = listOf(
    Arrow.Left to "Cursor left",
    Arrow.Up to "Cursor up",
    Arrow.Down to "Cursor down",
    Arrow.Right to "Cursor right",
)

/**
 * A tool with a drawing for its face. [name] says what it does, for a screen reader. The
 * face is drawn with [draw] in a square of 24 units, in lines of the colour [ink].
 */
@Composable
private fun Tool(name: String, ink: Color, onTouch: () -> Unit, enabled: Boolean = true, draw: Face.(Color) -> Unit) {
    TextButton(onTouch, Modifier.semantics { contentDescription = name }, enabled = enabled, contentPadding = PaddingValues(10.dp)) {
        Canvas(Modifier.size(28.dp)) { Face(this, size.width / 24f).draw(ink) }
    }
}

/** Where the face of a tool is drawn. A place is given in units: the face is 24 units wide. */
private class Face(private val on: DrawScope, private val unit: Float) {
    /** A line through the places, each as x and y, one after the other. */
    fun line(ink: Color, vararg places: Float) {
        val path = Path()
        path.moveTo(places[0] * unit, places[1] * unit)
        for (at in 2 until places.size step 2) path.lineTo(places[at] * unit, places[at + 1] * unit)
        on.drawPath(path, ink, style = Stroke(2.6f * unit, cap = StrokeCap.Round, join = StrokeJoin.Round))
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

/** An arrow to a bar at the side: the item goes in to the right, or back out to the left. */
private fun Face.toTheBar(ink: Color, left: Boolean) {
    // The face of the indent, and its mirror for the outdent
    fun x(at: Float) = if (left) 24f - at else at
    line(ink, x(3f), 12f, x(15f), 12f)
    line(ink, x(10f), 7f, x(15f), 12f, x(10f), 17f)
    line(ink, x(20.5f), 4.5f, x(20.5f), 19.5f)
}

/**
 * An item with an arrow over or under it: the item moves over or under its neighbour. Not a
 * bare arrow, which is the face of the cursor buttons.
 */
private fun Face.itemMoves(ink: Color, up: Boolean) {
    fun y(at: Float) = if (up) at else 24f - at
    // The item, as a bar of text
    line(ink, 5f, y(19f), 19f, y(19f))
    line(ink, 12f, y(14.5f), 12f, y(3.5f))
    line(ink, 7.5f, y(8f), 12f, y(3.5f), 16.5f, y(8f))
}

/** An arrow that points where the cursor goes. */
private fun Face.pointing(ink: Color, arrow: Arrow) {
    // The arrow to the right, turned for the others
    fun place(x: Float, y: Float): Pair<Float, Float> =
        when (arrow) {
            Arrow.Right -> x to y
            Arrow.Left -> 24f - x to y
            Arrow.Down -> y to x
            Arrow.Up -> y to 24f - x
        }
    fun line(vararg xy: Float) {
        val turned = xy.toList().chunked(2).flatMap { (x, y) -> place(x, y).toList() }
        line(ink, *turned.toFloatArray())
    }
    line(5f, 12f, 19f, 12f)
    line(13f, 6f, 19f, 12f, 13f, 18f)
}

/** A box with a check mark: the item becomes a task, and the task moves on. */
private fun Face.checkedBox(ink: Color) {
    square(ink, 4f, 4f, 16f)
    line(ink, 8f, 12.5f, 11f, 15.5f, 16.5f, 9f)
}

/** A cross: the item is deleted. */
private fun Face.cross(ink: Color) {
    line(ink, 6f, 6f, 18f, 18f)
    line(ink, 18f, 6f, 6f, 18f)
}
