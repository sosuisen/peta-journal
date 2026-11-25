package com.sosuisha.petajournal

import android.app.Application
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.os.Handler
import android.os.Looper
import android.util.Log
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import com.sosuisha.petajournal.dropbox.DropboxAccount
import com.sosuisha.petajournal.shell.Shell
import com.sosuisha.petajournal.sync.CloudAccount
import com.sosuisha.petajournal.sync.CloudSync
import java.io.File

/** How long the sync waits after the last change of a page before it starts (ADR 055). */
private const val AFTER_EDIT_MS = 3_000L

/**
 * Keeps the shell while the screen is made again, as when the device is turned. The journal can be
 * open only once, and closes when the app is left for good.
 *
 * @param filesDir where the app keeps its files: the Markdown folder, the data of the core, and
 *   what the sync knows of the cloud.
 * @param cacheDir where a download is written before it is put in its place.
 * @param account the account of the cloud storage.
 * @param afterEditMs how long the sync waits after the last change of a page before it starts.
 */
class JournalModel
internal constructor(
    app: Application,
    filesDir: File,
    cacheDir: File,
    private val account: CloudAccount,
    afterEditMs: Long,
) : AndroidViewModel(app) {
    /** The model of the app: its own folders, and the account of Dropbox. */
    constructor(
        app: Application,
    ) : this(
        app,
        app.filesDir,
        app.cacheDir,
        DropboxAccount(app, BuildConfig.DROPBOX_KEY, BuildConfig.DROPBOX_FOLDER),
        AFTER_EDIT_MS,
    )

    private val main = Handler(Looper.getMainLooper())

    /** The Markdown folder of ADR 030. The core makes the folders in it. */
    private val markdown = File(filesDir, "markdown")

    private val clipboard = app.getSystemService(ClipboardManager::class.java)

    val shell =
        Shell(
            File(filesDir, "data"),
            markdown,
            { main.post(it) },
            copy = { clipboard.setPrimaryClip(ClipData.newPlainText("Markdown", it)) },
        )

    /**
     * `null` when the account is not set up, as in an app that was built without the App key of
     * Dropbox: it then syncs nothing.
     */
    val cloud: CloudSync? =
        if (!account.isSetUp) {
            null
        } else {
            CloudSync(
                shell = shell,
                afterChangeMs = afterEditMs,
                temp = File(cacheDir, "downloads"),
                // Where the last download ended, for the next one to ask only for what changed
                // since, and the versions of the files in Dropbox that the app has seen
                cursor = File(filesDir, "dropbox-cursor"),
                record = File(filesDir, "dropbox-record"),
                signedIn = account.isSignedIn,
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
     * Syncs with the cloud: downloads what changed there, and uploads the pages that changed in the
     * app. Nothing happens before the first sign-in. [inFront] says that the user waits for this
     * sync: the screen shows it in a dialog.
     */
    fun sync(inFront: Boolean = false) {
        val sync = cloud ?: return
        sync.start(account.calls() ?: return, account.folder, inFront)
    }

    /**
     * The app went to the back: what was typed and still waits for its save is saved, also without
     * a sync, and then it syncs (#766).
     */
    fun wentToTheBack() {
        shell.wentToTheBack()
        sync()
    }

    /**
     * True from the time the screen is made again, as when the device is turned, until the new
     * screen is in front.
     */
    private var madeAgain = false

    /**
     * The screen is made again, as when the device is turned: the app stays in front, and the next
     * [syncInFront] syncs nothing.
     */
    fun screenIsMadeAgain() {
        madeAgain = true
    }

    /** The app came to the front, maybe back from the sign-in: it syncs, and the user waits. */
    fun syncInFront() {
        if (madeAgain) {
            madeAgain = false
            return
        }
        if (cloud == null) return
        account.keepNewSignIn()
        sync(inFront = true)
    }

    /**
     * Signs out of the cloud. The pages and the images that were downloaded stay, and can be read.
     * The app syncs nothing until the next sign-in.
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
        shell.close()
    }

    /** Waits, for at most [ms], until the journal is let go of after [close]. */
    internal fun awaitClosed(ms: Long): Boolean = shell.awaitClosed(ms)

    companion object {
        /**
         * Makes the model for the screen. A test of the screen puts a model on its own folders and
         * with its own account here.
         */
        internal var make: (Application) -> JournalModel = ::JournalModel

        internal val factory = viewModelFactory {
            initializer {
                make(checkNotNull(this[ViewModelProvider.AndroidViewModelFactory.APPLICATION_KEY]))
            }
        }
    }
}
