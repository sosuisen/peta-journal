package com.sosuisha.petajournal.screen

import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.click
import androidx.compose.ui.test.hasContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTouchInput
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.runner.RunWith
import org.junit.Test

/** The tools of the row at the bottom that the other tests do not cover. */
@RunWith(AndroidJUnit4::class)
class ToolbarTest : JournalScreenFixture() {
    private fun tool(name: String) = compose.onNode(hasContentDescription(name)).performClick()

    /** Touches the item that shows [text] at its end, so that the edit goes on there. */
    private fun touchEnd(text: String) = compose.onNodeWithText(text).performTouchInput { click(centerRight) }

    /** The items of the page that are shown, each as its level and its body. */
    private fun items(): List<String> = shell.view.outline.rows.map { "${it.item.level} ${it.item.body}" }

    private fun waitForItems(vararg items: String) = compose.waitUntil(WAIT_MS) { items() == items.toList() }

    /** Opens a page of two items, and edits the first after the second became its child. */
    private fun editAParent() {
        download("notes.md", above('A') + "parent" + NL + NL + above('B') + "child" + NL)
        openNotes()
        waitFor("child")
        touchEnd("child")
        pad("Move block", Arrow.Right)
        waitForItems("1 parent", "2 child")
        touchEnd("parent")
        compose.onNodeWithTag(EDITOR_TAG).assertExists()
    }

    // 折りたたみの道具は切り替える。子を隠すと「Expand」になり、もう一度押すと子が戻る。
    @Test
    fun the_collapse_tool_hides_the_children_and_shows_them_again() {
        editAParent()

        tool("Collapse")
        waitForItems("1 parent")
        tool("Expand")

        waitForItems("1 parent", "2 child")
        compose.onNode(hasContentDescription("Collapse")).assertExists()
    }

    // 子のない項目では、折りたたみの道具は押せない。
    @Test
    fun the_collapse_tool_cannot_be_touched_on_an_item_without_children() {
        editAParent()
        compose.onNode(hasContentDescription("Collapse")).assertIsEnabled()

        touchEnd("child")

        compose.onNode(hasContentDescription("Collapse")).assertIsNotEnabled()
    }

    // 元に戻すと、最後の変更が取り消される。やり直すと、また変わる。
    @Test
    fun undo_takes_back_the_last_change_and_redo_does_it_again() {
        download("notes.md", above('A') + "first" + NL + NL + above('B') + "second" + NL)
        openNotes()
        waitFor("second")
        touchEnd("second")

        pad("Move block", Arrow.Right)
        waitForItems("1 first", "2 second")
        tool("Undo")
        waitForItems("1 first", "1 second")
        tool("Redo")

        waitForItems("1 first", "2 second")
    }

    // 項目を動かすパッドを押し続けると、項目は動き続ける。
    @Test
    fun holding_the_pad_keeps_moving_the_item() {
        download(
            "notes.md",
            above('A') + "one" + NL + NL + above('B') + "two" + NL + NL + above('C') + "three" + NL + NL + above('D') + "four" + NL,
        )
        openNotes()
        waitFor("four")
        touchEnd("four")

        compose.onNode(hasContentDescription("Move block")).performTouchInput { down(nearEdge(Arrow.Up)) }
        waitForItems("1 four", "1 one", "1 two", "1 three")
        compose.onNode(hasContentDescription("Move block")).performTouchInput { up() }
    }
}
