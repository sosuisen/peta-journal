package com.sosuisha.petajournal.screen

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/** Which arrow of a pad a touch falls on. The pad here is 90 wide and 60 high. */
class PadTest {
    private fun at(x: Float, y: Float) = aim(x, y, width = 90f, height = 60f)

    // 端の近くへのタッチは、その向きの矢印になる。
    @Test
    fun a_touch_near_an_edge_is_the_arrow_of_that_side() {
        assertEquals(Arrow.Left, at(2f, 30f))
        assertEquals(Arrow.Right, at(88f, 30f))
        assertEquals(Arrow.Up, at(45f, 2f))
        assertEquals(Arrow.Down, at(45f, 58f))
    }

    // 矢印の絵の上でなくても、角から角への線で分けた範囲なら、その向きになる。
    @Test
    fun the_parts_are_cut_by_the_lines_from_corner_to_corner() {
        // Near the left top corner: a little more to the left than up
        assertEquals(Arrow.Left, at(5f, 4f))
        // A little more up than to the left
        assertEquals(Arrow.Up, at(7f, 4f))
        // Near the right bottom corner, more down than to the right
        assertEquals(Arrow.Down, at(84f, 58f))
    }

    // 真ん中の3分の1の範囲は、どの向きでもない。
    @Test
    fun the_middle_third_is_no_arrow() {
        assertNull(at(45f, 30f))
        assertNull(at(31f, 21f))
        assertNull(at(59f, 39f))
    }

    // 真ん中の範囲のすぐ外は、矢印になる。
    @Test
    fun just_outside_the_middle_is_an_arrow() {
        assertEquals(Arrow.Left, at(29f, 30f))
        assertEquals(Arrow.Down, at(45f, 41f))
    }
}
