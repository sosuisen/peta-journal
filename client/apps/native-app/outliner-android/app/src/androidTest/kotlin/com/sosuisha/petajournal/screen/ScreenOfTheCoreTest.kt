package com.sosuisha.petajournal.screen

import androidx.test.ext.junit.runners.AndroidJUnit4
import com.sosuisha.petajournal.types.EditorKey
import com.sosuisha.petajournal.types.EditorState
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** What the shell keeps of the screen is what the core has (ADR 076). */
@RunWith(AndroidJUnit4::class)
class ScreenOfTheCoreTest : JournalScreenFixture() {
    private fun assertTheScreenOfTheCore() = compose.runOnUiThread {
        assertArrayEquals(shell.wholeViewOfTheCore().bincodeSerialize(), shell.view.bincodeSerialize())
    }

    // シェルが差分から組み立てた画面は、コアの画面と同じである。ページを開いたとき、入力したとき、項目を分けたとき、検索したときである。
    @Test
    fun the_screen_made_from_the_changes_is_that_of_the_core() {
        download("notes.md", "first" + NL)
        openNotes()
        waitFor("first")
        assertTheScreenOfTheCore()

        val first = shell.view.outline.rows.single().item.id
        compose.runOnUiThread {
            shell.edit(first)
            shell.type(first, "first, typed again")
        }
        compose.waitForIdle()
        assertTheScreenOfTheCore()

        compose.runOnUiThread {
            val text = "first, typed again"
            val end = text.length.toULong()
            assertTrue(shell.press(EditorKey.ENTER, first, EditorState(text, end, end, onFirstLine = true, onLastLine = true)))
        }
        compose.waitUntil(WAIT_MS) { shell.view.outline.rows.size == 2 }
        assertTheScreenOfTheCore()

        compose.runOnUiThread { shell.search("notes") }
        compose.waitUntil(WAIT_MS) { shell.view.search.results.isNotEmpty() }
        assertTheScreenOfTheCore()
    }
}
