package com.sosuisha.petajournal.screen

import android.os.Handler
import android.os.Looper
import androidx.compose.ui.test.hasAnyAncestor
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.isDialog
import androidx.compose.ui.test.onNodeWithText
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.sosuisha.petajournal.shell.Shell
import com.sosuisha.petajournal.sync.SyncState
import java.io.File
import org.junit.Test
import org.junit.runner.RunWith

/** The dialog that shows a sync while the user waits for it. */
@RunWith(AndroidJUnit4::class)
class SyncDialogTest : JournalScreenFixture() {
    /** Opens the app on the folders with the sync at [state], as it is while a sync runs. */
    private fun startAt(state: SyncState, syncInFront: Boolean) {
        val main = Handler(Looper.getMainLooper())
        compose.runOnUiThread {
            shell = Shell(File(root, "data"), File(root, "markdown"), { main.post(it) })
            shell.sync()
        }
        compose.setContent { JournalScreen(shell, state, syncInFront = syncInFront) }
        compose.waitUntil(WAIT_MS) { !shell.syncing }
    }

    // アプリが前に出たときの同期がファイルを運ぶ間は、進み具合がダイアログに出る。
    @Test
    fun a_sync_in_front_that_moves_files_shows_a_dialog() {
        startAt(SyncState.Downloading(2, 5), syncInFront = true)

        compose.onNode(isDialog()).assertExists()
        compose.onNode(hasText("Downloading 2 of 5") and hasAnyAncestor(isDialog())).assertExists()
    }

    // 上げる間も、ダイアログに出る。
    @Test
    fun a_sync_in_front_that_uploads_shows_a_dialog() {
        startAt(SyncState.Uploading(1, 2), syncInFront = true)

        compose.onNode(hasText("Uploading 1 of 2") and hasAnyAncestor(isDialog())).assertExists()
    }

    // 運ぶファイルがまだ分からない間は、ダイアログは出ない。
    @Test
    fun a_sync_in_front_that_knows_of_no_file_yet_shows_no_dialog() {
        startAt(SyncState.Downloading(0, 0), syncInFront = true)
        compose.onNode(isDialog()).assertDoesNotExist()
    }

    // 編集のあとの同期では、ダイアログは出ない。進み具合は、ページの下の知らせに出る。
    @Test
    fun a_sync_after_an_edit_shows_no_dialog() {
        startAt(SyncState.Downloading(2, 5), syncInFront = false)

        compose.onNode(isDialog()).assertDoesNotExist()
        compose.onNodeWithText("Downloading 2 of 5").assertExists()
    }
}
