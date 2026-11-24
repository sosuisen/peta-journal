package com.sosuisha.petajournal.screen

import androidx.compose.runtime.compositionLocalOf

/** A button that moves the cursor, as an arrow key of a desktop does. */
internal enum class Arrow {
    Left,
    Up,
    Down,
    Right,
}

/**
 * The way from the arrow buttons to the field of the item that is edited. The field holds the
 * cursor and the lines of its text, and the buttons lie outside of it, under the tools.
 */
internal class Caret {
    /** Moves the cursor of the field that is edited. `null` while no item is edited. */
    var move: ((Arrow) -> Unit)? = null
}

/** The [Caret] of the screen. The field of an item and the arrow buttons share it. */
internal val LocalCaret = compositionLocalOf { Caret() }
