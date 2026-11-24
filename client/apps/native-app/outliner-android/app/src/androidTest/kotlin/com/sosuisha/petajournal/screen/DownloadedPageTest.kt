package com.sosuisha.petajournal.screen

import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.click
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.test.performTouchInput
import androidx.test.ext.junit.runners.AndroidJUnit4
import java.io.File
import org.junit.Test
import org.junit.runner.RunWith

/** A page that a download changed while it is open. */
@RunWith(AndroidJUnit4::class)
class DownloadedPageTest : JournalScreenFixture() {
    // 開いているページのファイルがダウンロードで変わると、取り込んだあと、触れなくても画面に出る。
    @Test
    fun an_open_page_that_a_download_changed_shows_the_change_without_a_touch() {
        download("notes.md", "first" + NL)
        openNotes()
        waitFor("first")

        download("notes.md", "changed elsewhere" + NL)
        // Later than the file that the core took in, as a download that comes after is
        val file = File(root, "markdown/docs/notes.md")
        file.setLastModified(file.lastModified() + 10_000)
        compose.runOnUiThread { shell.sync() }

        waitFor("changed elsewhere")
    }

    // ほかの端末が項目の本文だけを書き換えたファイルでも、取り込んだあと、触れなくても画面に出る。項目のIDは変わらない。
    @Test
    fun an_open_page_whose_item_changed_elsewhere_shows_the_change_without_a_touch() {
        download("notes.md", "first" + NL + "second" + NL)
        openNotes()
        waitFor("second")
        val file = File(root, "markdown/docs/notes.md")
        // The core writes the ids of the items above them when it has taken the file in
        compose.waitUntil(WAIT_MS) { file.readText().contains("<!--") }

        file.writeText(file.readText().replace("second", "changed elsewhere"))
        file.setLastModified(file.lastModified() + 10_000)
        compose.runOnUiThread { shell.sync() }

        waitFor("changed elsewhere")
    }

    private fun touchEnd(text: String) =
        compose.onNodeWithText(text).performTouchInput { click(centerRight) }

    /** A page `notes` with the items "first" and "second", open. */
    private fun openFirstAndSecond(): File {
        download("notes.md", above('A') + "first" + NL + NL + above('B') + "second" + NL)
        openNotes()
        waitFor("second")
        return File(root, "markdown/docs/notes.md")
    }

    /** Another device rewrote [text] in the file, and the sync takes the file in. */
    private fun changedElsewhere(file: File, text: String, into: String) {
        file.writeText(file.readText().replace(text, into))
        file.setLastModified(file.lastModified() + 10_000)
        compose.runOnUiThread { shell.sync() }
    }

    private fun waitForEditor(text: String) =
        compose.waitUntil(WAIT_MS) {
            compose
                .onAllNodes(hasTestTag(EDITOR_TAG) and hasText(text))
                .fetchSemanticsNodes()
                .isNotEmpty()
        }

    // 項目を編集している間でも、ほかの端末が別の項目を書き換えたファイルは、取り込んだあとに画面に出る。編集は続く（ADR 069）。
    @Test
    fun a_change_elsewhere_to_another_item_shows_while_an_item_is_edited() {
        val file = openFirstAndSecond()
        touchEnd("first")
        waitForEditor("first")

        changedElsewhere(file, "second", "changed elsewhere")

        waitFor("changed elsewhere")
        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals("first")
    }

    // 編集中の項目をほかの端末が書き換えると、こちらで打っていなければ、入力欄にその文が出る。カーソルは同じ文字数の位置に残り、続けて打てる。
    @Test
    fun a_change_elsewhere_to_the_edited_item_shows_in_its_field_when_nothing_was_typed() {
        val file = openFirstAndSecond()
        touchEnd("first")
        waitForEditor("first")

        changedElsewhere(file, "first", "changed elsewhere")

        waitForEditor("changed elsewhere")
        // The cursor was at the end of "first", 5 letters in
        compose.onNodeWithTag(EDITOR_TAG).performTextInput("!")
        waitForEditor("chang!ed elsewhere")
    }

    // 編集中の項目に打った文は、ほかの端末がその項目を書き換えても残る。
    @Test
    fun text_typed_into_the_edited_item_stays_when_it_changed_elsewhere() {
        val file = openFirstAndSecond()
        touchEnd("first")
        waitForEditor("first")
        compose.onNodeWithTag(EDITOR_TAG).performTextInput(" typed")
        waitForEditor("first typed")

        // The item "second" changes too: when it shows, the page has been read again
        file.writeText(
            file
                .readText()
                .replace("first", "changed elsewhere")
                .replace("second", "second changed"),
        )
        file.setLastModified(file.lastModified() + 10_000)
        compose.runOnUiThread { shell.sync() }
        waitFor("second changed")

        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals("first typed")
    }
}
