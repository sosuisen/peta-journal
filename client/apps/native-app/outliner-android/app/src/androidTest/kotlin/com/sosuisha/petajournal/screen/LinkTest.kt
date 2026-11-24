package com.sosuisha.petajournal.screen

import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.sosuisha.petajournal.types.ScreenView
import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

/** What a touch on a link or a tag opens. */
@RunWith(AndroidJUnit4::class)
class LinkTest : JournalScreenFixture() {
    // assetsのファイルへのリンクに触れると、コアがブラウザで開くように命じても、そのファイルが開かれる。アプリにはサーバーがないため。名前の大文字と小文字は問わない（ADR 083）。
    @Test
    fun a_link_to_a_file_of_the_assets_folder_opens_the_file() {
        File(root, "markdown/assets").mkdirs()
        File(root, "markdown/assets/report.pdf").writeText("pdf")
        download("notes.md", "[the report](../assets/Report.pdf)" + NL)
        openNotes()
        waitFor("the report")

        compose.onNode(hasText("the report"), useUnmergedTree = true).performClick()

        compose.runOnIdle { assertEquals(listOf("report.pdf"), opened.map { it.name }) }
    }

    // ファイルのないリンクに触れると、ないことが見える。何も開かれない。
    @Test
    fun a_link_to_a_file_that_is_not_here_says_so() {
        download("notes.md", "[the report](../assets/none.pdf)" + NL)
        openNotes()
        waitFor("the report")

        compose.onNode(hasText("the report"), useUnmergedTree = true).performClick()

        waitFor("No file yet: assets/none.pdf")
        assertEquals(emptyList<File>(), opened)
    }

    // ページへのリンクに触れると、そのページが開く。
    @Test
    fun a_link_to_a_page_opens_the_page() {
        download("notes.md", "[[other page]]" + NL)
        download("other page.md", "what the other page says" + NL)
        openNotes()
        waitFor("other page")

        compose.onNode(hasText("other page"), useUnmergedTree = true).performClick()

        waitFor("what the other page says")
    }

    // .mdへの相対リンクに触れても、そのページが開く。
    @Test
    fun a_relative_link_to_an_md_file_opens_its_page() {
        download("notes.md", "[the other](./other.md)" + NL)
        download("other.md", "what the other page says" + NL)
        openNotes()
        waitFor("the other")

        compose.onNode(hasText("the other"), useUnmergedTree = true).performClick()

        waitFor("what the other page says")
    }

    // タグに触れると、そのタグのページが開く。
    @Test
    fun a_tag_opens_the_page_of_the_tag() {
        download("notes.md", "#topic" + NL)
        openNotes()
        waitFor("#topic")

        compose.onNode(hasText("#topic"), useUnmergedTree = true).performClick()

        compose.waitUntil(WAIT_MS) {
            compose.onAllNodes(hasText("topic")).fetchSemanticsNodes().isNotEmpty()
        }
    }

    // まだないページへのリンクに触れると、ほかのシェルと同じく、そのページができて開き、ファイルが書かれる（ADR 083）。
    @Test
    fun a_link_to_a_page_that_is_not_there_makes_the_page_and_opens_it() {
        download("notes.md", "[[ghost]]" + NL)
        openNotes()
        waitFor("ghost")

        compose.onNode(hasText("ghost"), useUnmergedTree = true).performClick()

        compose.waitUntil(WAIT_MS) {
            (shell.view.screen as? ScreenView.Page)?.value?.title == "ghost"
        }
        compose.waitUntil(WAIT_MS) { isWritten("docs/ghost.md") }
    }

    // Webのアドレスへのリンクに触れると、そのアドレスがブラウザーで開かれる。
    @Test
    fun a_link_to_a_web_address_opens_it_in_the_browser() {
        download("notes.md", "[the site](https://example.com/page)" + NL)
        openNotes()
        waitFor("the site")

        compose.onNode(hasText("the site"), useUnmergedTree = true).performClick()

        compose.runOnIdle { assertEquals(listOf("https://example.com/page"), browsed) }
    }

    // http以外のスキームのリンクも、そのアドレスがOSに渡される（#1071）。
    @Test
    fun a_link_of_another_scheme_is_handed_to_the_os() {
        download("notes.md", "[write to me](mailto:someone@example.com)" + NL)
        openNotes()
        waitFor("write to me")

        compose.onNode(hasText("write to me"), useUnmergedTree = true).performClick()

        compose.runOnIdle { assertEquals(listOf("mailto:someone@example.com"), browsed) }
    }
}
