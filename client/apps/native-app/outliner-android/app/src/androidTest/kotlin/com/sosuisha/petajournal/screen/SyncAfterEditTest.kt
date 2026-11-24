package com.sosuisha.petajournal.screen

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
import org.junit.Test
import org.junit.runner.RunWith

/** How long the sync of these tests waits after the last change of a page. */
private const val AFTER_CHANGE_MS = 1_500L

/**
 * The sync that starts after an edit, while the user does not wait for it (ADR 055).
 *
 * The tests tell the shell that the keyboard composes a word, as the editor does. A test cannot
 * compose a word in the field itself: the keyboard of the emulator commits it at once.
 */
@RunWith(AndroidJUnit4::class)
class SyncAfterEditTest : JournalScreenFixture() {
    // キーボードが語を変換している間は、編集のあとの同期は始まらない。変換が終わってから待ち、同期する。
    @Test
    fun no_sync_starts_while_the_keyboard_composes_a_word() {
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
        compose.onNodeWithTag(EDITOR_TAG).performTextInput("zz9")
        compose.waitForIdle()

        // Twice the wait, with a word composed all the time
        repeat(6) {
            compose.runOnUiThread { shell.composing() }
            Thread.sleep(AFTER_CHANGE_MS / 3)
        }

        assertFalse(cloud.uploaded["docs/trip.md"].orEmpty().contains("zz9"))
        compose.waitUntil(WAIT_MS) { cloud.uploaded["docs/trip.md"].orEmpty().contains("zz9") }
    }
}
