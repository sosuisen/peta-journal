package net.sosuisen.petajournal.screen

import androidx.compose.ui.test.assertIsFocused
import androidx.compose.ui.test.hasContentDescription
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.onFirst
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import java.io.File
import java.io.IOException
import java.io.OutputStream
import java.security.MessageDigest
import net.sosuisen.petajournal.sync.Change
import net.sosuisen.petajournal.sync.Changes
import net.sosuisen.petajournal.sync.Cloud
import net.sosuisen.petajournal.sync.SyncState
import net.sosuisen.petajournal.types.ScreenView
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.runner.RunWith
import org.junit.Test

/** The bar at the top with its menu, the sync with the cloud, and the sign-in. */
@RunWith(AndroidJUnit4::class)
class BarAndCloudTest : JournalScreenFixture() {
    /** A cloud with one page, or one that cannot be reached. It keeps what is uploaded. */
    private class OnePage(var reachable: Boolean = true) : Cloud {
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
            return Change.File(path, contentHash(from), remote = "next")
        }

        override fun contentHash(file: File): String = hashOf(file.readBytes())

        /** The content hash of this cloud. It is not the one of Dropbox: the sync asks the cloud. */
        private fun hashOf(content: ByteArray): String =
            MessageDigest.getInstance("SHA-256").digest(content).joinToString("") { "%02x".format(it) }
    }

    /** Opens the menu beside the search field, and touches an entry of it. */
    private fun menu(entry: String) {
        compose.onNode(hasContentDescription("Menu")).performClick()
        compose.onNodeWithText(entry).performClick()
    }

    // 項目を編集している間も、メニューを開ける。開いても、編集は続き、キーボードは消えない。
    @Test
    fun the_menu_opens_while_an_item_is_edited_and_the_edit_goes_on() {
        download("notes.md", "text" + NL)
        start(OnePage())
        type("notes")
        waitFor("notes")
        compose.onAllNodes(shown("notes")).onFirst().performClick()
        waitFor("text")
        compose.onNode(hasText("text"), useUnmergedTree = true).performClick()
        compose.onNodeWithTag(EDITOR_TAG).assertIsFocused()

        compose.onNode(hasContentDescription("Menu")).performClick()

        compose.onNodeWithText("Sync").assertExists()
        compose.onNodeWithTag(EDITOR_TAG).assertIsFocused()
    }

    // クラウドのページは、同期のあと、検索で見つかる。
    @Test
    fun a_page_of_the_cloud_is_found_by_the_search_after_a_sync() {
        start(OnePage())

        menu("Sync")
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
        menu("Sync")
        compose.waitUntil(WAIT_MS) { File(root, "markdown/docs/trip.md").exists() }
        compose.waitUntil(WAIT_MS) { sync?.state == SyncState.Idle && !shell.syncing }
        File(root, "markdown/docs/trip.md").writeText("大阪へ行きました。\n")

        menu("Sync")

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

    /** Opens a new app that no one has signed in to, as the app does after it is installed. */
    private fun startNew(cloud: Cloud) {
        start(cloud, signedIn = false)
        compose.runOnUiThread { shell.restore(make = false) }
    }

    // 入れたばかりのアプリは、サインインの前に、サインインのボタンだけを出す。今日のジャーナルを作らず、上端のバーも出さない。
    @Test
    fun a_new_app_shows_only_the_sign_in_before_the_sign_in() {
        startNew(OnePage())

        compose.onNodeWithText("Sign in to Dropbox").assertExists()
        compose.onNodeWithTag(SEARCH_TAG).assertDoesNotExist()
        compose.onNode(hasContentDescription("Today")).assertDoesNotExist()
        compose.runOnIdle {
            assertFalse(shell.view.screen is ScreenView.Journal)
            assertEquals(0, edits)
        }
    }

    // サインインのあとの最初の同期が終わると、今日のジャーナルが開き、上端のバーが出る。
    @Test
    fun the_first_sync_after_the_sign_in_opens_today_in_a_new_app() {
        val cloud = OnePage()
        startNew(cloud)

        compose.runOnUiThread { sync?.start(cloud) }

        compose.waitUntil(WAIT_MS) { shell.view.screen is ScreenView.Journal }
        compose.onNodeWithTag(SEARCH_TAG).assertExists()
        compose.onNodeWithText("Sign in to Dropbox").assertDoesNotExist()
        compose.runOnIdle { assertTrue(File(root, "markdown/docs/trip.md").exists()) }
    }

    // サインアウトすると、サインインのボタンに戻る。取り込んだページは、そのまま読める。
    @Test
    fun a_sign_out_brings_the_sign_in_back_and_keeps_the_pages() {
        start(OnePage())
        menu("Sync")
        compose.waitUntil(WAIT_MS) { File(root, "markdown/docs/trip.md").exists() }
        compose.waitUntil(WAIT_MS) { sync?.state == SyncState.Idle && !shell.syncing }

        menu("Sign out")

        compose.onNodeWithText("Sign in to Dropbox").assertExists()
        compose.onNode(hasContentDescription("Menu")).assertDoesNotExist()
        compose.runOnIdle { assertEquals(1, signOuts) }
        type("会議")
        waitFor("trip")
    }

    // 失敗した同期は、「Try again」でやり直せる。
    @Test
    fun a_sync_that_failed_is_tried_again_from_its_message() {
        val cloud = OnePage(reachable = false)
        start(cloud)
        menu("Sync")
        waitFor("Sync failed: the network is gone")
        cloud.reachable = true

        compose.onNodeWithText("Try again").performClick()

        compose.waitUntil(WAIT_MS) { File(root, "markdown/docs/trip.md").exists() }
    }
}
