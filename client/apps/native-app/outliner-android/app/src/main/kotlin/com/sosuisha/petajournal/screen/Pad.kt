package com.sosuisha.petajournal.screen

import kotlin.math.abs

/**
 * The part of a pad of four arrows that a touch at ([x], [y]) falls on. The pad is [width] wide and
 * [height] high. The lines from its corners through its middle cut it into four parts, one for each
 * arrow. `null` in the middle, a third of the pad on each side, where no arrow is meant.
 */
internal fun aim(x: Float, y: Float, width: Float, height: Float): Arrow? {
    // Measured in halves of the pad, so that the cuts go through its corners
    val across = (x - width / 2) / (width / 2)
    val down = (y - height / 2) / (height / 2)
    if (abs(across) < MIDDLE && abs(down) < MIDDLE) return null
    return if (abs(across) >= abs(down)) {
        if (across < 0) Arrow.Left else Arrow.Right
    } else {
        if (down < 0) Arrow.Up else Arrow.Down
    }
}

/** How far the middle of a pad reaches from its centre, in halves of the pad. */
private const val MIDDLE = 1f / 3
