package com.sosuisha.petajournal.sync

import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.File

class SyncRoundTest {
    @get:Rule
    val dir = TemporaryFolder()

    private val cloud = FakeCloud()
    private lateinit var root: File

    /** What the core does with the files in a round, in place of the core. */
    private var takeIn: () -> Unit = {}
    private lateinit var round: SyncRound

    @Before
    fun folders() {
        root = dir.newFolder("markdown")
        val folder = MarkdownFolder(root, File(dir.root, "temp"))
        val record = CloudRecord(File(dir.root, "record"))
        val downloader = Downloader(cloud, folder, File(dir.root, "cursor"), record)
        round = SyncRound(downloader, { takeIn() }, Uploader(cloud, folder, record))
        cloud.answers[null] = Changes(listOf(cloud.file(NOTES, "- a")), "c1", whole = true)
    }

    private fun here(): String = File(root, NOTES).readText()

    private fun write(text: String) = File(root, NOTES).writeText(text)

    /** The desk changes the file in the cloud, and the cloud tells of it after [since]. */
    private fun deskWrites(text: String, since: String, next: String) {
        cloud.answers[since] = Changes(listOf(cloud.file(NOTES, text)), next, whole = false)
    }

    private fun run() = round.run({ _, _ -> }, { _, _ -> })

    // 1回の同期は、ダウンロード、コアの取り込みと書き出し、アップロードの順に進む。
    @Test
    fun a_round_downloads_then_lets_the_core_work_then_uploads() {
        takeIn = { write(here() + " and what the phone wrote") }

        run()

        assertEquals("- a and what the phone wrote", cloud.text(NOTES))
    }

    // アップロードの前にクラウドのファイルが変わったら、ダウンロードからやり直す。クラウドの変更は消えない。
    @Test
    fun a_round_starts_again_when_the_cloud_changed_before_the_upload() {
        var rounds = 0
        takeIn = {
            if (++rounds == 1) deskWrites("- desk", since = "c1", next = "c2")
            write(here() + " + phone")
        }

        run()

        assertEquals(listOf(null, "c1"), cloud.askedWith)
        assertEquals("- desk + phone", cloud.text(NOTES))
    }

    // クラウドが3回続けて断ったら、やめて例外を出す。クラウドのファイルは上書きしない。
    @Test
    fun a_round_gives_up_after_the_cloud_refused_three_times() {
        var rounds = 0
        takeIn = {
            rounds++
            deskWrites("- desk $rounds", since = "c$rounds", next = "c${rounds + 1}")
            write("- phone")
        }

        assertThrows(ChangedInCloud::class.java) { run() }

        assertEquals(3, rounds)
        assertEquals("- desk 3", cloud.text(NOTES))
    }

    // 進み具合は、ダウンロードとアップロードで別々に伝わる。
    @Test
    fun the_progress_of_the_download_and_of_the_upload_are_told_apart() {
        takeIn = { write("- phone") }
        val told = mutableListOf<String>()

        round.run({ done, total -> told += "down $done/$total" }, { done, total -> told += "up $done/$total" })

        assertEquals(listOf("down 0/1", "down 1/1", "up 0/1", "up 1/1"), told)
    }

    private companion object {
        const val NOTES = "docs/notes.md"
    }
}
