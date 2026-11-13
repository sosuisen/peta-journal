package net.sosuisen.petajournal.dropbox

import android.content.Context
import com.dropbox.core.DbxException
import com.dropbox.core.DbxRequestConfig
import com.dropbox.core.android.Auth
import com.dropbox.core.android.AuthActivity
import com.dropbox.core.oauth.DbxCredential
import com.dropbox.core.v2.DbxClientV2
import net.sosuisen.petajournal.sync.Cloud
import kotlin.concurrent.thread

/**
 * The app reads the folder in Dropbox, and writes the pages that changed in the app. It
 * deletes and moves nothing there (ADR 031).
 */
private val SCOPES = listOf("files.metadata.read", "files.content.read", "files.content.write")

/** Where the credential of a sign-in that may write is kept. */
private const val CREDENTIAL = "credential-write"

/** Where a version of the app that only read kept its credential. It cannot upload. */
private const val READ_ONLY = "credential"
private val CONFIG = DbxRequestConfig("peta-journal-android")

/**
 * The account of Dropbox that the app is signed in to: the sign-in, the sign-out, and the
 * Markdown folder of the account as a [Cloud]. All that the app knows of the Dropbox SDK
 * outside this package goes through here.
 *
 * @param key the App key of the Dropbox app. It is empty in an app that was built without
 *   one.
 * @param folder the path of the Markdown folder in Dropbox, such as `/PetaJournal`.
 */
class DropboxAccount(context: Context, private val key: String, private val folder: String) {
    /** The credential of Dropbox, in the storage of the app only. The app is in no backup. */
    private val kept = context.getSharedPreferences("dropbox", Context.MODE_PRIVATE)

    init {
        // A sign-in of a version that only read cannot upload: the user signs in again
        kept.getString(READ_ONLY, null)?.let { old ->
            kept.edit().remove(READ_ONLY).apply()
            revoke(old)
        }
    }

    /** False in an app that was built without the App key of Dropbox: it syncs nothing. */
    val isSetUp: Boolean get() = key.isNotEmpty()

    val isSignedIn: Boolean get() = kept.contains(CREDENTIAL)

    /** Starts the sign-in, in the browser. The app comes to the front again when it has ended. */
    fun signIn(from: Context) = Auth.startOAuth2PKCE(from, key, CONFIG, SCOPES)

    /**
     * Keeps the credential of a sign-in that has just ended, if any. It is for the time the
     * app comes to the front: the sign-in ends in the browser.
     */
    fun keepNewSignIn() {
        val fresh = Auth.getDbxCredential() ?: return
        kept.edit().putString(CREDENTIAL, DbxCredential.Writer.writeToString(fresh)).apply()
    }

    /** The Markdown folder of the account. `null` before the sign-in. */
    fun cloud(): Cloud? {
        val credential = kept.getString(CREDENTIAL, null) ?: return null
        // The client gets a new access token by itself when the old one has run out
        return DropboxCloud(client(credential), folder)
    }

    /** Lets go of the credential, and tells Dropbox that it is no longer to be taken. */
    fun signOut() {
        val credential = kept.getString(CREDENTIAL, null)
        kept.edit().remove(CREDENTIAL).apply()
        // The SDK keeps the answer of the last sign-in, and the next resume would take it again
        AuthActivity.result = null
        if (credential != null) revoke(credential)
    }

    /**
     * Tells Dropbox that the token is no longer to be taken. Without the network, Dropbox does
     * not hear of it: the token is then gone from the device only.
     */
    private fun revoke(credential: String) {
        val client = client(credential)
        thread(name = "dropbox-sign-out") {
            try {
                client.auth().tokenRevoke()
            } catch (unheard: DbxException) {
                // Nothing to do: the device has let go of the token
            }
        }
    }

    private fun client(credential: String) = DbxClientV2(CONFIG, DbxCredential.Reader.readFully(credential))
}
