package net.sosuisen.petajournal.sync

import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.File
import java.io.IOException
import java.io.OutputStream
import java.security.MessageDigest

/** A cloud of a test: files by their path, and the changes it tells of at each cursor. */
private class FakeCloud : Cloud {
    val contents = mutableMapOf<String, ByteArray>()

    /** What `changes` answers, by the cursor it is asked with. */
    val answers = mutableMapOf<String?, Changes>()
    val askedWith = mutableListOf<String?>()
    val downloaded = mutableListOf<String>()
    var broken: String? = null

    /** A file of the cloud, as a change that tells of it. */
    fun file(path: String, text: String): Change.File {
        contents[path] = text.toByteArray()
        return Change.File(path, hashOf(text.toByteArray()), remote = path)
    }

    override fun changes(cursor: String?): Changes {
        askedWith += cursor
        return answers.getValue(cursor)
    }

    override fun download(file: Change.File, into: OutputStream) {
        if (file.path == broken) throw IOException("the network is gone")
        synchronized(downloaded) { downloaded += file.path }
        into.write(contents.getValue(file.remote))
    }
}

private fun sha256(bytes: ByteArray): ByteArray = MessageDigest.getInstance("SHA-256").digest(bytes)

private fun hex(bytes: ByteArray): String = bytes.joinToString("") { "%02x".format(it) }

/** The content hash of Dropbox of a content of one block. */
private fun hashOf(content: ByteArray): String = hex(sha256(sha256(content)))

class DownloaderTest {
    @get:Rule
    val dir = TemporaryFolder()

    private val cloud = FakeCloud()
    private lateinit var root: File
    private lateinit var downloader: Downloader

    @Before
    fun folders() {
        root = dir.newFolder("markdown")
        val folder = MarkdownFolder(root, File(dir.root, "temp"))
        downloader = Downloader(cloud, folder, File(dir.root, "cursor"))
    }

    /** The files under the Markdown folder, by their path, with their text. */
    private fun here(): Map<String, String> =
        root.walk().filter { it.isFile }.associate { it.relativeTo(root).invariantSeparatorsPath to it.readText() }.toSortedMap()

    private fun run() = downloader.run { _, _ -> }

    // 最初の回は、クラウドのフォルダーのMarkdownをすべて取り、同じパスに置く。
    @Test
    fun the_first_run_downloads_every_markdown_file_to_its_path() {
        cloud.answers[null] = Changes(
            listOf(cloud.file("docs/notes.md", "- a"), cloud.file("journals/2026/2026-11-12.md", "- b")),
            cursor = "c1",
            whole = true,
        )

        run()

        assertEquals(mapOf("docs/notes.md" to "- a", "journals/2026/2026-11-12.md" to "- b"), here())
    }

    // assetsのフォルダーのファイルは、Markdownでなくても取る。ページの画像である。
    @Test
    fun a_file_of_the_assets_folder_is_downloaded() {
        cloud.answers[null] = Changes(
            listOf(cloud.file("Assets/Picture.PNG", "png"), cloud.file("docs/Notes.MD", "- a")),
            cursor = "c1",
            whole = true,
        )

        run()

        assertEquals(mapOf("Assets/Picture.PNG" to "png", "docs/Notes.MD" to "- a"), here())
    }

    // assetsの外にある、Markdownでないファイルは取らない。
    @Test
    fun a_file_that_is_neither_markdown_nor_an_asset_is_not_downloaded() {
        cloud.answers[null] = Changes(
            listOf(cloud.file("docs/picture.png", "png"), cloud.file(".obsidian/app.json", "{}"), cloud.file("assets.png", "png")),
            cursor = "c1",
            whole = true,
        )

        run()

        assertEquals(emptyMap<String, String>(), here())
    }

    // 次の回は、前の回のカーソルから後の変更だけを尋ね、変わったファイルだけを取る。
    @Test
    fun the_next_run_asks_for_the_changes_since_the_last_and_downloads_only_those() {
        cloud.answers[null] = Changes(listOf(cloud.file("docs/a.md", "- a"), cloud.file("docs/b.md", "- b")), "c1", whole = true)
        run()
        cloud.answers["c1"] = Changes(listOf(cloud.file("docs/b.md", "- b2")), "c2", whole = false)
        cloud.downloaded.clear()

        run()

        assertEquals(listOf(null, "c1"), cloud.askedWith)
        assertEquals(listOf("docs/b.md"), cloud.downloaded)
        assertEquals(mapOf("docs/a.md" to "- a", "docs/b.md" to "- b2"), here())
    }

    // クラウドで消えたファイルは、アプリのフォルダーからも消える。
    @Test
    fun a_file_that_has_gone_from_the_cloud_is_deleted_here() {
        cloud.answers[null] = Changes(listOf(cloud.file("docs/a.md", "- a"), cloud.file("docs/b.md", "- b")), "c1", whole = true)
        run()
        cloud.answers["c1"] = Changes(listOf(Change.Gone("docs/a.md")), "c2", whole = false)

        run()

        assertEquals(mapOf("docs/b.md" to "- b"), here())
    }

