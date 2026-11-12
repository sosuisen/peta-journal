package net.sosuisen.petajournal

import android.app.Application
import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.webkit.MimeTypeMap
import android.widget.Toast
import androidx.core.content.FileProvider
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import androidx.lifecycle.AndroidViewModel
import com.dropbox.core.DbxException
import com.dropbox.core.DbxRequestConfig
import com.dropbox.core.android.Auth
import com.dropbox.core.android.AuthActivity
import com.dropbox.core.oauth.DbxCredential
import com.dropbox.core.v2.DbxClientV2
import net.sosuisen.petajournal.sync.CloudRecord
import net.sosuisen.petajournal.sync.CloudSync
import net.sosuisen.petajournal.sync.Downloader
import net.sosuisen.petajournal.sync.DropboxCloud
import net.sosuisen.petajournal.sync.MarkdownFolder
import net.sosuisen.petajournal.sync.SyncRound
import net.sosuisen.petajournal.sync.Uploader
import java.io.File
import kotlin.concurrent.thread

/**
 * The app reads the folder in Dropbox, and writes the pages that changed in the app. It
 * deletes and moves nothing there (ADR 031).
 */
private val SCOPES = listOf("files.metadata.read", "files.content.read", "files.content.write")

/** How long the app waits after the last change of a page before it syncs. */
private const val AFTER_EDIT_MS = 3_000L

/** Where the credential of a sign-in that may write is kept. */
private const val CREDENTIAL = "credential-write"

/** Where a version of the app that only read kept its credential. It cannot upload. */
private const val READ_ONLY = "credential"
private val DROPBOX = DbxRequestConfig("peta-journal-android")

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

    /** The credential of Dropbox, in the storage of the app only. The app is in no backup. */
    private val kept = app.getSharedPreferences("dropbox", Context.MODE_PRIVATE)

    /** Where the last download ended, for the next one to ask only for what changed since. */
    private val cursor = File(app.filesDir, "dropbox-cursor")

    /** The versions of the files in Dropbox that the app has seen, for the upload. */
    private val record = CloudRecord(File(app.filesDir, "dropbox-record"))

    /** `null` when the app was built without the App key of Dropbox: it then syncs nothing. */
    val cloud: CloudSync? = if (BuildConfig.DROPBOX_KEY.isEmpty()) {
        null
    } else {
        val folder = MarkdownFolder(markdown, File(app.cacheDir, "downloads"))
        CloudSync(
            signedIn = kept.contains(CREDENTIAL),
            round = { SyncRound(Downloader(it, folder, cursor, record), shell::syncAndWait, Uploader(it, folder, record)) },
            onMainThread = { main.post(it) },
        )
    }

    init {
        // A sign-in of a version that only read cannot upload: the user signs in again
        kept.getString(READ_ONLY, null)?.let { old ->
            kept.edit().remove(READ_ONLY).apply()
            revoke(old)
        }
        // What an earlier download left, when the app ended before it was taken in
        shell.sync()
    }

    /**
     * Syncs with Dropbox: downloads what changed there, and uploads the pages that changed
     * in the app. [fresh] is the credential of a sign-in that has just
     * ended, if any. Nothing happens before the first sign-in. [inFront] says that the user
     * waits for this sync: the screen shows it in a dialog.
     */
    fun sync(fresh: DbxCredential? = null, inFront: Boolean = false) {
        val sync = cloud ?: return
        if (fresh != null) kept.edit().putString(CREDENTIAL, DbxCredential.Writer.writeToString(fresh)).apply()
        val credential = kept.getString(CREDENTIAL, null) ?: return
        // The client gets a new access token by itself when the old one has run out
        val client = DbxClientV2(DROPBOX, DbxCredential.Reader.readFully(credential))
        sync.start(DropboxCloud(client, BuildConfig.DROPBOX_FOLDER), inFront)
    }

    /**
     * Signs out of Dropbox. The pages and the images that were downloaded stay, and can be
     * read. The app syncs nothing until the next sign-in.
     */
    fun signOut() {
        val sync = cloud ?: return
        val credential = kept.getString(CREDENTIAL, null)
        kept.edit().remove(CREDENTIAL).apply()
        // The SDK keeps the answer of the last sign-in, and the next resume would take it again
        AuthActivity.result = null
        // The next sign-in may be to another account, where this cursor means nothing. Without
        // it, the next download lists everything: it takes only the files that differ, and
        // deletes the copies of what the account does not hold. The record of the versions
        // stays: it tells a copy from what was written here and is not uploaded yet
        cursor.delete()
        sync.signOut()
        if (credential != null) revoke(credential)
    }

    /**
     * Tells Dropbox that the token is no longer to be taken. Without the network, Dropbox does
     * not hear of it: the token is then gone from the device only.
     */
    private fun revoke(credential: String) {
        val client = DbxClientV2(DROPBOX, DbxCredential.Reader.readFully(credential))
        thread(name = "dropbox-sign-out") {
            try {
                client.auth().tokenRevoke()
            } catch (unheard: DbxException) {
                // Nothing to do: the device has let go of the token
            }
        }
    }

    override fun onCleared() {
        main.removeCallbacks(afterEdit)
        shell.close()
    }
}

class MainActivity : ComponentActivity() {
    private val model: JournalModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            JournalScreen(
                model.shell,
                model.cloud?.state,
                syncInFront = model.cloud?.inFront == true,
                onSignIn = { Auth.startOAuth2PKCE(this, BuildConfig.DROPBOX_KEY, DROPBOX, SCOPES) },
                onSync = { model.sync() },
                onSignOut = { model.signOut() },
                onFile = ::openWithAnotherApp,
            )
        }
    }

    /**
     * Opens a file of the assets folder with the app that the device has for its kind. The
     * other app may only read the file, and only this one.
     */
    private fun openWithAnotherApp(file: File) {
        val lent = FileProvider.getUriForFile(this, "$packageName.files", file)
        val kind = MimeTypeMap.getSingleton().getMimeTypeFromExtension(file.extension.lowercase()) ?: "*/*"
        val view = Intent(Intent.ACTION_VIEW).setDataAndType(lent, kind).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        try {
            startActivity(view)
        } catch (none: ActivityNotFoundException) {
            Toast.makeText(this, "No app opens ${file.name}", Toast.LENGTH_LONG).show()
        }
    }

    /**
     * The app syncs when it goes to the back: what was written goes up before Android may
     * stop the app. A sync that Android stops goes on when the app opens again.
     */
    override fun onStop() {
        super.onStop()
        // Not when the screen is only made again, as when the device is turned
        if (!isChangingConfigurations) model.sync()
    }

    /** The app syncs whenever it comes to the front, also back from the sign-in. */
    override fun onResume() {
        super.onResume()
        model.sync(Auth.getDbxCredential(), inFront = true)
    }
}
