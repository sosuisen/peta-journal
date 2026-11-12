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

    /** What waits for the main thread. The test is the main thread, and runs it in [settle]. */
    private val posted = LinkedBlockingQueue<() -> Unit>()

    /** How often the core was asked to work, and what holds it back in its first work. */
    private var rounds = 0
    private val held = CountDownLatch(1)
    private val working = CountDownLatch(1)
    private lateinit var sync: CloudSync

    @Before
    fun parts() {
        val folder = MarkdownFolder(dir.newFolder("markdown"), File(dir.root, "temp"))
        val record = CloudRecord(File(dir.root, "record"))
        val downloader = Downloader(cloud, folder, File(dir.root, "cursor"), record)
        val takeIn = {
            rounds++
            working.countDown()
            held.await()
        }
        sync = CloudSync(signedIn = true, { SyncRound(downloader, takeIn, Uploader(it, folder, record)) }, posted::put)
        cloud.answers[null] = Changes(emptyList(), "c1", whole = true)
        cloud.answers["c1"] = Changes(emptyList(), "c1", whole = false)
    }

    /** Does the work of the main thread until the sync rests. */
    private fun settle() {
        while (true) {
            val next = posted.poll(2, TimeUnit.SECONDS) ?: break
            next()
        }
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
}
