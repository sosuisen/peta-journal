package net.sosuisen.petajournal

import android.graphics.Bitmap
import android.os.Handler
import android.os.Looper
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.assertHeightIsEqualTo
import androidx.compose.ui.test.assertWidthIsEqualTo
import androidx.compose.ui.test.hasAnyAncestor
import androidx.compose.ui.test.isDialog
import androidx.compose.ui.unit.dp
import androidx.compose.ui.test.hasContentDescription
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
import net.sosuisen.petajournal.sync.CloudRecord
import net.sosuisen.petajournal.sync.CloudSync
import net.sosuisen.petajournal.sync.Downloader
import net.sosuisen.petajournal.sync.MarkdownFolder
import net.sosuisen.petajournal.sync.SyncRound
import net.sosuisen.petajournal.sync.SyncState
import net.sosuisen.petajournal.sync.Uploader
import net.sosuisen.petajournal.sync.dropboxContentHash
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
import java.security.MessageDigest

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
        val record = CloudRecord(File(root, "record"))
        sync = cloud?.let {
            val round = { from: Cloud ->
                SyncRound(Downloader(from, folder, File(root, "cursor"), record), shell::syncAndWait, Uploader(from, folder, record))
            }
            CloudSync(signedIn, round) { main.post(it) }
        }
        compose.setContent {
            JournalScreen(
                shell,
                sync?.state,
                onSignIn = { signIns += 1 },
                onSync = { sync?.start(checkNotNull(cloud)) },
                onSignOut = {
                    signOuts += 1
                    sync?.signOut()
                },
                onFile = { opened += it },
            )
        }
        compose.waitUntil(WAIT_MS) { !shell.syncing }
    }

    private var signIns = 0
    private var signOuts = 0
    private val opened = mutableListOf<File>()
    private var sync: CloudSync? = null

    /** A cloud with one page, or one that cannot be reached. It keeps what is uploaded. */
    private class OnePage(private val reachable: Boolean = true) : Cloud {
        private val text = "京都で会議がありました。\n".toByteArray()

        /** The text of each uploaded file, by its path. */
        val uploaded = mutableMapOf<String, String>()

        override fun changes(cursor: String?): Changes {
            if (!reachable) throw IOException("the network is gone")
            val whole = cursor == null
            val page = Change.File("docs/trip.md", hashOf(text), remote = "first")
            return Changes(if (whole) listOf(page) else emptyList(), "c1", whole)
        }

        override fun download(file: Change.File, into: OutputStream) = into.write(text)

        override fun upload(path: String, from: File, over: String?): Change.File {
            uploaded[path] = from.readText()
            return Change.File(path, dropboxContentHash(from), remote = "next")
        }

        /** The content hash of Dropbox of a content of one block. */
        private fun hashOf(content: ByteArray): String {
            val sha = { bytes: ByteArray -> MessageDigest.getInstance("SHA-256").digest(bytes) }
            return sha(sha(content)).joinToString("") { "%02x".format(it) }
        }
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

    /** Puts a picture into the assets folder, as a download does. */
    private fun picture(name: String, width: Int, height: Int) {
        val file = File(root, "markdown/assets/$name").apply { parentFile?.mkdirs() }
        val bitmap = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
        file.outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
    }

    private fun waitForPicture(alt: String) =
        compose.waitUntil(WAIT_MS) { compose.onAllNodes(hasContentDescription(alt)).fetchSemanticsNodes().isNotEmpty() }

    // assetsの画像は、前後の文字のあいだに、絵として見える。ファイルの名前の大文字と小文字は問わない。
    @Test
    fun an_image_of_the_assets_folder_shows_between_the_text_around_it() {
        picture("cat.png", width = 40, height = 20)
        download("notes.md", "before ![a cat](../assets/Cat.png) after" + NL)

        openNotes()

        waitForPicture("a cat")
        compose.onNode(hasText("before")).assertExists()
        compose.onNode(hasText("after")).assertExists()
    }

    // 画像は、ページが決めた幅で見える。幅がなければ、画像そのものの幅で見える。
    @Test
    fun an_image_has_the_width_that_the_page_asks_for() {
        picture("cat.png", width = 40, height = 20)
        download("notes.md", "![asked](../assets/cat.png#w=120)" + NL + NL + "![natural](../assets/cat.png)" + NL)

        openNotes()

        waitForPicture("asked")
        compose.onNode(hasContentDescription("asked")).assertWidthIsEqualTo(120.dp).assertHeightIsEqualTo(60.dp)
        compose.onNode(hasContentDescription("natural")).assertWidthIsEqualTo(40.dp)
    }

    // ページが高さも決めた画像は、その高さで見える。高さだけなら、幅は画像の形に従う。
    @Test
    fun an_image_has_the_height_that_the_page_asks_for() {
        picture("cat.png", width = 40, height = 20)
        download("notes.md", "![both](../assets/cat.png#w=120&h=30)" + NL + NL + "![tall](../assets/cat.png#h=60)" + NL)

        openNotes()

        waitForPicture("both")
        compose.onNode(hasContentDescription("both")).assertWidthIsEqualTo(120.dp).assertHeightIsEqualTo(30.dp)
        compose.onNode(hasContentDescription("tall")).assertWidthIsEqualTo(120.dp).assertHeightIsEqualTo(60.dp)
    }

    // 画像に触れると、その画像が大きく開く。開いた画像に触れると、閉じる。
    @Test
    fun a_touch_opens_an_image_large_and_another_touch_closes_it() {
        picture("cat.png", width = 40, height = 20)
        download("notes.md", "![a cat](../assets/cat.png)" + NL)
        openNotes()
        waitForPicture("a cat")
        val large = hasContentDescription("a cat") and hasAnyAncestor(isDialog())
        compose.onNode(large).assertDoesNotExist()

        compose.onNode(hasContentDescription("a cat")).performClick()

        compose.waitUntil(WAIT_MS) { compose.onAllNodes(large).fetchSemanticsNodes().isNotEmpty() }
        compose.onNode(large).performClick()
        compose.waitUntil(WAIT_MS) { compose.onAllNodes(large).fetchSemanticsNodes().isEmpty() }
        compose.onNode(hasContentDescription("a cat")).assertExists()
    }

    // ファイルのない画像は、代わりの文字（alt）が見える。
    @Test
    fun an_image_whose_file_is_not_here_shows_its_alt() {
        download("notes.md", "![a dog](../assets/dog.png)" + NL)

        openNotes()

        waitFor("a dog")
    }

    // Web上の画像は描かない。代わりの文字のリンクになる。
    @Test
    fun an_image_of_the_web_is_a_link_and_not_a_picture() {
        download("notes.md", "see ![a bird](https://example.com/bird.png) here" + NL)

        openNotes()

        waitFor("see a bird here")
        compose.onNode(hasContentDescription("a bird")).assertDoesNotExist()
    }

    // リストの中のコードブロックも、行が分かれたまま見える。リストの項目は、番号と一緒に残る。
    @Test
    fun a_code_block_inside_a_list_shows_its_lines_as_they_were_written() {
        download(
            "notes.md",
            above('A') + "1. step one" + NL + NL + "   ```" + NL + "   val x = 1" + NL + "     val y = 2" + NL + "   ```" + NL +
                "2. step two" + NL,
        )

        openNotes()

        waitFor("step one")
        compose.onNode(hasText("val x = 1\n  val y = 2")).assertExists()
        for (text in listOf("1.", "step one", "2.", "step two")) compose.onNode(hasText(text)).assertExists()
    }

    // 引用の中の表も、セルごとに分かれて見える。
    @Test
    fun a_table_inside_a_quote_shows_each_cell_apart() {
        download("notes.md", above('A') + "> said:" + NL + ">" + NL + "> | name | count |" + NL + "> |---|---|" + NL + "> | apple | 2 |" + NL)

        openNotes()

        waitFor("said:")
        for (cell in listOf("name", "count", "apple", "2")) compose.onNode(hasText(cell)).assertExists()
    }

    // リストの中の画像も、絵として見える。リストの中のリストでも同じである。
    @Test
    fun an_image_inside_a_list_shows_as_a_picture() {
        picture("cat.png", width = 40, height = 20)
        download("notes.md", above('A') + "- outer" + NL + "  - look ![a cat](../assets/cat.png)" + NL + "- next" + NL)

        openNotes()

        waitForPicture("a cat")
        for (text in listOf("outer", "look", "next")) compose.onNode(hasText(text)).assertExists()
    }

    // assetsのファイルへのリンクに触れると、そのファイルが開かれる。名前の大文字と小文字は問わない。
    @Test
    fun a_link_to_a_file_of_the_assets_folder_opens_the_file() {
        File(root, "markdown/assets").mkdirs()
        File(root, "markdown/assets/report.pdf").writeText("pdf")
        download("notes.md", "[the report](../assets/Report.pdf)" + NL)
        openNotes()
        waitFor("the report")

        compose.onNode(hasText("the report")).performClick()

        compose.runOnIdle { assertEquals(listOf("report.pdf"), opened.map { it.name }) }
    }

    // ファイルのないリンクに触れると、ないことが見える。何も開かれない。
    @Test
    fun a_link_to_a_file_that_is_not_here_says_so() {
        download("notes.md", "[the report](../assets/none.pdf)" + NL)
        openNotes()
        waitFor("the report")

        compose.onNode(hasText("the report")).performClick()

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

        compose.onNode(hasText("other page")).performClick()

        waitFor("what the other page says")
    }

    // .mdへの相対リンクに触れても、そのページが開く。
    @Test
    fun a_relative_link_to_an_md_file_opens_its_page() {
        download("notes.md", "[the other](./other.md)" + NL)
        download("other.md", "what the other page says" + NL)
        openNotes()
        waitFor("the other")

        compose.onNode(hasText("the other")).performClick()

        waitFor("what the other page says")
    }

    // タグに触れると、そのタグのページが開く。
    @Test
    fun a_tag_opens_the_page_of_the_tag() {
        download("notes.md", "#topic" + NL)
        openNotes()
        waitFor("#topic")

        compose.onNode(hasText("#topic")).performClick()

        compose.waitUntil(WAIT_MS) { compose.onAllNodes(hasText("topic")).fetchSemanticsNodes().isNotEmpty() }
    }

    // まだないページへのリンクに触れても、ページは作られない。ないことが見え、元のページが開いたままになる。
    @Test
    fun a_link_to_a_page_that_is_not_there_says_so_and_makes_no_page() {
        download("notes.md", "[[ghost]]" + NL)
        openNotes()
        waitFor("ghost")

        compose.onNode(hasText("ghost")).performClick()

        waitFor("No page yet: ghost")
        compose.onNode(hasText("notes")).assertExists()
        type("ghost")
        compose.waitUntil(WAIT_MS) { shell.view.search.results.any { it.kind == SearchKind.CREATE } }
        assertEquals(emptyList<String>(), shell.view.search.results.filter { it.kind == SearchKind.PAGE }.map { it.title })
    }

    /** The line that the core writes above an item in a file, with the properties of the item. */
    private fun above(id: Char, properties: String = "") =
        "<!-- @ 2020-01-01-00-00-00-AAAAAA$id idx=\"a${id}\"$properties -->" + NL

    // タスクの項目には、その状態の印が付く。まだ始めていない、進行中、済みの3つである。
    @Test
    fun a_task_shows_the_mark_of_its_state() {
        download(
            "notes.md",
            above('A') + "- [ ] buy milk" + NL + NL +
                above('B', " begin=\"2020-01-01T00:00:00Z\"") + "- [ ] write the report" + NL + NL +
                above('C', " end=\"2020-01-02T00:00:00Z\"") + "- [x] call home" + NL + NL +
                above('D') + "not a task" + NL,
        )

        openNotes()

        waitFor("call home")
        // The mark of the file is not part of the text
        compose.onNode(hasText("buy milk")).assertExists()
        val marks = compose.onAllNodes(hasContentDescription("Task", substring = true)).fetchSemanticsNodes()
        assertEquals(
            listOf("Task not started", "Task running", "Task done"),
            marks.map { it.config[SemanticsProperties.ContentDescription].single() },
        )
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

    // アプリのフォルダーで変わったページのファイルは、次の同期でクラウドへ上がる。コアが取り込んだあとに上がる。
    @Test
    fun a_page_file_that_changed_here_is_uploaded_by_the_next_sync() {
        val cloud = OnePage()
        start(cloud)
        compose.onNodeWithText("Sync").performClick()
        compose.waitUntil(WAIT_MS) { File(root, "markdown/docs/trip.md").exists() }
        compose.waitUntil(WAIT_MS) { sync?.state == SyncState.Idle && !shell.syncing }
        File(root, "markdown/docs/trip.md").writeText("大阪へ行きました。\n")

        compose.onNodeWithText("Sync").performClick()

        compose.waitUntil(WAIT_MS) { cloud.uploaded["docs/trip.md"].orEmpty().contains("大阪") }
        type("大阪")
        waitFor("trip")
    }

    // サインインの前は、サインインのボタンが出る。押すと、サインインが始まる。
    @Test
    fun before_the_sign_in_a_button_starts_it() {
        start(OnePage(), signedIn = false)

        compose.onNodeWithText("Sign in to Dropbox").performClick()

        compose.runOnIdle { assertEquals(1, signIns) }
    }

    // サインアウトすると、サインインのボタンに戻る。取り込んだページは、そのまま読める。
    @Test
    fun a_sign_out_brings_the_sign_in_back_and_keeps_the_pages() {
        start(OnePage())
        compose.onNodeWithText("Sync").performClick()
        compose.waitUntil(WAIT_MS) { File(root, "markdown/docs/trip.md").exists() }
        compose.waitUntil(WAIT_MS) { sync?.state == SyncState.Idle && !shell.syncing }

        compose.onNodeWithText("Sign out").performClick()

        compose.onNodeWithText("Sign in to Dropbox").assertExists()
        compose.onNodeWithText("Sync").assertDoesNotExist()
        compose.runOnIdle { assertEquals(1, signOuts) }
        type("会議")
        waitFor("trip")
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
        const val NL = "\n"
    }
}
