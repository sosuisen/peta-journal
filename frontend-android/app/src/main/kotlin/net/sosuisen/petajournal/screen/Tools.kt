package net.sosuisen.petajournal.screen

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.horizontalScroll
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
import net.sosuisen.petajournal.shell.Shell

/**
 * What changes the item that is edited, in a row above the keyboard (ADR 031). A phone has
 * no Tab key and no key with Alt. A touch on a tool leaves the focus in the field.
 */
@Composable
internal fun Tools(itemId: String, shell: Shell) {
    val focus = LocalFocusManager.current
    Surface(tonalElevation = 3.dp) {
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), verticalAlignment = Alignment.CenterVertically) {
            val ink = MaterialTheme.colorScheme.primary
            Tool("Outdent", ink, { shell.outdent(itemId) }) { toTheBar(it, left = true) }
            Tool("Indent", ink, { shell.indent(itemId) }) { toTheBar(it, left = false) }
            Tool("Move up", ink, { shell.moveUp(itemId) }) { upOrDown(it, up = true) }
            Tool("Move down", ink, { shell.moveDown(itemId) }) { upOrDown(it, up = false) }
            Tool("Task", ink, { shell.cycleTask(itemId) }) { checkedBox(it) }
            Tool("Delete item", MaterialTheme.colorScheme.error, { shell.delete(itemId) }) { cross(it) }
            Spacer(Modifier.weight(1f))
            // The field loses the focus, and tells the core that the edit has ended
            TextButton({ focus.clearFocus() }) { Text("Done") }
        }
    }
}

/**
 * A tool with a drawing for its face. [name] says what it does, for a screen reader. The
 * face is drawn with [draw] in a square of 24 units, in lines of the colour [ink].
 */
@Composable
private fun Tool(name: String, ink: Color, onTouch: () -> Unit, draw: Face.(Color) -> Unit) {
    TextButton(onTouch, Modifier.semantics { contentDescription = name }, contentPadding = PaddingValues(10.dp)) {
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

/** An arrow up or down: the item moves over or under its neighbour. */
private fun Face.upOrDown(ink: Color, up: Boolean) {
    fun y(at: Float) = if (up) at else 24f - at
    line(ink, 12f, y(20f), 12f, y(4.5f))
    line(ink, 5.5f, y(11f), 12f, y(4.5f), 18.5f, y(11f))
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