    // クラウドで消えたフォルダーは、中のファイルごと消える。同じ回でその後に来たファイルは残る。
    @Test
    fun a_folder_that_has_gone_takes_its_files_with_it_but_not_those_that_came_after() {
        cloud.answers[null] = Changes(listOf(cloud.file("docs/work/old.md", "- old")), "c1", whole = true)
        run()
        cloud.answers["c1"] = Changes(
            listOf(cloud.file("docs/work/lost.md", "- lost"), Change.Gone("docs/work"), cloud.file("docs/work/new.md", "- new")),
            "c2",
            whole = false,
        )

        run()

        assertEquals(mapOf("docs/work/new.md" to "- new"), here())
    }

    // 名前の大文字と小文字だけが変わったファイルは、新しい名前だけで残る。
    @Test
    fun a_file_whose_name_changed_only_in_case_stays_under_the_new_name_only() {
        cloud.answers[null] = Changes(listOf(cloud.file("docs/notes.md", "- a")), "c1", whole = true)
        run()
        cloud.answers["c1"] = Changes(listOf(cloud.file("Docs/Notes.md", "- a2")), "c2", whole = false)

        run()

        assertEquals(mapOf("docs/Notes.md" to "- a2"), here())
    }

    // 大文字と小文字が違うパスで消えたと知らされても、そのファイルは消える。
    @Test
    fun a_file_is_deleted_by_a_path_in_another_case() {
        cloud.answers[null] = Changes(listOf(cloud.file("docs/Notes.md", "- a")), "c1", whole = true)
        run()
        cloud.answers["c1"] = Changes(listOf(Change.Gone("docs/notes.md"), Change.Gone("docs/never-here.md")), "c2", whole = false)

        run()

        assertEquals(emptyMap<String, String>(), here())
    }

    // ダウンロードが失敗すると、例外が出て、カーソルは進まない。次の回は同じ変更を尋ね、足りないファイルだけを取る。
    @Test
    fun after_a_failed_download_the_next_run_asks_again_and_downloads_only_what_is_missing() {
        cloud.answers[null] = Changes(listOf(cloud.file("docs/a.md", "- a"), cloud.file("docs/b.md", "- b")), "c1", whole = true)
        cloud.broken = "docs/b.md"
        assertThrows(IOException::class.java) { run() }
        cloud.broken = null
        cloud.downloaded.clear()

        run()

        assertEquals(listOf(null, null), cloud.askedWith)
        assertEquals(listOf("docs/b.md"), cloud.downloaded)
        assertEquals(mapOf("docs/a.md" to "- a", "docs/b.md" to "- b"), here())
    }

    // 途中で失敗したファイルは、アプリのフォルダーに半分だけ残ることがない。
    @Test
    fun a_download_that_failed_leaves_no_file() {
        cloud.answers[null] = Changes(listOf(cloud.file("docs/a.md", "- a")), "c1", whole = true)
        cloud.broken = "docs/a.md"

        assertThrows(IOException::class.java) { run() }

        assertEquals(emptyMap<String, String>(), here())
    }

    // クラウドがカーソルを忘れて全部を答えたときは、そこにないMarkdownがアプリのフォルダーから消える。
    @Test
    fun a_whole_list_deletes_the_markdown_files_that_are_not_in_it() {
        cloud.answers[null] = Changes(listOf(cloud.file("docs/a.md", "- a"), cloud.file("docs/b.md", "- b")), "c1", whole = true)
        run()
        cloud.answers["c1"] = Changes(listOf(cloud.file("docs/B.md", "- b")), "c2", whole = true)
        cloud.downloaded.clear()

        run()

        assertEquals(mapOf("docs/b.md" to "- b"), here())
        assertEquals(emptyList<String>(), cloud.downloaded)
    }

    // 進み具合は、最初に0個、そのあと1個取るごとに、全体の数と一緒に伝わる。取らないファイルは数えない。
    @Test
    fun the_progress_counts_the_files_to_download_from_zero_to_all() {
        cloud.answers[null] = Changes(listOf(cloud.file("docs/a.md", "- a")), "c1", whole = true)
        run()
        cloud.answers["c1"] = Changes(
            listOf(cloud.file("docs/a.md", "- a"), cloud.file("docs/b.md", "- b"), cloud.file("docs/c.md", "- c")),
            "c2",
            whole = false,
        )
        val told = mutableListOf<Pair<Int, Int>>()

        downloader.run { done, total -> told += done to total }

        assertEquals(listOf(0 to 2, 1 to 2, 2 to 2), told)
    }

    // 4MBを超えるファイルのハッシュは、4MBごとのハッシュをつないだもののハッシュである。
    @Test
    fun the_content_hash_of_a_large_file_is_made_of_the_hashes_of_its_blocks() {
        val block = 4 * 1024 * 1024
        val content = ByteArray(block + 3) { (it % 251).toByte() }
        val file = dir.newFile("large").apply { writeBytes(content) }
        val first = sha256(content.copyOfRange(0, block))
        val second = sha256(content.copyOfRange(block, content.size))

        assertEquals(hex(sha256(first + second)), dropboxContentHash(file))
    }
}
