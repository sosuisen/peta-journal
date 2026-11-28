package com.sosuisha.petajournal.screen

import androidx.compose.ui.test.hasContentDescription
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.swipeUp
import androidx.test.ext.junit.runners.AndroidJUnit4
import java.io.File
import java.time.LocalDate
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class JournalScrollTest : JournalScreenFixture() {
    // 今日のページを下までスクロールすると、前の日が読み足され、最初に読んだ日より前の日も見える（#1145）。
    @Test
    fun scrolling_the_page_of_today_down_loads_the_days_before() {
        // More days than the journal loads at first: each day has one item
        val days = 130
        val folder = File(root, "markdown/journals").apply { mkdirs() }
        val today = LocalDate.now()
        for (ago in 1..days) {
            File(folder, "${today.minusDays(ago.toLong())}.md").writeText("day $ago$NL")
        }
        start()
        compose.onNode(hasContentDescription("Today")).performClick()
        waitFor("day 1")

        val oldest = "day $days"
        for (swipe in 0 until 60) {
            if (isShown(oldest)) break
            compose.onRoot().performTouchInput { swipeUp(durationMillis = 100) }
            compose.waitForIdle()
        }

        assertTrue(isShown(oldest))
    }

    // No other day has a name that "day 130" is a part of
    private fun isShown(text: String) =
        compose.onAllNodes(shown(text)).fetchSemanticsNodes().isNotEmpty()
}
