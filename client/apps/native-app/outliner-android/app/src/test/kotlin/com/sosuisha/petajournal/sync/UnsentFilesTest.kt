package com.sosuisha.petajournal.sync

import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * What the download does with the files that the cloud lost. A file that changed in the app and is
 * not uploaded yet stays. An app without a record has no such file.
 */
class UnsentFilesTest : SyncFilesFixture() {
    // クラウドで消えたファイルでも、アプリで変えてまだ上げていないものは消えない。新しいファイルとして上がる。
    @Test
    fun a_file_that_changed_here_stays_when_the_cloud_lost_it_and_is_uploaded_as_a_new_file() {
        download("docs/notes.md" to "- a", "docs/other.md" to "- b")
        write("docs/notes.md", "- from the phone")

        download("c1", "c2", false, cloud.gone("docs/Notes.md"), cloud.gone("docs/other.md"))
        upload()

        assertEquals(listOf("docs/notes.md"), here())
        assertEquals("- from the phone", cloud.text("docs/notes.md"))
    }

    // クラウドでフォルダーごと消えたときも、その中の、アプリで変えたファイルと作ったファイルは残る。
    @Test
    fun the_files_that_changed_here_stay_when_the_cloud_lost_their_folder() {
        download("docs/work/plan.md" to "- a", "docs/work/old.md" to "- b", "docs/work/deep/old.md" to "- c")
        write("docs/work/plan.md", "- from the phone")
        write("docs/work/new.md", "- made on the phone")

        download("c1", "c2", false, cloud.gone("docs/work"))

        assertEquals(listOf("docs/work/new.md", "docs/work/plan.md"), here())
        assertEquals(false, File(root, "docs/work/deep").exists())
    }

    // クラウドが全部を答えたときも、そこにないファイルのうち、アプリで変えたものと作ったものは残る。
    @Test
    fun the_files_that_changed_here_stay_when_a_whole_list_lacks_them() {
        download("docs/notes.md" to "- a", "docs/old.md" to "- b", "docs/other.md" to "- c")
        write("docs/notes.md", "- from the phone")
        write("docs/new.md", "- made on the phone")
        cloud.gone("docs/notes.md")
        cloud.gone("docs/old.md")

        download("c1", "c2", true, cloud.file("docs/other.md", "- c"))
        upload()

        assertEquals(listOf("docs/new.md", "docs/notes.md", "docs/other.md"), here())
        assertEquals(listOf("docs/new.md", "docs/notes.md"), cloud.uploaded)
    }

    // 記録のないアプリ（アップロードを知らない版が取り込んだもの）は、カーソルがあっても、クラウドの全部を尋ねる。
    @Test
    fun an_app_without_a_record_asks_for_everything_though_it_has_a_cursor() {
        download("docs/notes.md" to "- a")
        File(dir.root, "record").delete()
        open()
        cloud.downloaded.clear()

        downloader.run { _, _ -> }
        upload()

        assertEquals(listOf(null, null), cloud.askedWith)
        assertEquals(emptyList<String>(), cloud.downloaded)
        assertEquals(emptyList<String>(), cloud.uploaded)
    }

    // 記録のないアプリ（アップロードを知らない版が取り込んだもの）のファイルは、すべてクラウドの写しである。クラウドにないものは消える。
    @Test
    fun an_app_without_a_record_deletes_every_file_that_the_cloud_lacks() {
        download("docs/notes.md" to "- a", "docs/other.md" to "- b")
        File(dir.root, "record").delete()
        open()
        cloud.gone("docs/notes.md")
        cloud.answers[null] = Changes(listOf(cloud.file("docs/other.md", "- b")), "c2", whole = true)

        downloader.run { _, _ -> }
        upload()

        assertEquals(listOf("docs/other.md"), here())
        assertEquals(emptyList<String>(), cloud.uploaded)
    }
}
