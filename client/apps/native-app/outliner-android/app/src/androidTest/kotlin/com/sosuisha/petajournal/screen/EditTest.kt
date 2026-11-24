package com.sosuisha.petajournal.screen

import android.content.ClipData
import android.content.ClipboardManager
import android.view.inputmethod.EditorInfo
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.platform.ViewRootForTest
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.click
import androidx.compose.ui.test.hasContentDescription
import androidx.compose.ui.test.hasScrollToNodeAction
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.onFirst
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performKeyInput
import androidx.compose.ui.test.performMouseInput
import androidx.compose.ui.test.performScrollToNode
import androidx.compose.ui.test.performSemanticsAction
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.test.performTextReplacement
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.pressKey
import androidx.compose.ui.test.rightClick
import androidx.compose.ui.test.withKeyDown
import androidx.compose.ui.text.TextRange
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.sosuisha.petajournal.types.TaskState
import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

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
    private fun waitForWritten(text: String) =
        compose.waitUntil(WAIT_MS) { writtenNotes().contains(text) }

    // 項目に触れると、その項目がMarkdownの本文の入力欄になる。
    @Test
    fun a_touch_turns_an_item_into_a_field_with_its_markdown() {
        download("notes.md", "a **bold** word" + NL)
        openNotes()
        waitFor("a bold word")

        touchEnd("a bold word")

        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals("a **bold** word")
    }

    // 項目の文字に触れると、触れた文字のMarkdownでの位置から書ける。
    @Test
    fun a_touch_on_the_text_of_an_item_writes_at_its_place_in_the_markdown() {
        download("notes.md", "**bold** plain" + NL)
        openNotes()
        waitFor("bold plain")

        compose.onNodeWithText("bold plain").performTouchInput { click(centerLeft) }

        compose.onNodeWithTag(EDITOR_TAG).performTextInput("X")
        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals("X**bold** plain")
    }

    // 入力欄に打った文字は、文字の終わりに入り、ページのファイルに書かれる。
    @Test
    fun what_is_typed_goes_to_the_end_of_the_item_and_into_the_file_of_the_page() {
        download("notes.md", "first" + NL)
        openNotes()
        waitFor("first")
        touchEnd("first")

        compose.onNodeWithTag(EDITOR_TAG).performTextInput(" and more")

        waitForWritten("first and more")
    }

    // アプリが裏に回ると、保存を待っていた本文が、待たずに保存され、ページのファイルに書かれる（#766）。
    @Test
    fun what_waits_to_be_saved_is_saved_when_the_app_goes_to_the_back() {
        download("notes.md", "first" + NL)
        openNotes()
        waitFor("first")
        touchEnd("first")
        compose.onNodeWithTag(EDITOR_TAG).performTextInput(" before the back")

        compose.runOnUiThread { shell.wentToTheBack() }

        assertEquals(true, writtenNotes().contains("first before the back"))
    }

    // 開始記号を打ち終えると、コアが閉じる記号を補う（ADR 050、ADR 057）。
    @Test
    fun typing_an_opening_closes_it() {
        download("notes.md", "first" + NL)
        openNotes()
        waitFor("first")
        touchEnd("first")

        compose.onNodeWithTag(EDITOR_TAG).performTextReplacement(">>")
        compose.onNodeWithTag(EDITOR_TAG).performTextInput(">")

        compose.waitUntil(WAIT_MS) { shell.view.outline.rows[0].item.body == ">>>\n>>>" }
        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals(">>>\n>>>")
    }

    // 改行のキーは、下に新しい項目を作る。続けて打った文字は、新しい項目に入る。
    @Test
    fun enter_makes_a_new_item_below_and_the_typing_goes_on_there() {
        download("notes.md", "first" + NL)
        openNotes()
        waitFor("first")
        touchEnd("first")

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
        download(
            "notes.md",
            above('A') + "a **bold** word" + NL + NL + above('B') + "the other" + NL,
        )
        openNotes()
        waitFor("the other")
        touchEnd("a bold word")

        touchEnd("the other")

        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals("the other")
        compose.onNodeWithText("a bold word").assertExists()
    }

    /** Opens a page of three items, and touches the second to edit it. */
    private fun editSecondOfThree() {
        download(
            "notes.md",
            above('A') +
                "first" +
                NL +
                NL +
                above('B') +
                "second" +
                NL +
                NL +
                above('C') +
                "third" +
                NL,
        )
        openNotes()
        waitFor("third")
        touchEnd("second")
        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals("second")
    }

    private fun tool(name: String) = compose.onNode(hasContentDescription(name)).performClick()

    /** Touches the item that shows [text] at its end, so that the edit goes on there. */
    private fun touchEnd(text: String) =
        compose.onNodeWithText(text).performTouchInput { click(centerRight) }

    /** The items of the page, each as its level and its body. */
    private fun items(): List<String> =
        shell.view.outline.rows.map { "${it.item.level} ${it.item.body}" }

    /** The word of the system for Paste, as the menu of the text shows it. */
    private val PASTE =
        InstrumentationRegistry.getInstrumentation().targetContext.getString(android.R.string.paste)

    private fun waitForItems(vararg items: String) =
        compose.waitUntil(WAIT_MS) { items() == items.toList() }

    /** Puts [text] on the clipboard of the system, as another app does. */
    private fun onTheClipboard(text: String) = compose.runOnUiThread {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        context
            .getSystemService(ClipboardManager::class.java)
            .setPrimaryClip(ClipData.newPlainText("test", text))
    }

    // Markdownのリストを項目の終わりに貼り付けると、1つ目はその項目に続き、2つ目は下の新しい項目になる。
    @Test
    fun a_list_pasted_at_the_end_of_an_item_goes_on_in_it_and_makes_a_new_item_below() {
        editSecondOfThree()
        onTheClipboard("- apple" + NL + "- pear")

        compose.onNodeWithTag(EDITOR_TAG).performSemanticsAction(SemanticsActions.PasteText)

        waitForItems("1 first", "1 secondapple", "1 pear", "1 third")
    }

    /**
     * Opens a page of a parent with a child and a third item, copies the parent and its child with
     * the tool, and edits the third at its end.
     */
    private fun copyParentAndChildThenEditThird() {
        download(
            "notes.md",
            above('A') +
                "parent" +
                NL +
                NL +
                above('B').replace("<!-- @ ", "<!-- @@ ") +
                "child" +
                NL +
                NL +
                above('C') +
                "third" +
                NL,
        )
        openNotes()
        waitForItems("1 parent", "2 child", "1 third")
        touchEnd("parent")
        tool("Hide keyboard")
        compose.waitUntil(WAIT_MS) { shell.view.outline.rows.any { it.isSelected } }
        tool("Copy")
        compose.waitUntil(WAIT_MS) { copied.isNotEmpty() }
        touchEnd("third")
        // The touch ends the selection: the field of the item comes when the core has answered
        compose.waitUntil(WAIT_MS) {
            compose
                .onAllNodes(hasTestTag(EDITOR_TAG) and hasText("third"))
                .fetchSemanticsNodes()
                .isNotEmpty()
        }
    }

    // 親と子を選んでコピーの道具でコピーし、別の項目の終わりに貼り付けると、親と子の形のまま戻る。
    @Test
    fun a_parent_and_its_child_that_were_copied_are_pasted_back_as_a_parent_and_its_child() {
        copyParentAndChildThenEditThird()

        compose.onNodeWithTag(EDITOR_TAG).performSemanticsAction(SemanticsActions.PasteText)

        waitForItems("1 parent", "2 child", "1 thirdparent", "2 child")
    }

    // コピーの道具でコピーした項目を、画面のキーボードのクリップボードの候補から入れても、項目の形のまま戻る。
    @Test
    fun items_copied_and_put_in_from_the_clipboard_of_the_keyboard_on_the_screen_come_back_as_items() {
        copyParentAndChildThenEditThird()

        commitOnTheScreen(copied.last())

        waitForItems("1 parent", "2 child", "1 thirdparent", "2 child")
    }

    // 実のキーのキーボードのCtrl+Vも、コアが貼り付ける。リストは項目になる。
    @Test
    fun ctrl_v_of_a_keyboard_with_real_keys_pastes_a_list_as_items() {
        editSecondOfThree()
        onTheClipboard("- apple" + NL + "- pear")

        compose.onNodeWithTag(EDITOR_TAG).performKeyInput {
            withKeyDown(Key.CtrlLeft) { pressKey(Key.V) }
        }

        waitForItems("1 first", "1 secondapple", "1 pear", "1 third")
    }

    // 文字のメニューの貼り付けも、コアが貼り付ける。リストは項目になる。
    @Test
    fun paste_of_the_menu_of_the_text_pastes_a_list_as_items() {
        editSecondOfThree()
        onTheClipboard("- apple" + NL + "- pear")

        compose.onNodeWithTag(EDITOR_TAG).performMouseInput { rightClick(centerRight) }
        compose.onNodeWithText(PASTE).performClick()

        waitForItems("1 first", "1 secondapple", "1 pear", "1 third")
    }

    // 文字を選んで貼り付けると、選んだ文字が貼り付けた文字に置き換わる。
    @Test
    fun a_paste_over_chosen_text_puts_the_pasted_text_in_its_place() {
        editSecondOfThree()
        onTheClipboard("best")
        compose.onNodeWithTag(EDITOR_TAG).performSemanticsAction(SemanticsActions.SetSelection) {
            it(0, 6, false)
        }

        compose.onNodeWithTag(EDITOR_TAG).performSemanticsAction(SemanticsActions.PasteText)

        waitForItems("1 first", "1 best", "1 third")
    }

    // 道具は、編集しているときだけ出る。
    @Test
    fun the_tools_show_only_while_an_item_is_edited() {
        download("notes.md", "first" + NL)
        openNotes()
        waitFor("first")

        compose.onNode(hasContentDescription("Move block")).assertDoesNotExist()
        touchEnd("first")

        compose.onNode(hasContentDescription("Move block")).assertExists()
    }

    // 項目を動かすパッドの左右は、字下げと字下げを戻す。編集中の項目を上の項目の子にし、元に戻す。編集は続く。
    @Test
    fun the_indent_tools_move_the_item_under_the_one_above_and_back() {
        editSecondOfThree()

        pad("Move block", Arrow.Right)
        waitForItems("1 first", "2 second", "1 third")
        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals("second")
        pad("Move block", Arrow.Left)

        waitForItems("1 first", "1 second", "1 third")
    }

    // 項目を動かすパッドの上下は、編集中の項目を、隣の項目と入れ替える。
    @Test
    fun the_move_tools_swap_the_item_with_its_neighbour() {
        editSecondOfThree()

        pad("Move block", Arrow.Up)
        waitForItems("1 second", "1 first", "1 third")
        pad("Move block", Arrow.Down)
        waitForItems("1 first", "1 second", "1 third")
        pad("Move block", Arrow.Down)

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

    // 道具の列に、削除の道具はない（2026-11-13にユーザーが決めた）。項目を消すのは、空の項目の先頭でのBackspaceである。
    @Test
    fun the_tools_have_no_tool_to_delete_the_item() {
        editSecondOfThree()

        compose.onNode(hasContentDescription("Delete item")).assertDoesNotExist()
    }

    /**
     * Opens a page whose third item shows the second one, and joins the second to the first with
     * Backspace at its start. The second item goes away then.
     */
    private fun deleteAnItemThatAnotherShows() {
        val shows = " transclude=\"2020-01-01-00-00-00-AAAAAAB\""
        download(
            "notes.md",
            above('A') + "first" + NL + NL + above('B') + "second" + NL + NL + above('C', shows),
        )
        openNotes()
        waitFor("first")
        compose.onAllNodes(hasText("second")).onFirst().performTouchInput { click(centerRight) }
        cursorAt(0)
        pressBackspace()
        waitFor("Delete anyway?")
    }

    // ほかの項目が写している項目が、つなぐことで消えるときは、先に尋ねる。「Cancel」なら、項目は残る。
    @Test
    fun the_app_asks_before_it_deletes_an_item_that_another_shows() {
        deleteAnItemThatAnotherShows()

        compose.onNodeWithText("Cancel").performClick()

        compose.onNodeWithText("Cancel").assertDoesNotExist()
        compose.runOnIdle { assertEquals(3, shell.view.outline.rows.size) }
    }

    // 尋ねられて「Delete」と答えると、項目はつながって消える。
    @Test
    fun the_item_that_another_shows_is_deleted_when_the_user_says_so() {
        deleteAnItemThatAnotherShows()

        compose.onNodeWithText("Delete").performClick()

        compose.waitUntil(WAIT_MS) { shell.view.outline.rows.size == 2 }
        compose.onNodeWithText("Cancel").assertDoesNotExist()
    }

    // キーボードを閉じる道具は、編集を終える。項目は、飾りのついた表示に戻り、選ばれたまま残る。
    @Test
    fun the_tool_that_hides_the_keyboard_ends_the_edit_and_selects_the_item() {
        download("notes.md", "a **bold** word" + NL)
        openNotes()
        waitFor("a bold word")
        touchEnd("a bold word")

        tool("Hide keyboard")

        waitFor("a bold word")
        compose.onNodeWithTag(EDITOR_TAG).assertDoesNotExist()
        compose.waitUntil(WAIT_MS) { shell.view.outline.rows.single().isSelected }
        compose.onNode(hasContentDescription("Show keyboard")).assertExists()
    }

    // キーボードを出す道具は、選んでいた項目の編集に戻る。
    @Test
    fun the_tool_that_shows_the_keyboard_edits_the_selected_item() {
        download("notes.md", "a **bold** word" + NL)
        openNotes()
        waitFor("a bold word")
        touchEnd("a bold word")
        tool("Hide keyboard")
        compose.waitUntil(WAIT_MS) { shell.view.outline.rows.single().isSelected }

        tool("Show keyboard")

        compose.onNodeWithTag(EDITOR_TAG).assertExists()
        compose.waitUntil(WAIT_MS) { !shell.view.outline.rows.single().isSelected }
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

    /** Puts the cursor of the field before the UTF-16 unit [at]. */
    private fun cursorAt(at: Int) =
        compose.onNodeWithTag(EDITOR_TAG).performSemanticsAction(SemanticsActions.SetSelection) {
            it(at, at, true)
        }

    // 文の途中での改行は、カーソルの位置で項目を分ける。絵文字は、1文字として数える。
    @Test
    fun enter_in_the_middle_splits_the_item_at_the_cursor_and_counts_an_emoji_as_one_character() {
        download("notes.md", "😀ab" + NL)
        openNotes()
        waitFor("😀ab")
        touchEnd("😀ab")
        cursorAt(3)

        compose.onNodeWithTag(EDITOR_TAG).performTextInput(NL)

        compose.waitUntil(WAIT_MS) { shell.view.outline.rows.size == 2 }
        assertEquals(listOf("😀a", "b"), shell.view.outline.rows.map { it.item.body })
    }

    // 空の子の項目での改行は、項目を1段外へ出す。編集は続き、次の改行は、新しい項目を作る。
    @Test
    fun enter_in_an_empty_child_moves_it_out_and_the_next_enter_makes_a_new_item() {
        download("notes.md", "first" + NL)
        openNotes()
        waitFor("first")
        touchEnd("first")
        compose.onNodeWithTag(EDITOR_TAG).performTextInput(NL)
        waitForItems("1 first", "1 ")
        pad("Move block", Arrow.Right)
        waitForItems("1 first", "2 ")

        compose.onNodeWithTag(EDITOR_TAG).performTextInput(NL)
        waitForItems("1 first", "1 ")
        compose.onNodeWithTag(EDITOR_TAG).performTextInput(NL)

        compose.waitUntil(WAIT_MS) { shell.view.outline.rows.size == 3 }
    }

    // 長いページの末尾で改行すると、新しい項目は画面の外にできる。画面は、その項目まで動く。
    @Test
    fun the_screen_moves_to_a_new_item_that_is_made_below_what_is_shown() {
        val ids = ('0'..'9') + ('A'..'Z') + ('a'..'d')
        download("notes.md", ids.joinToString("") { above(it) + "item $it" + NL })
        openNotes()
        waitFor("item 0")
        val before = shell.view.outline.rows.size
        compose.onNode(hasScrollToNodeAction()).performScrollToNode(hasText("item d"))
        touchEnd("item d")

        compose.onNodeWithTag(EDITOR_TAG).performTextInput(NL)

        compose.waitUntil(WAIT_MS) { shell.view.outline.rows.size == before + 1 }
        compose.onNodeWithTag(EDITOR_TAG).assertIsDisplayed().assertTextEquals("")
    }

    /** Opens a page of two items, and touches the second to edit it. */
    private fun editSecondOfTwo(first: String, second: String) {
        download("notes.md", above('A') + first + NL + NL + above('B') + second + NL)
        openNotes()
        waitFor(second)
        touchEnd(second)
        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals(second)
    }

    private fun pressBackspace() =
        compose.onNodeWithTag(EDITOR_TAG).performKeyInput { pressKey(Key.Backspace) }

    /** Where the cursor of the field is, in UTF-16 units. */
    private fun cursor(): TextRange =
        compose
            .onNodeWithTag(EDITOR_TAG)
            .fetchSemanticsNode()
            .config[SemanticsProperties.TextSelectionRange]

    // 先頭でのBackspaceは、項目を前の項目につなぐ。編集は、つないだ項目で続く。カーソルは、つなぎ目に置かれる。
    @Test
    fun backspace_at_the_start_joins_the_item_to_the_one_before_with_the_cursor_where_they_meet() {
        editSecondOfTwo("first", "second")
        cursorAt(0)

        pressBackspace()

        waitForItems("1 firstsecond")
        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals("firstsecond")
        compose.waitUntil(WAIT_MS) { cursor() == TextRange(5) }
        waitForWritten("firstsecond")
    }

    // つなぎ目のカーソルは、絵文字を1文字として数えた位置に置かれる。
    @Test
    fun the_cursor_after_a_join_counts_an_emoji_before_it_as_one_character() {
        editSecondOfTwo("a😀", "b")
        cursorAt(0)

        pressBackspace()

        waitForItems("1 a😀b")
        compose.waitUntil(WAIT_MS) { cursor() == TextRange(3) }
    }

    // 空の項目の先頭でのBackspaceは、その項目を消す。編集は、前の項目の終わりで続く。
    @Test
    fun backspace_in_an_empty_item_takes_it_away_and_the_edit_goes_on_at_the_end_of_the_one_before() {
        download("notes.md", "first" + NL)
        openNotes()
        waitFor("first")
        touchEnd("first")
        compose.onNodeWithTag(EDITOR_TAG).performTextInput(NL)
        waitForItems("1 first", "1 ")
        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals("")

        pressBackspace()

        waitForItems("1 first")
        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals("first")
        compose.waitUntil(WAIT_MS) { cursor() == TextRange(5) }
    }

    // 先頭より後ろでのBackspaceは、前の1文字を消すだけである。
    @Test
    fun backspace_after_the_start_only_deletes_the_character_before_the_cursor() {
        editSecondOfTwo("first", "second")
        cursorAt(1)

        pressBackspace()

        waitForItems("1 first", "1 econd")
    }

    // 先頭から文字を選んでいるときのBackspaceは、選んだ文字を消すだけである。
    @Test
    fun backspace_with_text_chosen_from_the_start_only_deletes_what_is_chosen() {
        editSecondOfTwo("first", "second")
        compose.onNodeWithTag(EDITOR_TAG).performSemanticsAction(SemanticsActions.SetSelection) {
            it(0, 3, true)
        }

        pressBackspace()

        waitForItems("1 first", "1 ond")
    }

    // 最初の項目の先頭でのBackspaceは、何もしない。
    @Test
    fun backspace_at_the_start_of_the_first_item_does_nothing() {
        download("notes.md", above('A') + "first" + NL + NL + above('B') + "second" + NL)
        openNotes()
        waitFor("second")
        touchEnd("first")
        cursorAt(0)

        pressBackspace()
        compose.onNodeWithTag(EDITOR_TAG).performTextInput("x")

        waitForItems("1 xfirst", "1 second")
    }

    /**
     * Presses Backspace as the keyboard on the screen does. Gboard sends no key: it asks the field
     * to delete the character before the cursor, also when there is none.
     */
    /**
     * Puts [text] in at the cursor as the keyboard on the screen does, as with what it shows of the
     * clipboard.
     */
    private fun commitOnTheScreen(text: String) {
        val root = compose.onNodeWithTag(EDITOR_TAG).fetchSemanticsNode().root as ViewRootForTest
        compose.runOnUiThread {
            val keyboard = checkNotNull(root.view.onCreateInputConnection(EditorInfo()))
            keyboard.commitText(text, 1)
        }
    }

    private fun pressBackspaceOnTheScreen() {
        val root = compose.onNodeWithTag(EDITOR_TAG).fetchSemanticsNode().root as ViewRootForTest
        compose.runOnUiThread {
            val keyboard = checkNotNull(root.view.onCreateInputConnection(EditorInfo()))
            keyboard.deleteSurroundingText(1, 0)
        }
    }

    // 画面のキーボードのBackspaceでも、先頭では項目を前の項目につなぐ。画面のキーボードは、キーを送らない。
    @Test
    fun backspace_of_the_keyboard_on_the_screen_at_the_start_joins_the_item_to_the_one_before() {
        editSecondOfTwo("first", "second")
        cursorAt(0)

        pressBackspaceOnTheScreen()

        waitForItems("1 firstsecond")
        compose.waitUntil(WAIT_MS) { cursor() == TextRange(5) }
    }

    // 画面のキーボードのBackspaceは、先頭より後ろでは、前の1文字を消すだけである。
    @Test
    fun backspace_of_the_keyboard_on_the_screen_after_the_start_only_deletes_a_character() {
        editSecondOfTwo("first", "second")
        cursorAt(1)

        pressBackspaceOnTheScreen()

        waitForItems("1 first", "1 econd")
    }

    // 画面のキーボードのBackspaceを押し続けると、描き直しの直後に次のBackspaceが来ても、残った文字の末尾にカーソルがある。
    @Test
    fun holding_backspace_of_the_keyboard_on_the_screen_keeps_the_cursor_at_the_end_of_what_is_left() {
        editSecondOfTwo("first", "abcdefg")
        val root = compose.onNodeWithTag(EDITOR_TAG).fetchSemanticsNode().root as ViewRootForTest
        // As on a slow phone: each Backspace after the first comes just after the screen was drawn
        // again
        var more = 3
        afterDrawn = {
            if (more > 0) {
                more -= 1
                checkNotNull(root.view.onCreateInputConnection(EditorInfo()))
                    .deleteSurroundingText(1, 0)
            }
        }

        pressBackspaceOnTheScreen()

        waitForItems("1 first", "1 abc")
        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals("abc")
        assertEquals(TextRange(3), cursor())
    }

    // 画面のキーボードのBackspaceは、先頭から文字を選んでいるときは、項目をつながない。
    @Test
    fun backspace_of_the_keyboard_on_the_screen_with_text_chosen_from_the_start_joins_nothing() {
        editSecondOfTwo("first", "second")
        compose.onNodeWithTag(EDITOR_TAG).performSemanticsAction(SemanticsActions.SetSelection) {
            it(0, 3, true)
        }

        pressBackspaceOnTheScreen()
        compose.onNodeWithTag(EDITOR_TAG).performTextInput("x")

        waitForItems("1 first", "1 xond")
    }
}
