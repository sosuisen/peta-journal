package com.sosuisha.petajournal.screen

import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/** What the editor tells the shell of a change of its field. */
class TellingTest {
    // キーボードが語を変換している間は、変換中と伝える。語はまだ確定していない。
    @Test
    fun a_word_being_composed_is_told_as_composing() {
        val next = TextFieldValue("京都でかい", TextRange(5), composition = TextRange(3, 5))

        assertEquals(Telling.Composing, telling(next, body = "京都で"))
    }

    // 確定した文字は、入力として伝える。
    @Test
    fun committed_text_is_told_as_typed() {
        val next = TextFieldValue("京都で会議", TextRange(5))

        assertEquals(Telling.Typed, telling(next, body = "京都で"))
    }

    // 文字が変わらない変化、たとえばカーソルの移動は、何も伝えない。
    @Test
    fun a_change_that_keeps_the_text_tells_nothing() {
        val next = TextFieldValue("京都で", TextRange(1))

        assertNull(telling(next, body = "京都で"))
    }
}
