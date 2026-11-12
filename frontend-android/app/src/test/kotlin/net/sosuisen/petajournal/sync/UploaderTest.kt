package net.sosuisen.petajournal.sync

import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.File
import java.io.IOException

class UploaderTest {
    @get:Rule
    val dir = TemporaryFolder()

    private val cloud = FakeCloud()
    private lateinit var root: File
    private lateinit var downloader: Downloader
    private lateinit var uploader: Uploader

    @Before
    fun folders() {
        root = dir.newFolder("markdown")
        val folder = MarkdownFolder(root, File(dir.root, "temp"))
        val record = CloudRecord(File(dir.root, "record"))
        downloader = Downloader(cloud, folder, File(dir.root, "cursor"), record)
        uploader = Uploader(cloud, folder, record)
    }

    /** Downloads the files, as the first sync of the app does. */
    private fun download(vararg files: Pair<String, String>) {
        cloud.answers[null] = Changes(files.map { (path, text) -> cloud.file(path, text) }, "c1", whole = true)
        downloader.run { _, _ -> }
    }

    /** Writes the file of a page, as the core does after an edit in the app. */
    private fun write(path: String, text: String) {
        File(root, path).apply { parentFile.mkdirs() }.writeText(text)
    }

    private fun upload() = uploader.run { _, _ -> }

    // アプリで変わったページのファイルは、クラウドへ上がる。
    @Test
    fun the_file_of_a_page_that_changed_in_the_app_is_uploaded() {
        download("docs/notes.md" to "- a", "docs/other.md" to "- b")
        write("docs/notes.md", "- a2")

        upload()

        assertEquals(listOf("docs/notes.md"), cloud.uploaded)
        assertEquals("- a2", cloud.text("docs/notes.md"))
    }

    // アプリで作ったページのファイルは、新しいファイルとして上がる。
    @Test
    fun the_file_of_a_page_that_was_made_in_the_app_is_uploaded_as_a_new_file() {
        download("docs/notes.md" to "- a")
        write("journals/2026-11-12.md", "- today")

        upload()

        assertEquals("- today", cloud.text("journals/2026-11-12.md"))
    }

    // 上げたファイルは、変わらないかぎり、次の回に上がらない。
    @Test
    fun an_uploaded_file_is_not_uploaded_again_while_it_stays_the_same() {
        download("docs/notes.md" to "- a")
        write("docs/notes.md", "- a2")
        write("docs/new.md", "- new")
        upload()
        cloud.uploaded.clear()

        upload()
        write("docs/notes.md", "- a3")
        upload()

        assertEquals(listOf("docs/notes.md"), cloud.uploaded)
        assertEquals("- a3", cloud.text("docs/notes.md"))
    }

    // Markdownでないファイルは、変わっていても上がらない。
    @Test
    fun a_file_that_is_not_markdown_is_not_uploaded() {
        download("assets/picture.png" to "png")
        write("assets/picture.png", "other")
        write("assets/new.png", "new")

        upload()

        assertEquals(emptyList<String>(), cloud.uploaded)
    }

    // クラウドのファイルが、アプリの知らない版になっていたら、上書きせずに例外が出る。
    @Test
    fun a_file_that_changed_in_the_cloud_meanwhile_is_not_written_over() {
        download("docs/notes.md" to "- a")
        write("docs/notes.md", "- from the phone")
        cloud.file("docs/notes.md", "- from the desk")

        assertThrows(ChangedInCloud::class.java) { upload() }

        assertEquals("- from the desk", cloud.text("docs/notes.md"))
    }

    // アプリで作ったページと同じ名前のファイルがクラウドにできていたら、上書きせずに例外が出る。
    @Test
    fun a_new_file_does_not_write_over_a_file_that_appeared_in_the_cloud() {
        download("docs/notes.md" to "- a")
        write("docs/new.md", "- from the phone")
        cloud.file("docs/New.md", "- from the desk")

        assertThrows(ChangedInCloud::class.java) { upload() }

        assertEquals("- from the desk", cloud.text("docs/new.md"))
    }

    // 途中で失敗しても、それまでに上げたファイルは、次の回に上がらない。
    @Test
    fun the_files_uploaded_before_a_failure_are_not_uploaded_again() {
        download("docs/a.md" to "- a", "docs/b.md" to "- b")
        write("docs/a.md", "- a2")
        write("docs/b.md", "- b2")
        cloud.broken = "docs/b.md"
        assertThrows(IOException::class.java) { upload() }
        cloud.broken = null
        cloud.uploaded.clear()

        upload()

        assertEquals(listOf("docs/b.md"), cloud.uploaded)
    }

    // 進み具合は、最初に0個、そのあと1個上げるごとに、全体の数と一緒に伝わる。
    @Test
    fun the_progress_counts_the_files_to_upload_from_zero_to_all() {
        download("docs/a.md" to "- a", "docs/b.md" to "- b", "docs/c.md" to "- c")
        write("docs/a.md", "- a2")
        write("docs/b.md", "- b2")
        val told = mutableListOf<Pair<Int, Int>>()

        uploader.run { done, total -> told += done to total }

        assertEquals(listOf(0 to 2, 1 to 2, 2 to 2), told)
    }
}
