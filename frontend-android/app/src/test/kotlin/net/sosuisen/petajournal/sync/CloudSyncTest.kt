package net.sosuisen.petajournal.sync

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.File
import java.util.concurrent.CountDownLatch
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

class CloudSyncTest {
    @get:Rule
    val dir = TemporaryFolder()

    private val cloud = FakeCloud()

    /** What waits for the main thread. The test is the main thread, and runs it in [rest]. */
    private val posted = LinkedBlockingQueue<() -> Unit>()

    /** Where the sync stood after each piece of work of the main thread. */
    private val seen = mutableListOf<SyncState>()

    /** How often the core was asked to work, and what holds it back in its first work. */
    private var rounds = 0
    private val held = CountDownLatch(1)
    private val working = CountDownLatch(1)

    /** What the core does with the files in a round, in place of the core. */
    private var work: () -> Unit = {}
    private lateinit var root: File
    private lateinit var sync: CloudSync

    @Before
    fun parts() {
        root = dir.newFolder("markdown")
        val folder = MarkdownFolder(root, File(dir.root, "temp"))
        val record = CloudRecord(File(dir.root, "record"))
        val downloader = Downloader(cloud, folder, File(dir.root, "cursor"), record)
        val takeIn = {
            rounds++
            working.countDown()
            held.await()
            work()
        }
        sync = CloudSync(signedIn = true, { SyncRound(downloader, takeIn, Uploader(it, folder, record)) }, posted::put)
        cloud.answers[null] = Changes(emptyList(), "c1", whole = true)
        cloud.answers["c1"] = Changes(emptyList(), "c1", whole = false)
    }

    /** Does the work of the main thread until nothing more comes. */
    private fun rest() {
        while (true) {
            val next = posted.poll(2, TimeUnit.SECONDS) ?: break
            next()
            seen += sync.state
        }
    }

    /** Does the work of the main thread until the sync rests. */
    private fun settle() {
        rest()
        assertEquals(SyncState.Idle, sync.state)
    }

    // 同期の途中で頼まれた同期は、捨てられない。今の同期が終わったあとに、1回だけ始まる。
    @Test
    fun a_sync_that_is_asked_for_while_one_runs_starts_once_after_it() {
        sync.start(cloud)
        assertTrue(working.await(2, TimeUnit.SECONDS))

        sync.start(cloud)
        sync.start(cloud)
        held.countDown()
        settle()

        assertEquals(2, rounds)
    }

    // アプリが前に出たときの同期は、終わるまで「前の同期」である。画面は、その間ダイアログを出す。
    @Test
    fun a_sync_of_the_app_that_came_to_the_front_is_in_front_until_it_rests() {
        sync.start(cloud, inFront = true)
        assertTrue(working.await(2, TimeUnit.SECONDS))
        assertTrue(sync.inFront)

        held.countDown()
        settle()

        assertFalse(sync.inFront)
    }

    // 編集のあとの同期は、前の同期ではない。その途中でアプリが前に出ると、前の同期になる。
    @Test
    fun a_quiet_sync_comes_to_the_front_when_the_app_does() {
        sync.start(cloud)
        assertTrue(working.await(2, TimeUnit.SECONDS))
        assertFalse(sync.inFront)

        sync.start(cloud, inFront = true)

        assertTrue(sync.inFront)
        held.countDown()
        settle()
        assertFalse(sync.inFront)
    }

    // 途中で頼まれなければ、同期は1回で終わる。
    @Test
    fun a_sync_runs_once_when_no_other_is_asked_for() {
        sync.start(cloud)
        held.countDown()
        settle()

        assertEquals(1, rounds)
    }

    // 同期は、ダウンロードとアップロードがどこまで進んだかを、画面に伝える。
    @Test
    fun a_sync_tells_how_far_the_download_and_the_upload_are() {
        cloud.answers[null] = Changes(listOf(cloud.file("docs/a.md", "- a")), "c1", whole = true)
        work = { File(root, "docs/new.md").writeText("- new") }
        held.countDown()

        sync.start(cloud)
        assertEquals(SyncState.Downloading(0, 0), sync.state)
        settle()

        assertEquals(
            listOf(
                SyncState.Downloading(0, 1),
                SyncState.Downloading(1, 1),
                SyncState.Uploading(0, 1),
                SyncState.Uploading(1, 1),
                SyncState.Idle,
            ),
            seen,
        )
    }

    // 上げるファイルがない同期は、アップロードの状態にならない。
    @Test
    fun a_sync_with_nothing_to_upload_tells_of_no_upload() {
        held.countDown()

        sync.start(cloud)
        settle()

        assertEquals(emptyList<SyncState>(), seen.filterIsInstance<SyncState.Uploading>())
    }

    // 同期が失敗すると、その理由を持って止まる。前の同期でもなくなる。
    @Test
    fun a_sync_that_fails_rests_with_the_reason() {
        cloud.answers[null] = Changes(listOf(cloud.file("docs/a.md", "- a")), "c1", whole = true)
        cloud.broken = "docs/a.md"

        sync.start(cloud, inFront = true)
        rest()

        assertEquals(SyncState.Failed("the network is gone"), sync.state)
        assertFalse(sync.inFront)
    }

    // 失敗したあとも、同期はまた始められる。
    @Test
    fun a_sync_starts_again_after_one_failed() {
        cloud.answers[null] = Changes(listOf(cloud.file("docs/a.md", "- a")), "c1", whole = true)
        cloud.broken = "docs/a.md"
        sync.start(cloud)
        rest()
        cloud.broken = null
        held.countDown()

        sync.start(cloud)
        settle()

        assertEquals("- a", File(root, "docs/a.md").readText())
    }

    // サインアウトすると、サインインしていない状態になる。
    @Test
    fun a_sign_out_rests_signed_out() {
        sync.signOut()

        assertEquals(SyncState.SignedOut, sync.state)
    }
}
