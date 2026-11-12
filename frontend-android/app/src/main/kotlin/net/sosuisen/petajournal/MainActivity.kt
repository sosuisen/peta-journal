package net.sosuisen.petajournal

import android.app.Application
import android.content.Context
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
import net.sosuisen.petajournal.sync.CloudSync
import net.sosuisen.petajournal.sync.Downloader
import net.sosuisen.petajournal.sync.DropboxCloud
import net.sosuisen.petajournal.sync.MarkdownFolder
import java.io.File
import kotlin.concurrent.thread

/** Dropbox only reads: the app changes nothing there (ADR 030). */
private val SCOPES = listOf("files.metadata.read", "files.content.read")
private const val CREDENTIAL = "credential"
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

    val shell = Shell(File(app.filesDir, "data"), markdown) { main.post(it) }

    /** The credential of Dropbox, in the storage of the app only. The app is in no backup. */
    private val kept = app.getSharedPreferences("dropbox", Context.MODE_PRIVATE)

    /** Where the last download ended, for the next one to ask only for what changed since. */
    private val cursor = File(app.filesDir, "dropbox-cursor")

    /** `null` when the app was built without the App key of Dropbox: it then syncs nothing. */
    val cloud: CloudSync? = if (BuildConfig.DROPBOX_KEY.isEmpty()) {
        null
    } else {
        val folder = MarkdownFolder(markdown, File(app.cacheDir, "downloads"))
        CloudSync(
            signedIn = kept.contains(CREDENTIAL),
            downloader = { Downloader(it, folder, cursor) },
            onMainThread = { main.post(it) },
            onDownloaded = shell::sync,
        )
    }

    init {
        // What an earlier download left, when the app ended before it was taken in
        shell.sync()
    }

    /**
     * Downloads what changed in Dropbox. [fresh] is the credential of a sign-in that has just
     * ended, if any. Nothing happens before the first sign-in.
     */
    fun sync(fresh: DbxCredential? = null) {
        val sync = cloud ?: return
        if (fresh != null) kept.edit().putString(CREDENTIAL, DbxCredential.Writer.writeToString(fresh)).apply()
        val credential = kept.getString(CREDENTIAL, null) ?: return
        // The client gets a new access token by itself when the old one has run out
        val client = DbxClientV2(DROPBOX, DbxCredential.Reader.readFully(credential))
        sync.start(DropboxCloud(client, BuildConfig.DROPBOX_FOLDER))
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
        // deletes what the account does not hold
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

    override fun onCleared() = shell.close()
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
                onSignIn = { Auth.startOAuth2PKCE(this, BuildConfig.DROPBOX_KEY, DROPBOX, SCOPES) },
                onSync = { model.sync() },
                onSignOut = { model.signOut() },
            )
        }
    }

    /** The app syncs whenever it comes to the front, also back from the sign-in. */
    override fun onResume() {
        super.onResume()
        model.sync(Auth.getDbxCredential())
    }
}
