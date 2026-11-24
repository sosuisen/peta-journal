package com.sosuisha.petajournal.screen

import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.hasContentDescription
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.isEnabled
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Test
import org.junit.runner.RunWith

/** The buttons that go back and forward among the pages that were open (ADR 035). */
@RunWith(AndroidJUnit4::class)
class HistoryTest : JournalScreenFixture() {
    /** Opens `notes`, and from there the page `other` by its link. */
    private fun openOther() {
        download("notes.md", "[[other]]" + NL)
        download("other.md", "what the other page says" + NL)
        openNotes()
        waitFor("other")
        compose.onNode(hasText("other"), useUnmergedTree = true).performClick()
        waitFor("what the other page says")
    }

    private fun waitForGone(text: String) =
        compose.waitUntil(WAIT_MS) {
            compose.onAllNodes(shown(text)).fetchSemanticsNodes().isEmpty()
        }

    // 戻るのボタンで前のページが開き、進むのボタンでそのあとのページが開く。
    @Test
    fun back_opens_the_page_before_and_forward_the_page_after() {
        openOther()

        compose.onNodeWithContentDescription("Back").performClick()
        waitForGone("what the other page says")
        waitFor("notes")

        compose.onNodeWithContentDescription("Forward").performClick()
        waitFor("what the other page says")
    }

    // 戻る先も進む先もないときは、2つのボタンは出たまま押せない。
    @Test
    fun the_buttons_cannot_be_touched_without_a_page_in_their_direction() {
        start()

        compose.onNodeWithContentDescription("Back").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Forward").assertIsNotEnabled()
    }

    // ページを2つ開くと戻るのボタンが押せるようになり、戻ると進むのボタンが押せるようになる。
    @Test
    fun the_buttons_can_be_touched_when_a_page_lies_in_their_direction() {
        openOther()
        compose.onNodeWithContentDescription("Back").assertIsEnabled()
        compose.onNodeWithContentDescription("Forward").assertIsNotEnabled()

        compose.onNodeWithContentDescription("Back").performClick()

        compose.waitUntil(WAIT_MS) {
            compose
                .onAllNodes(hasContentDescription("Forward") and isEnabled())
                .fetchSemanticsNodes()
                .isNotEmpty()
        }
        compose.onNodeWithContentDescription("Back").assertIsNotEnabled()
    }
}
