package com.sosuisha.petajournal

import android.app.Application
import android.content.Context
import android.os.Handler
import android.os.Looper
import android.util.Log
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.lifecycle.ViewModelProvider
import java.io.File
import com.sosuisha.petajournal.dropbox.DropboxAccount
import com.sosuisha.petajournal.shell.Shell
import com.sosuisha.petajournal.sync.CloudAccount
import com.sosuisha.petajournal.sync.CloudSync

/** How long the app waits after the last change of a page before it syncs. */
private const val AFTER_EDIT_MS = 3_000L

/**
 * Keeps the shell while the screen is made again, as when the device is turned. The journal
 * can be open only once, and closes when the app is left for good.
 *
 * @param filesDir where the app keeps its files: the Markdown folder, the data of the core,
 *   and what the sync knows of the cloud.
 * @param cacheDir where a download is written before it is put in its place.
 * @param account the account of the cloud storage.
 * @param afterEditMs how long the app waits after the last change of a page before it syncs.
 */
class JournalModel internal constructor(
    app: Application,
    filesDir: File,
    cacheDir: File,
    private val account: CloudAccount,
    afterEditMs: Long,
) : AndroidViewModel(app) {
    /** The model of the app: its own folders, and the account of Dropbox. */
    constructor(app: Application) : this(
        app,
        app.filesDir,
        app.cacheDir,
        DropboxAccount(app, BuildConfig.DROPBOX_KEY, BuildConfig.DROPBOX_FOLDER),
        AFTER_EDIT_MS,
    )

    private val main = Handler(Looper.getMainLooper())

    // The folders of ADR 030. Kotlin makes the folders of the pages: the core takes in
    // nothing from a Markdown folder that has neither of them
    private val markdown = File(filesDir, "markdown").also { root ->
        for (name in listOf("docs", "journals", "assets")) File(root, name).mkdirs()
    }

    /** Syncs a moment after the last change of a page, and not once for each key. */
    private val afterEdit = Runnable { sync() }

    val shell = Shell(File(filesDir, "data"), markdown, { main.post(it) }) {
        main.removeCallbacks(afterEdit)
        main.postDelayed(afterEdit, afterEditMs)
    }

    /** `null` when the account is not set up, as in an app that was built without the App key of Dropbox: it then syncs nothing. */
    val cloud: CloudSync? = if (!account.isSetUp) {
        null
    } else {
        CloudSync(
            markdown = markdown,
            temp = File(cacheDir, "downloads"),
            // Where the last download ended, for the next one to ask only for what changed
            // since, and the versions of the files in Dropbox that the app has seen
            cursor = File(filesDir, "dropbox-cursor"),
            record = File(filesDir, "dropbox-record"),
            signedIn = account.isSignedIn,
            takeIn = shell::syncAndWait,
            onMainThread = { main.post(it) },
            log = { Log.w("PetaSync", "The sync failed: $it") },
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
        sync.start(account.calls() ?: return, account.folder, inFront)
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
        // The core forgets where the last download ended: the next sign-in may be to another
        // account. The record of the versions stays: it tells a copy from what was written
        // here and is not uploaded yet
        sync.signOut()
    }

    override fun onCleared() = close()

    /** Closes the journal. The model does nothing more after it. */
    internal fun close() {
        main.removeCallbacks(afterEdit)
        shell.close()
    }

    companion object {
        /**
         * Makes the model for the screen. A test of the screen puts a model on its own folders
         * and with its own account here.
         */
        internal var make: (Application) -> JournalModel = ::JournalModel

        internal val factory = viewModelFactory {
            initializer { make(checkNotNull(this[ViewModelProvider.AndroidViewModelFactory.APPLICATION_KEY])) }
        }
    }
}
