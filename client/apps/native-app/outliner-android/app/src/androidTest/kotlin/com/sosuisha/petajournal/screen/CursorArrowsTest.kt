package com.sosuisha.petajournal.screen

import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.center
import androidx.compose.ui.test.click
import androidx.compose.ui.test.hasContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performSemanticsAction
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.text.TextRange
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

/** The pad that moves the cursor, as the arrow keys of a desktop do. */
@RunWith(AndroidJUnit4::class)
class CursorArrowsTest : JournalScreenFixture() {
    private fun arrow(arrow: Arrow) = pad("Move cursor", arrow)

    /** Touches the item that shows [text] at its end, so that the edit goes on there. */
    private fun touchEnd(text: String) =
        compose.onNodeWithText(text).performTouchInput { click(centerRight) }

    /** Puts the cursor of the field before the UTF-16 unit [at]. */
    private fun cursorAt(at: Int) =
        compose.onNodeWithTag(EDITOR_TAG).performSemanticsAction(SemanticsActions.SetSelection) {
            it(at, at, true)
        }

    /** Where the cursor of the field is, in UTF-16 units. */
    private fun cursor(): TextRange =
        compose
            .onNodeWithTag(EDITOR_TAG)
            .fetchSemanticsNode()
            .config[SemanticsProperties.TextSelectionRange]

    /** Opens a page of two items, and touches the one that shows [edited] to edit it. */
    private fun editOneOfTwo(first: String, second: String, edited: String) {
        download("notes.md", above('A') + first + NL + NL + above('B') + second + NL)
        openNotes()
        waitFor(second)
        touchEnd(edited)
        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals(edited)
    }

    /** Waits until the field edits [text], with the cursor before the UTF-16 unit [at]. */
    private fun waitForEdit(text: String, at: Int) =
        compose.waitUntil(WAIT_MS) {
            runCatching { editedText() == text && cursor() == TextRange(at) }.getOrDefault(false)
        }

    private fun editedText(): String =
        compose
            .onNodeWithTag(EDITOR_TAG)
            .fetchSemanticsNode()
            .config[SemanticsProperties.EditableText]
            .text

    // カーソルのパッドは、編集しているときだけ出る。
    @Test
    fun the_pad_shows_only_while_an_item_is_edited() {
        download("notes.md", "first" + NL)
        openNotes()
        waitFor("first")

        compose.onNode(hasContentDescription("Move cursor")).assertDoesNotExist()
        touchEnd("first")

        compose.onNode(hasContentDescription("Move cursor")).assertExists()
    }

    // パッドの真ん中に触れても、カーソルは動かない。
    @Test
    fun a_touch_on_the_middle_of_the_pad_does_not_move_the_cursor() {
        download("notes.md", "abc" + NL)
        openNotes()
        waitFor("abc")
        touchEnd("abc")
        cursorAt(1)

        compose.onNode(hasContentDescription("Move cursor")).performTouchInput { click(center) }

        compose.waitForIdle()
        assertEquals(TextRange(1), cursor())
    }

    // パッドを押し続けると、カーソルは動き続ける。
    @Test
    fun holding_the_pad_keeps_moving_the_cursor() {
        download("notes.md", "abcdefgh" + NL)
        openNotes()
        waitFor("abcdefgh")
        touchEnd("abcdefgh")
        cursorAt(8)

        compose.onNode(hasContentDescription("Move cursor")).performTouchInput {
            down(nearEdge(Arrow.Left))
        }
        compose.waitUntil(WAIT_MS) { cursor() == TextRange(0) }
        compose.onNode(hasContentDescription("Move cursor")).performTouchInput { up() }

        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals("abcdefgh")
    }

    // ←と→は、カーソルを1文字ずつ動かす。絵文字は、1文字として数える。
    @Test
    fun left_and_right_move_the_cursor_by_one_character_and_count_an_emoji_as_one() {
        download("notes.md", "😀ab" + NL)
        openNotes()
        waitFor("😀ab")
        touchEnd("😀ab")
        cursorAt(4)

        arrow(Arrow.Left)
        arrow(Arrow.Left)
        arrow(Arrow.Left)
        assertEquals(TextRange(0), cursor())
        arrow(Arrow.Right)

        assertEquals(TextRange(2), cursor())
        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals("😀ab")
    }

    // 項目の先頭での←は、前の項目の末尾へ移る。
    @Test
    fun left_at_the_start_moves_to_the_end_of_the_item_before() {
        editOneOfTwo("first", "second", edited = "second")
        cursorAt(0)

        arrow(Arrow.Left)

        waitForEdit("first", 5)
    }

    // 項目の末尾での→は、次の項目の先頭へ移る。
    @Test
    fun right_at_the_end_moves_to_the_start_of_the_item_after() {
        editOneOfTwo("first", "second", edited = "first")
        cursorAt(5)

        arrow(Arrow.Right)

        waitForEdit("second", 0)
    }

    // 最初の行での↑は、前の項目へ移る。行の中の位置は変わらない。
    @Test
    fun up_on_the_first_line_moves_to_the_item_before_at_the_same_place() {
        editOneOfTwo("first", "second", edited = "second")
        cursorAt(3)

        arrow(Arrow.Up)

        waitForEdit("first", 3)
    }

    // 最後の行での↓は、次の項目へ移る。行の中の位置は変わらない。
    @Test
    fun down_on_the_last_line_moves_to_the_item_after_at_the_same_place() {
        editOneOfTwo("first", "second", edited = "first")
        cursorAt(2)

        arrow(Arrow.Down)

        waitForEdit("second", 2)
    }

    // 何行もある項目では、↑と↓は、同じ項目の上と下の行へ動かす。
    @Test
    fun up_and_down_move_between_the_lines_of_an_item() {
        download("notes.md", "one" + NL + "two" + NL)
        openNotes()
        waitFor("two")
        compose.onNodeWithText("two", substring = true).performTouchInput { click(centerRight) }
        val text = editedText()
        cursorAt(text.length)

        arrow(Arrow.Up)
        assertEquals(TextRange(text.indexOf('\n')), cursor())
        arrow(Arrow.Down)

        assertEquals(TextRange(text.length), cursor())
        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals(text)
    }
}
