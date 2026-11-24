package com.sosuisha.petajournal.dropbox

import android.content.Context
import android.content.SharedPreferences
import com.sosuisha.petajournal.core.PetaDropboxSdk
import com.sosuisha.petajournal.sync.CloudAccount

/** Where the credential of a sign-in that may write is kept. */
private const val CREDENTIAL = "credential-write"

/** Where a version of the app that only read kept its credential. It cannot upload. */
private const val READ_ONLY = "credential"

/**
 * The account of Dropbox that the app is signed in to: the sign-in, the sign-out, and the calls of
 * the SDK that the sync of the core makes. All that the app knows of the Dropbox SDK outside this
 * package goes through here.
 *
 * @param kept where the credential of Dropbox is kept.
 * @param key the App key of the Dropbox app. It is empty in an app that was built without one.
 * @param folder the path of the Markdown folder in Dropbox, such as `/PetaJournal`.
 */
class DropboxAccount
internal constructor(
    private val kept: SharedPreferences,
    private val key: String,
    override val folder: String,
    private val sdk: DropboxSdk,
) : CloudAccount {
    /**
     * The account with the Dropbox SDK. The credential is in the storage of the app only. The app
     * is in no backup.
     */
    constructor(
        context: Context,
        key: String,
        folder: String,
    ) : this(
        context.getSharedPreferences("dropbox", Context.MODE_PRIVATE),
        key,
        folder,
        RealDropboxSdk,
    )

    init {
        // A sign-in of a version that only read cannot upload: the user signs in again
        kept.getString(READ_ONLY, null)?.let { old ->
            kept.edit().remove(READ_ONLY).apply()
            sdk.revoke(old)
        }
    }

    override val isSetUp: Boolean
        get() = key.isNotEmpty()

    override val isSignedIn: Boolean
        get() = kept.contains(CREDENTIAL)

    override fun signIn(from: Context) = sdk.startSignIn(from, key)

    override fun keepNewSignIn() {
        val fresh = sdk.newSignIn() ?: return
        kept.edit().putString(CREDENTIAL, fresh).apply()
    }

    override fun calls(): PetaDropboxSdk? {
        val credential = kept.getString(CREDENTIAL, null) ?: return null
        return sdk.calls(credential)
    }

    /** Lets go of the credential, and tells Dropbox that it is no longer to be taken. */
    override fun signOut() {
        val credential = kept.getString(CREDENTIAL, null)
        kept.edit().remove(CREDENTIAL).apply()
        // The SDK keeps the answer of the last sign-in, and the next resume would take it again
        sdk.forgetSignIn()
        if (credential != null) sdk.revoke(credential)
    }
}
