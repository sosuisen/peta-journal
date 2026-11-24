package com.sosuisha.petajournal.dropbox

import android.content.Context
import com.dropbox.core.DbxException
import com.dropbox.core.DbxRequestConfig
import com.dropbox.core.android.Auth
import com.dropbox.core.android.AuthActivity
import com.dropbox.core.oauth.DbxCredential
import com.dropbox.core.v2.DbxClientV2
import com.sosuisha.petajournal.core.PetaDropboxSdk
import kotlin.concurrent.thread

/**
 * What [DropboxAccount] needs of the Dropbox SDK. A credential is a text here, as the app keeps it.
 * A test puts its own SDK here. The app puts [RealDropboxSdk] here.
 */
internal interface DropboxSdk {
    /** Starts the sign-in, in the browser. The app comes to the front again when it has ended. */
    fun startSignIn(from: Context, key: String)

    /** The credential of a sign-in that has just ended. `null` when there is none. */
    fun newSignIn(): String?

    /** Lets go of the answer of the last sign-in, so that [newSignIn] gives it no more. */
    fun forgetSignIn()

    /** The calls of the account that the credential is for, for the sync. */
    fun calls(credential: String): PetaDropboxSdk

    /**
     * Tells Dropbox that the credential is no longer to be taken. It does not wait for the answer.
     */
    fun revoke(credential: String)
}

/**
 * The app reads the folder in Dropbox, and writes the pages that changed in the app. It deletes and
 * moves nothing there (ADR 031).
 */
private val SCOPES = listOf("files.metadata.read", "files.content.read", "files.content.write")
private val CONFIG = DbxRequestConfig("outliner-android")

/** The Dropbox SDK itself. It only passes each call on. No test runs it. */
internal object RealDropboxSdk : DropboxSdk {
    override fun startSignIn(from: Context, key: String) =
        Auth.startOAuth2PKCE(from, key, CONFIG, SCOPES)

    override fun newSignIn(): String? =
        Auth.getDbxCredential()?.let(DbxCredential.Writer::writeToString)

    override fun forgetSignIn() {
        AuthActivity.result = null
    }

    // The client gets a new access token by itself when the old one has run out
    override fun calls(credential: String): PetaDropboxSdk = SdkDropbox(client(credential))

    /**
     * Without the network, Dropbox does not hear of it: the token is then gone from the device
     * only.
     */
    override fun revoke(credential: String) {
        val client = client(credential)
        thread(name = "dropbox-sign-out") {
            try {
                client.auth().tokenRevoke()
            } catch (unheard: DbxException) {
                // Nothing to do: the device has let go of the token
            }
        }
    }

    private fun client(credential: String) =
        DbxClientV2(CONFIG, DbxCredential.Reader.readFully(credential))
}
