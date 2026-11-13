package net.sosuisen.petajournal.screen

import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.hasContentDescription
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.onFirst
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.test.ext.junit.runners.AndroidJUnit4
import java.io.File
import net.sosuisen.petajournal.types.TaskState
import org.junit.Assert.assertEquals
import org.junit.runner.RunWith
import org.junit.Test

/** The edit of an item: the field, Enter, and the tools above the keyboard. */
@RunWith(AndroidJUnit4::class)
class EditTest : JournalScreenFixture() {
    /** The text of the file of the page `notes`, after the core wrote what changed in the app. */
    private fun writtenNotes(): String {
        compose.runOnUiThread { shell.sync() }
        compose.waitUntil(WAIT_MS) { !shell.syncing }
        return File(root, "markdown/docs/notes.md").readText()
    }

    /** Waits until the core has saved an edit and written it to the file of the page. */
    private fun waitForWritten(text: String) = compose.waitUntil(WAIT_MS) { writtenNotes().contains(text) }

    // 項目に触れると、その項目がMarkdownの本文の入力欄になる。
    @Test
    fun a_touch_turns_an_item_into_a_field_with_its_markdown() {
        download("notes.md", "a **bold** word" + NL)
        openNotes()
        waitFor("a bold word")

        compose.onNodeWithText("a bold word").performClick()

        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals("a **bold** word")
    }

    // 入力欄に打った文字は、文字の終わりに入り、ページのファイルに書かれる。
    @Test
    fun what_is_typed_goes_to_the_end_of_the_item_and_into_the_file_of_the_page() {
        download("notes.md", "first" + NL)
        openNotes()
        waitFor("first")
        compose.onNodeWithText("first").performClick()

        compose.onNodeWithTag(EDITOR_TAG).performTextInput(" and more")

        waitForWritten("first and more")
    }

    // 改行のキーは、下に新しい項目を作る。続けて打った文字は、新しい項目に入る。
    @Test
    fun enter_makes_a_new_item_below_and_the_typing_goes_on_there() {
        download("notes.md", "first" + NL)
        openNotes()
        waitFor("first")
        compose.onNodeWithText("first").performClick()

        compose.onNodeWithTag(EDITOR_TAG).performTextInput(NL)
        compose.waitUntil(WAIT_MS) { shell.view.outline.rows.size == 2 }
        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals("")
        compose.onNodeWithTag(EDITOR_TAG).performTextInput("second")

        waitForWritten("second")
        waitFor("first")
        assertEquals(listOf("first", "second"), shell.view.outline.rows.map { it.item.body })
    }

    // ほかの項目に触れると、編集がそこへ移る。前の項目は、飾りのついた表示に戻る。
    @Test
    fun a_touch_on_another_item_moves_the_edit_there() {
        download("notes.md", above('A') + "a **bold** word" + NL + NL + above('B') + "the other" + NL)
        openNotes()
        waitFor("the other")
        compose.onNodeWithText("a bold word").performClick()

        compose.onNodeWithText("the other").performClick()

        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals("the other")
        compose.onNodeWithText("a bold word").assertExists()
    }

    /** Opens a page of three items, and touches the second to edit it. */
    private fun editSecondOfThree() {
        download(
            "notes.md",
            above('A') + "first" + NL + NL + above('B') + "second" + NL + NL + above('C') + "third" + NL,
        )
        openNotes()
        waitFor("third")
        compose.onNodeWithText("second").performClick()
        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals("second")
    }

    private fun tool(name: String) = compose.onNode(hasContentDescription(name)).performClick()

    /** The items of the page, each as its level and its body. */
    private fun items(): List<String> = shell.view.outline.rows.map { "${it.item.level} ${it.item.body}" }

    private fun waitForItems(vararg items: String) = compose.waitUntil(WAIT_MS) { items() == items.toList() }

    // ページを変える操作は、そのたびにアプリに伝わる。アプリは、少しあとに同期する。開くだけでは伝わらない。
    @Test
    fun every_change_of_a_page_is_told_to_the_app() {
        editSecondOfThree()
        compose.runOnIdle { assertEquals(0, edits) }

        compose.onNodeWithTag(EDITOR_TAG).performTextInput("!")
        compose.runOnIdle { assertEquals(1, edits) }
        tool("Indent")
        tool("Task")

        compose.runOnIdle { assertEquals(3, edits) }
    }

