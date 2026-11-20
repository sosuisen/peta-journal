package com.sosuisha.petajournal.screen

import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import androidx.compose.ui.platform.ViewRootForTest
import androidx.compose.ui.test.hasContentDescription
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.onFirst
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.sosuisha.petajournal.sync.SyncState
import org.junit.Assert.assertFalse
import org.junit.runner.RunWith
import org.junit.Test

/** How long the sync of these tests waits after the last change of a page. */
private const val AFTER_CHANGE_MS = 1_500L

/** The sync that starts after an edit, while the user does not wait for it (ADR 055). */
@RunWith(AndroidJUnit4::class)
class SyncAfterEditTest : JournalScreenFixture() {
    /** The keyboard on the screen of the item that is edited, as an input method sees it. */
    private fun keyboard(): InputConnection {
        val root = compose.onNodeWithTag(EDITOR_TAG).fetchSemanticsNode().root as ViewRootForTest
        var keyboard: InputConnection? = null
        compose.runOnUiThread { keyboard = root.view.onCreateInputConnection(EditorInfo()) }
        return checkNotNull(keyboard)
    }

    // 入力メソッドが変換している間は、編集のあとの同期は始まらない。変換が確定してから待ち、同期する。
    @Test
    fun no_sync_starts_while_the_input_method_composes_a_word() {
        val cloud = OnePage()
        start(cloud, afterChangeMs = AFTER_CHANGE_MS)
        compose.onNode(hasContentDescription("Menu")).performClick()
        compose.onNodeWithText("Sync").performClick()
        compose.waitUntil(WAIT_MS) { sync?.state == SyncState.Idle && !shell.syncing }
        type("trip")
        waitFor("trip")
        compose.onAllNodes(shown("trip")).onFirst().performClick()
        waitFor("京都")
        compose.onNode(hasText("京都", substring = true), useUnmergedTree = true).performClick()
        compose.onNodeWithTag(EDITOR_TAG).performTextInput("x")
        compose.waitForIdle()

        val keyboard = keyboard()
        for (word in listOf("か", "かい", "かいぎ", "かいぎを", "会議を", "会議を", "会議を", "会議を")) {
            compose.runOnUiThread { keyboard.setComposingText(word, 1) }
            Thread.sleep(AFTER_CHANGE_MS / 3)
        }

        assertFalse(cloud.uploaded["docs/trip.md"].orEmpty().contains("x"))
        compose.runOnUiThread { keyboard.finishComposingText() }
        compose.waitUntil(WAIT_MS) { cloud.uploaded["docs/trip.md"].orEmpty().contains("x会議を") }
    }
}
