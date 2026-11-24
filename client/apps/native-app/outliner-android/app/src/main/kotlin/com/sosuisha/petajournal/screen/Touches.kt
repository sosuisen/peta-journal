package com.sosuisha.petajournal.screen

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.text.TextLayoutResult
import kotlin.math.max

/**
 * The texts of an item that a touch can fall on, as they are laid out. Each text of the item tells
 * where it stands and how its letters are laid out; a touch on the item finds the letter under it,
 * and the core puts the cursor at its place in the Markdown (ADR 042).
 */
class Touches {
    private val targets = mutableListOf<TouchTarget>()

    /** Where the item stands, which a touch on it is measured from. */
    var item: LayoutCoordinates? = null

    fun add(target: TouchTarget) {
        targets += target
    }

    fun remove(target: TouchTarget) {
        targets -= target
    }

    /**
     * How many letters of the shown text of the item stand before a touch at [point] of the item. A
     * touch beside the texts goes to the nearest line of the text nearest to it. `null` when the
     * item shows no text, as an image alone.
     */
    fun letter(point: Offset): Int? {
        val item = item?.takeIf { it.isAttached } ?: return null
        val placed = targets.mapNotNull { target ->
            val coordinates = target.coordinates?.takeIf { it.isAttached } ?: return@mapNotNull null
            val layout = target.layout ?: return@mapNotNull null
            Triple(target, coordinates, layout)
        }
        val (target, coordinates, layout) =
            placed.minByOrNull { (_, coordinates) ->
                distance(point, item.localBoundingBoxOf(coordinates, clipBounds = false))
            } ?: return null
        val inside = coordinates.localPositionOf(item, point)
        return target.letter(layout.getOffsetForPosition(inside))
    }

    /** How far a point is from a box: 0 inside it. Up and down count more than sideways. */
    private fun distance(point: Offset, box: Rect): Float {
        val across = max(max(box.left - point.x, 0f), point.x - box.right)
        val down = max(max(box.top - point.y, 0f), point.y - box.bottom)
        return across + down * 100
    }
}

/** The touches of the item that a text is drawn in. `null` outside an item. */
val LocalTouches = compositionLocalOf<Touches?> { null }

/** A text of an item that a touch can fall on. */
class TouchTarget {
    /** How many letters of the shown text of the item stand before an offset of the text. */
    var letter: (Int) -> Int = { 0 }
    var coordinates: LayoutCoordinates? = null
    var layout: TextLayoutResult? = null
}

/**
 * Makes the text that is drawn here one that a touch can fall on. [letter] gives how many letters
 * of the shown text of the item stand before an offset of the text. `null` outside an item.
 */
@Composable
fun rememberTouchTarget(letter: (Int) -> Int): TouchTarget? {
    val touches = LocalTouches.current ?: return null
    val target = remember(touches) { TouchTarget() }
    target.letter = letter
    DisposableEffect(touches, target) {
        touches.add(target)
        onDispose { touches.remove(target) }
    }
    return target
}

/** Tells the target where its text stands. */
fun Modifier.touchTarget(target: TouchTarget?): Modifier =
    if (target == null) this else onGloballyPositioned { target.coordinates = it }