    // 編集していないときは、道具の列は出ない。
    @Test
    fun the_tools_show_only_while_an_item_is_edited() {
        download("notes.md", "first" + NL)
        openNotes()
        waitFor("first")

        compose.onNode(hasContentDescription("Indent")).assertDoesNotExist()
        compose.onNodeWithText("first").performClick()

        compose.onNode(hasContentDescription("Indent")).assertExists()
    }

    // 字下げの道具は、編集中の項目を上の項目の子にする。戻す道具は、元に戻す。編集は続く。
    @Test
    fun the_indent_tools_move_the_item_under_the_one_above_and_back() {
        editSecondOfThree()

        tool("Indent")
        waitForItems("1 first", "2 second", "1 third")
        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals("second")
        tool("Outdent")

        waitForItems("1 first", "1 second", "1 third")
    }

    // 上へと下への道具は、編集中の項目を、隣の項目と入れ替える。
    @Test
    fun the_move_tools_swap_the_item_with_its_neighbour() {
        editSecondOfThree()

        tool("Move up")
        waitForItems("1 second", "1 first", "1 third")
        tool("Move down")
        waitForItems("1 first", "1 second", "1 third")
        tool("Move down")

        waitForItems("1 first", "1 third", "1 second")
    }

    // タスクの道具は、編集中の項目をタスクにし、押すたびに状態を進める。
    @Test
    fun the_task_tool_makes_the_item_a_task_and_moves_it_on() {
        editSecondOfThree()
        val task = { shell.view.outline.rows[1].task }

        tool("Task")
        compose.waitUntil(WAIT_MS) { task() == TaskState.NOTSTARTED }
        tool("Task")

        compose.waitUntil(WAIT_MS) { task() == TaskState.RUNNING }
    }

    // 削除の道具は、編集中の項目を消す。
    @Test
    fun the_delete_tool_deletes_the_item() {
        editSecondOfThree()

        tool("Delete item")

        waitForItems("1 first", "1 third")
    }

    /** Opens a page whose third item shows the second one, and asks to delete the second. */
    private fun deleteAnItemThatAnotherShows() {
        val shows = " transclude=\"2020-01-01-00-00-00-AAAAAAB\""
        download("notes.md", above('A') + "first" + NL + NL + above('B') + "second" + NL + NL + above('C', shows))
        openNotes()
        waitFor("first")
        compose.onAllNodes(hasText("second")).onFirst().performClick()
        tool("Delete item")
        waitFor("Delete anyway?")
    }

    // ほかの項目が写している項目を消すときは、先に尋ねる。「Cancel」なら、項目は残る。
    @Test
    fun the_app_asks_before_it_deletes_an_item_that_another_shows() {
        deleteAnItemThatAnotherShows()

        compose.onNodeWithText("Cancel").performClick()

        compose.onNodeWithText("Cancel").assertDoesNotExist()
        compose.runOnIdle { assertEquals(3, shell.view.outline.rows.size) }
    }

    // 尋ねられて「Delete」と答えると、項目は消える。
    @Test
    fun the_item_that_another_shows_is_deleted_when_the_user_says_so() {
        deleteAnItemThatAnotherShows()

        compose.onNodeWithText("Delete").performClick()

        compose.waitUntil(WAIT_MS) { shell.view.outline.rows.size == 2 }
        compose.onNodeWithText("Cancel").assertDoesNotExist()
    }

    // 完了の道具は、編集を終える。項目は、飾りのついた表示に戻り、道具の列は消える。
    @Test
    fun the_done_tool_ends_the_edit() {
        download("notes.md", "a **bold** word" + NL)
        openNotes()
        waitFor("a bold word")
        compose.onNodeWithText("a bold word").performClick()

        compose.onNodeWithText("Done").performClick()

        waitFor("a bold word")
        compose.onNodeWithTag(EDITOR_TAG).assertDoesNotExist()
        compose.onNode(hasContentDescription("Indent")).assertDoesNotExist()
    }

    // タスクの印に触れると、状態が進む。編集には入らない。
    @Test
    fun a_touch_on_the_mark_of_a_task_moves_the_task_on() {
        download("notes.md", above('A') + "- [ ] buy milk" + NL)
        openNotes()
        waitFor("buy milk")

        compose.onNode(hasContentDescription("Task not started")).performClick()

        compose.waitUntil(WAIT_MS) { shell.view.outline.rows[0].task == TaskState.RUNNING }
        compose.onNodeWithTag(EDITOR_TAG).assertDoesNotExist()
    }
}
