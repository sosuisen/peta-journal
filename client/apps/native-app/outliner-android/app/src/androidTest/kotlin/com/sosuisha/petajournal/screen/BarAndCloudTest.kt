package com.sosuisha.petajournal.screen

import androidx.compose.ui.test.assertIsFocused
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.getBoundsInRoot
import androidx.compose.ui.test.hasContentDescription
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.onFirst
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performClick
import androidx.compose.ui.unit.dp
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.sosuisha.petajournal.core.PetaDropboxSdk
import com.sosuisha.petajournal.sync.SyncState
import com.sosuisha.petajournal.types.ScreenView
import java.io.File
import java.util.concurrent.CountDownLatch
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** The bar at the top with its menu, the sync with the cloud, and the sign-in. */
@RunWith(AndroidJUnit4::class)
class BarAndCloudTest : JournalScreenFixture() {
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
    private fun startNew(cloud: PetaDropboxSdk) {
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
        compose.runOnIdle { assertFalse(shell.view.screen is ScreenView.Journal) }
    }

    // サインインのあとの最初の同期が終わると、今日のジャーナルが開き、上端のバーが出る。
    @Test
    fun the_first_sync_after_the_sign_in_opens_today_in_a_new_app() {
        val cloud = OnePage()
        startNew(cloud)

        compose.runOnUiThread { sync?.start(cloud, FOLDER) }

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
        waitFor("Sync failed: no connection")
        cloud.reachable = true

        compose.onNodeWithText("Try again").performClick()

        compose.waitUntil(WAIT_MS) { File(root, "markdown/docs/trip.md").exists() }
    }

    // 通信のほかの理由で失敗した同期は、「Sync failed」とだけ伝える。理由の長い言葉は、画面に出さない。
    @Test
    fun a_sync_that_failed_for_another_reason_tells_only_that_it_failed() {
        start(OnePage(refuses = true))
        menu("Sync")

        waitFor("Sync failed")
        compose.onNodeWithText("the cloud refused the sync", substring = true).assertDoesNotExist()
        compose.onNodeWithText("Sync failed: no connection").assertDoesNotExist()
    }

    // 同期のメッセージは、ページの右に寄せて出る。中央に出すと、編集している行の上に重なりやすい（#884）。
    @Test
    fun the_message_of_the_sync_lies_at_the_right_of_the_page() {
        val held = CountDownLatch(1)
        start(OnePage().apply { this.held = held })
        menu("Sync")
        waitFor("Syncing…")

        val message = compose.onNodeWithText("Syncing…").getBoundsInRoot()
        val screen = compose.onRoot().getBoundsInRoot()
        held.countDown()

        assertTrue(message.left + message.right > screen.left + screen.right + 16.dp)
    }

    // 同期のメッセージは、ページの上端に出る。下端に出すと、いちばん下の行で打つ文字に重なる。
    @Test
    fun the_message_of_the_sync_lies_over_the_top_of_the_page() {
        start(OnePage(reachable = false))
        menu("Sync")
        waitFor("Sync failed: no connection")

        val message = compose.onNodeWithText("Sync failed: no connection").getBoundsInRoot()
        val bar = compose.onNodeWithTag(SEARCH_TAG).getBoundsInRoot()
        val screen = compose.onRoot().getBoundsInRoot()

        val height = screen.bottom - screen.top
        assertTrue(message.top - bar.bottom < height / 4)
    }

    // 同期が失敗したあとは、メニューにサインインの項目が出る。Dropboxが受け付けなくなった資格情報を、新しいサインインで置き換えられる。
    @Test
    fun after_a_failed_sync_the_menu_offers_the_sign_in() {
        start(OnePage(reachable = false))
        menu("Sync")
        waitFor("Sync failed: no connection")

        menu("Sign in to Dropbox")

        compose.runOnIdle { assertEquals(1, signIns) }
    }

    // 同期が動いている間は、メニューの「Sync」と「Sign out」は押せない。
    @Test
    fun the_menu_asks_nothing_of_a_sync_that_runs() {
        val cloud = OnePage().apply { held = CountDownLatch(1) }
        start(cloud)
        menu("Sync")

        compose.onNode(hasContentDescription("Menu")).performClick()

        compose.onNodeWithText("Sync").assertIsNotEnabled()
        compose.onNodeWithText("Sign out").assertIsNotEnabled()
        cloud.held?.countDown()
        compose.waitUntil(WAIT_MS) { sync?.state == SyncState.Idle }
    }
}
