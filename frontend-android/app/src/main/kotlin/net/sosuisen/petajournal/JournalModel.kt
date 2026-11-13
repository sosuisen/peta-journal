package net.sosuisen.petajournal

import android.app.Application
import android.content.Context
import android.os.Handler
import android.os.Looper
import androidx.lifecycle.AndroidViewModel
import net.sosuisen.petajournal.dropbox.DropboxAccount
import net.sosuisen.petajournal.sync.CloudRecord
import net.sosuisen.petajournal.sync.CloudSync
import net.sosuisen.petajournal.sync.Downloader
import net.sosuisen.petajournal.sync.MarkdownFolder
import net.sosuisen.petajournal.sync.SyncRound
import net.sosuisen.petajournal.sync.Uploader
import java.io.File

/** How long the app waits after the last change of a page before it syncs. */
private const val AFTER_EDIT_MS = 3_000L

/**
 * Keeps the shell while the screen is made again, as when the device is turned. The journal
 * can be open only once, and closes when the app is left for good.
 */
class JournalModel(app: Application) : AndroidViewModel(app) {
    private val main = Handler(Looper.getMainLooper())

    // The folders of ADR 030. Kotlin makes the folders of the pages: the core takes in
    // nothing from a Markdown folder that has neither of them
    private val markdown = File(app.filesDir, "markdown").also { root ->
        for (name in listOf("docs", "journals", "assets")) File(root, name).mkdirs()
    }

    /** Syncs a moment after the last change of a page, and not once for each key. */
    private val afterEdit = Runnable { sync() }

    val shell = Shell(File(app.filesDir, "data"), markdown, { main.post(it) }) {
        main.removeCallbacks(afterEdit)
        main.postDelayed(afterEdit, AFTER_EDIT_MS)
    }

    private val account = DropboxAccount(app, BuildConfig.DROPBOX_KEY, BuildConfig.DROPBOX_FOLDER)

    /** Where the last download ended, for the next one to ask only for what changed since. */
    private val cursor = File(app.filesDir, "dropbox-cursor")

    /** The versions of the files in Dropbox that the app has seen, for the upload. */
    private val record = CloudRecord(File(app.filesDir, "dropbox-record"))

    /** `null` when the app was built without the App key of Dropbox: it then syncs nothing. */
    val cloud: CloudSync? = if (!account.isSetUp) {
        null
    } else {
        val folder = MarkdownFolder(markdown, File(app.cacheDir, "downloads"))
        CloudSync(
            signedIn = account.isSignedIn,
            round = { SyncRound(Downloader(it, folder, cursor, record), shell::syncAndWait, Uploader(it, folder, record)) },
            onMainThread = { main.post(it) },
        )
    }

    init {
        // What an earlier download left, when the app ended before it was taken in
        shell.sync()
        // Before the sign-in, the journal of today is not made: the first download may bring it
        shell.restore(make = cloud == null || account.isSignedIn)
    }

    /** Starts the sign-in to the cloud. The app comes to the front again when it has ended. */
    fun signIn(from: Context) = account.signIn(from)

    /**
     * Syncs with the cloud: downloads what changed there, and uploads the pages that changed
     * in the app. Nothing happens before the first sign-in. [inFront] says that the user
     * waits for this sync: the screen shows it in a dialog.
     */
    fun sync(inFront: Boolean = false) {
        val sync = cloud ?: return
        sync.start(account.cloud() ?: return, inFront)
    }

    /** The app came to the front, maybe back from the sign-in: it syncs, and the user waits. */
    fun syncInFront() {
        if (cloud == null) return
        account.keepNewSignIn()
        sync(inFront = true)
    }

    /**
     * Signs out of the cloud. The pages and the images that were downloaded stay, and can be
     * read. The app syncs nothing until the next sign-in.
     */
    fun signOut() {
        val sync = cloud ?: return
        account.signOut()
        // The next sign-in may be to another account, where this cursor means nothing. Without
        // it, the next download lists everything: it takes only the files that differ, and
        // deletes the copies of what the account does not hold. The record of the versions
        // stays: it tells a copy from what was written here and is not uploaded yet
        cursor.delete()
        sync.signOut()
    }

    override fun onCleared() {
        main.removeCallbacks(afterEdit)
        shell.close()
    }
}
