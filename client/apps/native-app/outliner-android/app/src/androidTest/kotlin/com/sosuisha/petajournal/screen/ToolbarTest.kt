package com.sosuisha.petajournal.screen

import androidx.compose.ui.test.assert
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.click
import androidx.compose.ui.test.hasAnyDescendant
import androidx.compose.ui.test.hasContentDescription
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTouchInput
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

/** The tools of the row at the bottom that the other tests do not cover. */
@RunWith(AndroidJUnit4::class)
class ToolbarTest : JournalScreenFixture() {
    private fun tool(name: String) = compose.onNode(hasContentDescription(name)).performClick()

    /** Touches the item that shows [text] at its end, so that the edit goes on there. */
    private fun touchEnd(text: String) =
        compose.onNodeWithText(text).performTouchInput { click(centerRight) }

    /** The items of the page that are shown, each as its level and its body. */
    private fun items(): List<String> =
        shell.view.outline.rows.map { "${it.item.level} ${it.item.body}" }

    private fun waitForItems(vararg items: String) =
        compose.waitUntil(WAIT_MS) { items() == items.toList() }

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

    // 子を隠した項目の点は、大きな薄い円の上に描かれる。子を戻すと、円は消える。
    @Test
    fun the_bullet_of_a_folded_item_stands_on_a_disc() {
        editAParent()
        compose.onNodeWithTag(FOLDED_TAG, useUnmergedTree = true).assertDoesNotExist()

        tool("Collapse")
        compose.waitUntil(WAIT_MS) {
            compose
                .onAllNodesWithTag(FOLDED_TAG, useUnmergedTree = true)
                .fetchSemanticsNodes()
                .size == 1
        }
        tool("Expand")

        compose.waitUntil(WAIT_MS) {
            compose
                .onAllNodesWithTag(FOLDED_TAG, useUnmergedTree = true)
                .fetchSemanticsNodes()
                .isEmpty()
        }
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
            above('A') +
                "one" +
                NL +
                NL +
                above('B') +
                "two" +
                NL +
                NL +
                above('C') +
                "three" +
                NL +
                NL +
                above('D') +
                "four" +
                NL,
        )
        openNotes()
        waitFor("four")
        touchEnd("four")

        compose.onNode(hasContentDescription("Move block")).performTouchInput {
            down(nearEdge(Arrow.Up))
        }
        waitForItems("1 four", "1 one", "1 two", "1 three")
        compose.onNode(hasContentDescription("Move block")).performTouchInput { up() }
    }

    /** Opens a page of three items and selects the middle one, as Hide keyboard leaves it. */
    private fun selectTwo() {
        download(
            "notes.md",
            above('A') + "one" + NL + NL + above('B') + "two" + NL + NL + above('C') + "three" + NL,
        )
        openNotes()
        waitFor("three")
        touchEnd("two")
        tool("Hide keyboard")
        compose.waitUntil(WAIT_MS) { selected() == listOf("two") }
    }

    private fun selected(): List<String> =
        shell.view.outline.rows.filter { it.isSelected }.map { it.item.body }

    // 選んでいる項目は、色で示される。選び直すと、色も移る。
    @Test
    fun the_selected_item_is_coloured() {
        selectTwo()
        compose.onNodeWithTag(SELECTED_TAG).assert(hasAnyDescendant(hasText("two")))

        pad("Move selection", Arrow.Down)

        compose.waitUntil(WAIT_MS) { selected() == listOf("three") }
        compose.onNodeWithTag(SELECTED_TAG).assert(hasAnyDescendant(hasText("three")))
    }

    // 項目を選んでいる間、2つ目のパッドは、上下で選ぶ項目を変える。
    @Test
    fun the_pad_of_the_selection_selects_the_item_above_or_below() {
        selectTwo()

        pad("Move selection", Arrow.Down)
        compose.waitUntil(WAIT_MS) { selected() == listOf("three") }
        pad("Move selection", Arrow.Up)
        pad("Move selection", Arrow.Up)

        compose.waitUntil(WAIT_MS) { selected() == listOf("one") }
    }

    // 項目を選んでいる間、項目を動かすパッドは、選んだ項目を動かす。
    @Test
    fun the_pad_that_moves_the_block_moves_the_selected_item() {
        selectTwo()

        pad("Move block", Arrow.Up)

        waitForItems("1 two", "1 one", "1 three")
        compose.waitUntil(WAIT_MS) { selected() == listOf("two") }
    }

    // 選んだ項目をコピーの道具でコピーすると、その項目がMarkdownでクリップボードに載る。項目はページに残る。
    @Test
    fun the_copy_tool_puts_the_selected_item_on_the_clipboard_as_markdown() {
        selectTwo()

        tool("Copy")

        compose.waitUntil(WAIT_MS) { copied.isNotEmpty() }
        assertEquals(listOf("- two"), copied)
        assertEquals(listOf("1 one", "1 two", "1 three"), items())
    }

    // 選んだ項目を切り取りの道具で切り取ると、その項目がクリップボードに載り、ページから消える。
    @Test
    fun the_cut_tool_puts_the_selected_item_on_the_clipboard_and_takes_it_off_the_page() {
        selectTwo()

        tool("Cut")

        waitForItems("1 one", "1 three")
        assertEquals(listOf("- two"), copied)
    }

    // コピーと切り取りの道具は、項目を選んでいるときだけ出る。編集しているときは出ない。
    @Test
    fun the_copy_and_cut_tools_show_only_while_an_item_is_selected() {
        selectTwo()
        compose.onNode(hasContentDescription("Copy")).assertExists()
        compose.onNode(hasContentDescription("Cut")).assertExists()

        tool("Show keyboard")

        compose.waitUntil(WAIT_MS) {
            compose.onAllNodesWithTag(EDITOR_TAG).fetchSemanticsNodes().isNotEmpty()
        }
        compose.onNode(hasContentDescription("Copy")).assertDoesNotExist()
        compose.onNode(hasContentDescription("Cut")).assertDoesNotExist()
    }

    // 道具を縮めると、丸いボタンだけになる。押すと、道具が戻る。
    @Test
    fun the_tools_shrink_to_a_round_button_and_come_back() {
        selectTwo()

        tool("Shrink tools")
        compose.onNode(hasContentDescription("Move block")).assertDoesNotExist()
        tool("Expand tools")

        compose.onNode(hasContentDescription("Move block")).assertExists()
    }

    // 縮めた道具は、編集と選択を行き来しても縮めたままである。
    @Test
    fun the_tools_stay_shrunk_while_the_selection_changes_to_an_edit() {
        selectTwo()
        tool("Shrink tools")

        touchEnd("two")

        // The touch ends the selection: the tools of the edit come when the core has answered
        compose.waitUntil(WAIT_MS) {
            selected().isEmpty() &&
                compose.onAllNodesWithTag(EDITOR_TAG).fetchSemanticsNodes().isNotEmpty()
        }
        compose.onNode(hasContentDescription("Expand tools")).assertExists()
    }
}
