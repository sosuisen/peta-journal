package com.sosuisha.petajournal

import android.app.Application
import androidx.compose.ui.test.junit4.createEmptyComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onFirst
import androidx.compose.ui.test.performClick
import androidx.lifecycle.Lifecycle
import androidx.test.core.app.ActivityScenario
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * When the activity asks the model to sync. The activity is the real one. Its model is on the
 * folders of a test, with the account of a test: the test touches neither the pages nor the sign-in
 * of the app.
 */
@RunWith(AndroidJUnit4::class)
class MainActivityTest {
    @get:Rule val compose = createEmptyComposeRule()

    private val app =
        InstrumentationRegistry.getInstrumentation().targetContext.applicationContext as Application
    private val root = File(app.cacheDir, "activity-test")
    private var scenario: ActivityScenario<MainActivity>? = null

    @Before
    fun emptyFolders() {
        root.deleteRecursively()
    }

    @After
    fun close() {
        scenario?.close()
        JournalModel.make = ::JournalModel
    }

    /** Opens the app with the account, and brings it to the front. */
    private fun launch(account: FakeAccount): ActivityScenario<MainActivity> {
        JournalModel.make = {
            JournalModel(it, File(root, "files"), File(root, "cache"), account, NEVER_MS)
        }
        return ActivityScenario.launch(MainActivity::class.java).also { scenario = it }
    }

    // アプリは、前に出るたびに、新しいサインインを探して同期する。
    @Test
    fun the_app_looks_for_a_new_sign_in_and_syncs_when_it_comes_to_the_front() {
        val account = FakeAccount()

        launch(account)

        assertEquals(1, account.looks)
        assertEquals(1, account.syncs)
    }

    // アプリは、後ろへ回るときに同期する。書いたものを、Androidがアプリを止める前に上げる。
    @Test
    fun the_app_syncs_when_it_goes_to_the_back() {
        val account = FakeAccount()
        val app = launch(account)

        app.moveToState(Lifecycle.State.CREATED)

        assertEquals(2, account.syncs)
        assertEquals(1, account.looks)
    }

    // 端末を回したときなど、画面を作り直すだけのときは、同期しない。アプリは前にいたままである。
    @Test
    fun the_app_does_not_sync_when_the_screen_is_only_made_again() {
        val account = FakeAccount()
        val app = launch(account)

        app.recreate()

        assertEquals(1, account.looks)
        assertEquals(1, account.syncs)
    }

    // 画面を作り直したあとも、後ろへ回って前に戻れば、同期する。
    @Test
    fun the_app_syncs_when_it_comes_to_the_front_after_the_screen_was_made_again() {
        val account = FakeAccount()
        val app = launch(account)
        app.recreate()

        app.moveToState(Lifecycle.State.CREATED)
        app.moveToState(Lifecycle.State.RESUMED)

        assertEquals(2, account.looks)
        assertEquals(3, account.syncs)
    }

    // サインインのボタンは、このアクティビティからサインインを始める。
    @Test
    fun the_sign_in_button_starts_the_sign_in_from_the_activity() {
        val account = FakeAccount(signedIn = false)
        val app = launch(account)

        compose.onAllNodesWithText("Sign in to Dropbox").onFirst().performClick()

        compose.waitForIdle()
        app.onActivity { assertEquals(listOf<Any>(it), account.signIns) }
    }

    private companion object {
        /** Longer than a test runs: no edit syncs. */
        const val NEVER_MS = 600_000L
    }
}
