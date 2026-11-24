package com.sosuisha.petajournal.sync

import android.content.Context
import com.sosuisha.petajournal.core.PetaDropboxSdk

/**
 * The account of the cloud storage that the app is signed in to: the sign-in, the sign-out, and the
 * calls of the SDK of the storage that the sync makes. The app knows the storage only by this, so a
 * test puts its own account here.
 */
interface CloudAccount {
    /** False in an app that was built without what the storage asks of an app: it syncs nothing. */
    val isSetUp: Boolean

    val isSignedIn: Boolean

    /** The path of the Markdown folder in the storage, such as `/PetaJournal`. */
    val folder: String

    /** Starts the sign-in, in the browser. The app comes to the front again when it has ended. */
    fun signIn(from: Context)

    /**
     * Keeps the credential of a sign-in that has just ended, if any. It is for the time the app
     * comes to the front: the sign-in ends in the browser.
     */
    fun keepNewSignIn()

    /** The calls of the SDK of the account, for the sync (ADR 054). `null` before the sign-in. */
    fun calls(): PetaDropboxSdk?

    /** Lets go of the credential, and tells the storage that it is no longer to be taken. */
    fun signOut()
}
