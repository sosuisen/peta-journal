package net.sosuisen.petajournal

import android.os.Handler
import android.os.Looper
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.hasSetTextAction
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onFirst
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import net.sosuisen.petajournal.sync.Change
import net.sosuisen.petajournal.sync.Changes
import net.sosuisen.petajournal.sync.Cloud
import net.sosuisen.petajournal.sync.CloudSync
import net.sosuisen.petajournal.sync.Downloader
import net.sosuisen.petajournal.sync.MarkdownFolder
import net.sosuisen.petajournal.sync.SyncState
import net.sosuisen.petajournal.types.SearchKind
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.io.IOException
import java.io.OutputStream

/** The screen with a real core under it, on the folders of a test. */
@RunWith(AndroidJUnit4::class)
class JournalScreenTest {
    @get:Rule
    val compose = createComposeRule()

    private lateinit var root: File
    private lateinit var shell: Shell

    @Before
    fun emptyFolders() {
        val cache = InstrumentationRegistry.getInstrumentation().targetContext.cacheDir
        root = File(cache, "journal-test").apply { deleteRecursively() }
        File(root, "markdown/docs").mkdirs()
    }

    @After
    fun close() {
        compose.runOnUiThread { shell.close() }
    }

    /**
     * Puts a doc page into the Markdown folder, as a download does. A file without the lines
     * that the core writes above each item is one item, with the whole text as its body.
     */
    private fun download(name: String, text: String) = File(root, "markdown/docs/$name").writeText(text)

    /**
     * Opens the app on the folders, and waits until the files are taken in. With a cloud, the
     * app syncs with it when "Sync" is touched; [signedIn] says whether the user has signed in.
     */
    private fun start(cloud: Cloud? = null, signedIn: Boolean = true) {
        val main = Handler(Looper.getMainLooper())
        compose.runOnUiThread {
            shell = Shell(File(root, "data"), File(root, "markdown")) { main.post(it) }
            shell.sync()
        }
        val folder = MarkdownFolder(File(root, "markdown"), File(root, "temp"))
        sync = cloud?.let {
            CloudSync(signedIn, { Downloader(it, folder, File(root, "cursor")) }, { main.post(it) }, shell::sync)
        }
        compose.setContent {
            JournalScreen(shell, sync?.state, onSignIn = { signIns += 1 }, onSync = { sync?.start(checkNotNull(cloud)) })
        }
        compose.waitUntil(WAIT_MS) { !shell.syncing }
    }

    private var signIns = 0
    private var sync: CloudSync? = null

    /** A cloud with one page, or one that cannot be reached. */
    private class OnePage(private val reachable: Boolean = true) : Cloud {
        private val text = "京都で会議がありました。\n".toByteArray()

        override fun changes(cursor: String?): Changes {
            if (!reachable) throw IOException("the network is gone")
            // The hash is of no use here: nothing is in the folder to compare with
            return Changes(listOf(Change.File("docs/trip.md", contentHash = "", remote = "")), "c1", whole = true)
        }

        override fun download(file: Change.File, into: OutputStream) = into.write(text)
    }

    private fun type(query: String) = compose.onNodeWithTag(SEARCH_TAG).performTextInput(query)

    /** A text on the screen that is not what the search field holds. */
    private fun shown(text: String): SemanticsMatcher = hasText(text, substring = true) and !hasSetTextAction()

    private fun waitFor(text: String) =
        compose.waitUntil(WAIT_MS) { compose.onAllNodes(shown(text)).fetchSemanticsNodes().isNotEmpty() }

    // 検索すると、取り込んだページが結果に出る。結果を選ぶと、そのページの項目が見える。
    @Test
    fun a_page_that_was_taken_in_is_found_by_the_search_and_opens() {
        download("notes.md", "今日は東京都で会議がありました。\n")
        start()

        type("会議")
        waitFor("notes")
        compose.onAllNodes(shown("notes")).onFirst().performClick()

        waitFor("今日は東京都で会議がありました。")
    }

    // 項目は、Markdownの記号ではなく、飾りの付いた文字で見える。
    @Test
    fun an_item_shows_its_decoration_and_not_the_marks_of_markdown() {
        download("notes.md", "some **bold** text\n")
        start()

        type("notes")
        waitFor("notes")
        compose.onAllNodes(shown("notes")).onFirst().performClick()

        waitFor("some bold text")
    }

    /** Opens the page `notes`, which a test has put into the Markdown folder. */
    private fun openNotes() {
        start()
        type("notes")
        waitFor("notes")
        compose.onAllNodes(shown("notes")).onFirst().performClick()
    }

    // コードブロックは、行が分かれたまま、書かれたとおりに見える。フェンスの記号は見えない。
    @Test
    fun a_code_block_shows_its_lines_as_they_were_written() {
        download("notes.md", "before\n\n```kotlin\nval x = 1\n  val y = 2\n```\n\nafter\n")

        openNotes()

        waitFor("before")
        compose.onNode(hasText("val x = 1\n  val y = 2")).assertExists()
        compose.onNode(hasText("after")).assertExists()
    }

    // 表は、セルごとに分かれて見える。セルの中の文字の飾りも付く。
    @Test
    fun a_table_shows_each_cell_apart() {
        download("notes.md", "| name | count |\n|---|---|\n| **apple** | 2 |\n")

        openNotes()

        waitFor("name")
        for (cell in listOf("name", "count", "apple", "2")) compose.onNode(hasText(cell)).assertExists()
    }

    // ページを作る結果（今日のジャーナル、入力した名前のページ）は、結果に出ない。最初の版は何も変えない。
    @Test
    fun the_results_that_make_a_page_are_left_out() {
        start()

        type("nothing-like-this")
        compose.waitUntil(WAIT_MS) { shell.view.search.results.any { it.kind == SearchKind.CREATE } }

        assertEquals(0, compose.onAllNodes(shown("nothing-like-this")).fetchSemanticsNodes().size)
        assertEquals(0, compose.onAllNodes(shown("Open Today")).fetchSemanticsNodes().size)
    }

    // クラウドのページは、同期のあと、検索で見つかる。
    @Test
    fun a_page_of_the_cloud_is_found_by_the_search_after_a_sync() {
        start(OnePage())

        compose.onNodeWithText("Sync").performClick()
        compose.waitUntil(WAIT_MS) { File(root, "markdown/docs/trip.md").exists() }
        // The file is there before the core has taken it in
        compose.waitUntil(WAIT_MS) { sync?.state == SyncState.Idle && !shell.syncing }
        type("会議")

        waitFor("trip")
    }

    // サインインの前は、サインインのボタンが出る。押すと、サインインが始まる。
    @Test
    fun before_the_sign_in_a_button_starts_it() {
        start(OnePage(), signedIn = false)

        compose.onNodeWithText("Sign in to Dropbox").performClick()

        compose.runOnIdle { assertEquals(1, signIns) }
    }

    // 同期が失敗すると、その理由が見える。
    @Test
    fun a_sync_that_failed_shows_why() {
        start(OnePage(reachable = false))

        compose.onNodeWithText("Sync").performClick()

        waitFor("Sync failed: the network is gone")
    }

    private companion object {
        /** The search waits a moment after the last key, and the first sync opens the index. */
        const val WAIT_MS = 10_000L
    }
}
