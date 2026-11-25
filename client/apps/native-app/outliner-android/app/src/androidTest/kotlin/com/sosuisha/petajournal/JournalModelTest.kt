package com.sosuisha.petajournal

import android.app.Application
import android.os.SystemClock
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.sosuisha.petajournal.sync.SyncState
import com.sosuisha.petajournal.types.RichLink
import com.sosuisha.petajournal.types.ScreenView
import java.io.File
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * When the app syncs, and what the sign-in and the sign-out do to it. The model has a real core
 * under it, on the folders of a test. The cloud is the account of a test.
 */
@RunWith(AndroidJUnit4::class)
class JournalModelTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private val app = instrumentation.targetContext.applicationContext as Application
    private val root = File(app.cacheDir, "model-test")
    private var opened: JournalModel? = null

    @Before
    fun emptyFolders() {
        root.deleteRecursively()
    }

    /** The next test empties the folders: the core must have let go of them first (#1132). */
    @After
    fun close() {
        val model = opened ?: return
        onMain { model.close() }
        check(model.awaitClosed(WAIT_MS)) { "the core did not close in $WAIT_MS ms" }
    }

    private fun onMain(work: () -> Unit) = instrumentation.runOnMainSync(work)

    /** Opens the app on the folders of the test. An edit syncs after [afterEditMs]. */
    private fun open(account: FakeAccount, afterEditMs: Long = NEVER_MS): JournalModel {
        lateinit var model: JournalModel
        onMain {
            model =
                JournalModel(app, File(root, "files"), File(root, "cache"), account, afterEditMs)
        }
        opened = model
        return model
    }

    private fun waitUntil(what: () -> Boolean) {
        val end = SystemClock.uptimeMillis() + WAIT_MS
        while (!what()) {
            check(SystemClock.uptimeMillis() < end) { "waited $WAIT_MS ms in vain" }
            Thread.sleep(20)
        }
    }

    // 閉じたアプリのコアが閉じ終わるのを待てば、同じフォルダーでアプリを開き直せる。閉じ終わる前は、索引のロックが取れずに開けないことがある（#1132）。
    @Test
    fun the_app_opens_again_on_the_same_folders_once_its_core_has_closed() {
        val first = open(FakeAccount())
        onMain { first.close() }

        assertTrue(first.awaitClosed(WAIT_MS))
        val again = open(FakeAccount())

        assertTrue(again.shell.view.screen is ScreenView.Journal)
    }

    // クラウドの用意がないアプリは、何も同期しない。今日のジャーナルが開く。
    @Test
    fun an_app_without_a_cloud_syncs_nothing_and_opens_at_today() {
        val account = FakeAccount(isSetUp = false)

        val model = open(account)
        onMain {
            model.syncInFront()
            model.sync()
            model.signOut()
        }

        assertNull(model.cloud)
        assertEquals(0, account.syncs)
        assertTrue(model.shell.view.screen is ScreenView.Journal)
    }

    // サインインの前は、今日のジャーナルを作らない。最初のダウンロードが、その日のジャーナルを持ってくるかもしれない。
    @Test
    fun a_new_app_makes_no_journal_before_the_sign_in() {
        val model = open(FakeAccount(signedIn = false))

        assertTrue(model.shell.unopened)
        assertEquals(SyncState.SignedOut, model.cloud?.state)
        assertEquals(
            emptyList<String>(),
            File(root, "files/markdown/journals").list().orEmpty().toList(),
        )
    }

    // サインインしているアプリは、今日のジャーナルを開く。
    @Test
    fun an_app_that_is_signed_in_opens_at_today() {
        val model = open(FakeAccount())

        assertFalse(model.shell.unopened)
        assertTrue(model.shell.view.screen is ScreenView.Journal)
    }

    // サインインから戻って前に出たアプリは、新しいサインインを取っておき、ユーザーが待つ同期を始める。
    @Test
    fun the_app_that_comes_to_the_front_keeps_a_new_sign_in_and_syncs_in_front() {
        val account = FakeAccount(signedIn = false).apply { fresh = true }
        val model = open(account)
        var inFront = false

        onMain {
            model.syncInFront()
            inFront = model.cloud?.inFront == true
        }

        assertTrue(account.isSignedIn)
        assertTrue(inFront)
        waitUntil {
            File(root, "files/markdown/docs/trip.md").exists() &&
                model.cloud?.state == SyncState.Idle
        }
    }

    // 始めた同期は、コアの知らせを待たずに、画面で動いていると分かる。その間、画面はサインアウトを出さない。コアは、同期の途中のサインアウトを受け付けない。
    @Test
    fun a_sync_that_started_runs_on_the_screen_before_the_core_tells_of_it() {
        val model = open(FakeAccount())
        var state: SyncState? = null

        onMain {
            model.sync()
            state = model.cloud?.state
        }

        assertTrue(state is SyncState.Downloading)
        waitUntil { model.cloud?.state == SyncState.Idle }
    }

    // サインインの前に前に出たアプリは、何も同期しない。
    @Test
    fun the_app_that_comes_to_the_front_before_the_sign_in_syncs_nothing() {
        val account = FakeAccount(signedIn = false)
        val model = open(account)

        onMain { model.syncInFront() }

        assertEquals(0, account.syncs)
        assertEquals(SyncState.SignedOut, model.cloud?.state)
    }

    // アプリでページを作ると、少し待ってから同期し、そのページがDropboxに上がる。いつ同期するかは、コアが決める（ADR 055）。
    @Test
    fun a_page_made_in_the_app_is_uploaded_a_moment_later() {
        val account = FakeAccount()
        val model = open(account, afterEditMs = 300)
        onMain { model.syncInFront() }
        waitUntil { model.cloud?.state == SyncState.Idle }

        onMain { model.shell.follow(RichLink.Page("/pages/brand-new")) }

        waitUntil { "docs/brand-new.md" in account.page.uploaded }
        assertEquals(1, account.syncs)
    }

    // サインアウトは、ダウンロードがどこまで進んだかを忘れる。次のサインインは、別のアカウントかもしれない。そのあとは同期しない。
    @Test
    fun the_sign_out_forgets_where_the_download_ended_and_the_app_syncs_no_more() {
        val account = FakeAccount()
        val model = open(account)
        val cursor = File(root, "files/dropbox-cursor")
        onMain { model.sync() }
        waitUntil { cursor.exists() && model.cloud?.state == SyncState.Idle }

        onMain {
            model.signOut()
            model.sync()
        }

        assertFalse(cursor.exists())
        assertFalse(account.isSignedIn)
        assertEquals(SyncState.SignedOut, model.cloud?.state)
        assertEquals(1, account.syncs)
        assertTrue(File(root, "files/markdown/docs/trip.md").exists())
    }

    private companion object {
        const val WAIT_MS = 10_000L

        /** Longer than a test runs: no edit syncs. */
        const val NEVER_MS = 600_000L
    }
}
