package com.sosuisha.petajournal

import android.content.Context
import com.sosuisha.petajournal.core.PetaDropboxSdk
import com.sosuisha.petajournal.screen.FOLDER
import com.sosuisha.petajournal.screen.OnePage
import com.sosuisha.petajournal.sync.CloudAccount

/** An account of a cloud with one page. It keeps what the app asked of it. */
internal class FakeAccount(override val isSetUp: Boolean = true, signedIn: Boolean = true) :
    CloudAccount {
    @Volatile
    override var isSignedIn = signedIn
        private set

    /** True when a sign-in has ended in the browser, and the app has not kept it yet. */
    var fresh = false

    /** What the sign-in was started from. */
    val signIns = mutableListOf<Context>()

    /** How often the app came to the front and looked for a new sign-in. */
    @Volatile
    var looks = 0
        private set

    /** How often the calls were asked for: once for each sync that the app started. */
    @Volatile
    var syncs = 0
        private set

    val page = OnePage()

    override val folder = FOLDER

    override fun signIn(from: Context) {
        signIns += from
    }

    override fun keepNewSignIn() {
        looks++
        if (fresh) isSignedIn = true
        fresh = false
    }

    override fun calls(): PetaDropboxSdk? = if (isSignedIn) page.also { syncs++ } else null

    override fun signOut() {
        isSignedIn = false
    }
}
