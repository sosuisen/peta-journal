package net.sosuisen.petajournal.dropbox

import net.sosuisen.petajournal.sync.Change
import net.sosuisen.petajournal.sync.ChangedInCloud
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.IOException
import java.io.OutputStream
import java.util.Date

/** What the app makes of the answers of Dropbox. Dropbox itself is a Dropbox of the test. */
class DropboxCloudTest {
    @get:Rule
    val dir = TemporaryFolder()

    private val dropbox = FakeDropboxFiles()
    private val cloud = DropboxCloud(dropbox, "/PetaJournal/")

    /** A Dropbox that answers what the test tells it, and keeps what it was asked. */
    private class FakeDropboxFiles : DropboxFiles {
        /** The whole list of each folder, and the parts after each cursor. */
        val lists = mutableMapOf<String, Listing>()
        val after = mutableMapOf<String, Listing>()

        /** The cursors that Dropbox has forgotten. */
        val forgotten = mutableSetOf<String>()
        val contents = mutableMapOf<String, String>()

        /** What stops the next upload, if anything. */
        var refusal: IOException? = null
        val uploads = mutableListOf<Upload>()

        override fun list(folder: String): Listing = lists.getValue(folder)

        override fun listAfter(cursor: String): Listing {
            if (cursor in forgotten) throw CursorForgotten()
            return after.getValue(cursor)
        }

        override fun download(rev: String, into: OutputStream) = into.write(contents.getValue(rev).toByteArray())

        override fun upload(path: String, from: File, over: String?, modified: Date): Entry.File {
            refusal?.let { throw it }
            uploads += Upload(path, from.readText(), over, modified)
            return Entry.File(path, "hash-of-upload", "r9")
        }
    }

    private data class Upload(val path: String, val text: String, val over: String?, val modified: Date)

    private fun file(path: String, rev: String = "r1") = Entry.File(path, "hash-$rev", rev)

    private fun page(text: String = "- a"): File = dir.newFile("a.md").apply { writeText(text) }

    // カーソルがなければ、フォルダーの全部の一覧を取る。パスは、フォルダーからの相対になる。
    @Test
    fun without_a_cursor_the_changes_are_the_whole_folder_with_paths_inside_it() {
        dropbox.lists["/PetaJournal"] = Listing(listOf(file("/PetaJournal/docs/a.md")), "c1", hasMore = false)

        val changes = cloud.changes(null)

        assertTrue(changes.whole)
        assertEquals("c1", changes.cursor)
        assertEquals(listOf<Change>(Change.File("docs/a.md", "hash-r1", "rev:r1")), changes.entries)
    }

    // カーソルがあれば、そのあとに変わったものだけを取る。
    @Test
    fun with_a_cursor_the_changes_are_what_came_after_it() {
        dropbox.after["c1"] = Listing(listOf(file("/PetaJournal/docs/b.md", "r2")), "c2", hasMore = false)

        val changes = cloud.changes("c1")

        assertFalse(changes.whole)
        assertEquals("c2", changes.cursor)
        assertEquals(listOf("docs/b.md"), changes.entries.map { it.path })
    }

    // Dropboxがカーソルを忘れていれば、全部の一覧を取り直す。
    @Test
    fun a_cursor_that_dropbox_forgot_gives_the_whole_folder_again() {
        dropbox.forgotten += "old"
        dropbox.lists["/PetaJournal"] = Listing(listOf(file("/PetaJournal/docs/a.md")), "c1", hasMore = false)

        val changes = cloud.changes("old")

        assertTrue(changes.whole)
        assertEquals(listOf("docs/a.md"), changes.entries.map { it.path })
    }

    // Dropboxは一覧を何回かに分けて答える。全部をつなぎ、最後のカーソルを返す。
    @Test
    fun a_list_that_comes_in_parts_is_read_to_its_end() {
        dropbox.lists["/PetaJournal"] = Listing(listOf(file("/PetaJournal/docs/a.md")), "part", hasMore = true)
        dropbox.after["part"] = Listing(listOf(file("/PetaJournal/docs/b.md")), "end", hasMore = false)

        val changes = cloud.changes(null)

        assertEquals(listOf("docs/a.md", "docs/b.md"), changes.entries.map { it.path })
        assertEquals("end", changes.cursor)
        assertTrue(changes.whole)
    }

    // パスは、Dropboxが見せるとおりの大文字と小文字を保つ。ページの名前になるからである。
    @Test
    fun a_path_keeps_the_case_that_dropbox_shows() {
        dropbox.lists["/PetaJournal"] = Listing(listOf(file("/petajournal/Docs/Trip.md")), "c1", hasMore = false)

        assertEquals(listOf("Docs/Trip.md"), cloud.changes(null).entries.map { it.path })
    }

