package net.sosuisen.petajournal.sync

import android.content.Context

/**
 * The account of the cloud storage that the app is signed in to: the sign-in, the sign-out,
 * and the Markdown folder of the account. The app knows the storage only by this, so a test
 * puts its own account here.
 */
interface CloudAccount {
    /** False in an app that was built without what the storage asks of an app: it syncs nothing. */
    val isSetUp: Boolean

    val isSignedIn: Boolean

    /** Starts the sign-in, in the browser. The app comes to the front again when it has ended. */
    fun signIn(from: Context)

    /**
     * Keeps the credential of a sign-in that has just ended, if any. It is for the time the
     * app comes to the front: the sign-in ends in the browser.
     */
    fun keepNewSignIn()

    /** The Markdown folder of the account. `null` before the sign-in. */
    fun cloud(): Cloud?

    /** Lets go of the credential, and tells the storage that it is no longer to be taken. */
    fun signOut()
}
