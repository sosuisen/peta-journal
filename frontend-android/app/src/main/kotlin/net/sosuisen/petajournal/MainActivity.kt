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
import com.dropbox.core.DbxRequestConfig
import com.dropbox.core.android.Auth
import com.dropbox.core.oauth.DbxCredential
import com.dropbox.core.v2.DbxClientV2
import net.sosuisen.petajournal.sync.CloudSync
import net.sosuisen.petajournal.sync.Downloader
import net.sosuisen.petajournal.sync.DropboxCloud
import net.sosuisen.petajournal.sync.MarkdownFolder
import java.io.File

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

    /** `null` when the app was built without the App key of Dropbox: it then syncs nothing. */
    val cloud: CloudSync? = if (BuildConfig.DROPBOX_KEY.isEmpty()) {
        null
    } else {
        val folder = MarkdownFolder(markdown, File(app.cacheDir, "downloads"))
        val cursor = File(app.filesDir, "dropbox-cursor")
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
            )
        }
    }

    /** The app syncs whenever it comes to the front, also back from the sign-in. */
    override fun onResume() {
        super.onResume()
        model.sync(Auth.getDbxCredential())
    }
}