    // あるフォルダーの知らせは、変更にならない。フォルダーは、その中のファイルで分かる。
    @Test
    fun a_folder_that_is_there_is_no_change() {
        val entries = listOf(Entry.Folder("/PetaJournal"), Entry.Folder("/PetaJournal/docs"), file("/PetaJournal/docs/a.md"))
        dropbox.lists["/PetaJournal"] = Listing(entries, "c1", hasMore = false)

        assertEquals(listOf("docs/a.md"), cloud.changes(null).entries.map { it.path })
    }

    // 消えたファイルやフォルダーは、「なくなった」という変更になる。
    @Test
    fun what_was_deleted_in_dropbox_is_a_change_that_says_gone() {
        dropbox.after["c1"] = Listing(listOf(Entry.Deleted("/PetaJournal/docs")), "c2", hasMore = false)

        assertEquals(listOf<Change>(Change.Gone("docs")), cloud.changes("c1").entries)
    }

    // パスやハッシュのない知らせでも、落ちない。パスがなければ捨て、ハッシュがなければ空にする。
    @Test
    fun an_entry_without_a_path_is_left_out_and_one_without_a_hash_gets_an_empty_one() {
        val entries = listOf(Entry.File(null, "h", "r1"), Entry.File("/PetaJournal/docs/a.md", null, "r2"))
        dropbox.lists["/PetaJournal"] = Listing(entries, "c1", hasMore = false)

        assertEquals(listOf<Change>(Change.File("docs/a.md", "", "rev:r2")), cloud.changes(null).entries)
    }

    // ダウンロードは、一覧で見たそのバージョンを取る。
    @Test
    fun a_download_takes_the_very_version_that_the_list_showed() {
        dropbox.lists["/PetaJournal"] = Listing(listOf(file("/PetaJournal/docs/a.md", "r7")), "c1", hasMore = false)
        dropbox.contents["r7"] = "- seventh"
        val seen = cloud.changes(null).entries.single() as Change.File
        val into = ByteArrayOutputStream()

        cloud.download(seen, into)

        assertEquals("- seventh", into.toString())
    }

    // Dropboxにないファイルは、置き換えるバージョンなしで上げる。フォルダーのパスを前に付ける。
    @Test
    fun a_new_file_is_uploaded_over_no_version_into_the_folder() {
        val answer = cloud.upload("docs/a.md", page("- new"), over = null)

        assertEquals(listOf("/PetaJournal/docs/a.md" to null), dropbox.uploads.map { it.path to it.over })
        assertEquals("- new", dropbox.uploads.single().text)
        assertEquals(Change.File("docs/a.md", "hash-of-upload", "rev:r9"), answer)
    }

    // Dropboxにあるファイルは、アプリが見たバージョンの上にだけ上げる。
    @Test
    fun a_file_that_dropbox_has_is_uploaded_over_the_version_that_the_app_saw() {
        cloud.upload("docs/a.md", page(), over = "rev:r3")

        assertEquals("r3", dropbox.uploads.single().over)
    }

    // 上げるファイルの更新時刻は、秒までにする。Dropboxは、秒より細かい時刻を受け取らない。
    @Test
    fun an_upload_tells_the_time_of_the_last_change_in_whole_seconds() {
        val page = page().apply { setLastModified(1_700_000_000_999) }

        cloud.upload("docs/a.md", page, over = null)

        assertEquals(Date(1_700_000_000_000), dropbox.uploads.single().modified)
    }

    // Dropboxのファイルが別のバージョンなら、「クラウドで変わった」というエラーになる。
    @Test
    fun an_upload_over_another_version_fails_as_changed_in_the_cloud() {
        dropbox.refusal = UploadConflict()

        val changed = assertThrows(ChangedInCloud::class.java) { cloud.upload("docs/a.md", page(), over = "rev:r3") }

        assertEquals("docs/a.md", changed.path)
    }

    // ほかの失敗は、そのまま伝わる。同期は、その理由を持って止まる。
    @Test
    fun an_upload_that_fails_for_another_reason_fails_with_that_reason() {
        val gone = IOException("the network is gone")
        dropbox.refusal = gone

        val failure = assertThrows(IOException::class.java) { cloud.upload("docs/a.md", page(), over = null) }

        assertEquals(gone, failure)
    }

    // ファイルのハッシュは、Dropboxのコンテンツハッシュである。
    @Test
    fun the_content_hash_of_a_file_is_the_one_of_dropbox() {
        val page = page("- a")

        assertEquals(dropboxContentHash(page), cloud.contentHash(page))
    }
}
