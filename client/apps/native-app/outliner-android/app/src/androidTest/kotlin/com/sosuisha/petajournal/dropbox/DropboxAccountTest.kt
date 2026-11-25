package com.sosuisha.petajournal.dropbox

import android.content.Context
import android.content.SharedPreferences
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.sosuisha.petajournal.core.PetaDropboxEntry
import com.sosuisha.petajournal.core.PetaDropboxFile
import com.sosuisha.petajournal.core.PetaDropboxSdk
import com.sosuisha.petajournal.core.PetaListing
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * The sign-in and the sign-out. The Dropbox SDK is an SDK of the test, and the credential is kept
 * in a place of the test: the credential of the app itself is not touched.
 */
@RunWith(AndroidJUnit4::class)
class DropboxAccountTest {
    private val context: Context = InstrumentationRegistry.getInstrumentation().targetContext
    private lateinit var kept: SharedPreferences
    private val sdk = FakeSdk()

    /** An SDK that keeps what it was asked, and has a sign-in ready when the test gives one. */
    private class FakeSdk : DropboxSdk {
        val started = mutableListOf<Pair<Context, String>>()

        /** The credential of the sign-in that ended in the browser, if any. */
        var fresh: String? = null
        val revoked = mutableListOf<String>()

        /** The credentials that the calls were asked for. */
        val opened = mutableListOf<String>()

        override fun startSignIn(from: Context, key: String) {
            started += from to key
        }

        override fun newSignIn(): String? = fresh

        override fun forgetSignIn() {
            fresh = null
        }

        override fun calls(credential: String): PetaDropboxSdk {
            opened += credential
            return OneFile
        }

        override fun revoke(credential: String) {
            revoked += credential
        }
    }

    /** A Dropbox with one file in the folder of the app. */
    private object OneFile : PetaDropboxSdk {
        override fun list(folder: String) =
            PetaListing(
                listOf(PetaDropboxEntry.File(PetaDropboxFile("$folder/docs/a.md", "hash", "r1"))),
                "c1",
                false,
            )

        override fun listAfter(cursor: String) = PetaListing(emptyList(), cursor, false)

        override fun download(rev: String, to: String) = Unit

        override fun upload(path: String, from: String, over: String?, modifiedMs: Long) =
            PetaDropboxFile(path, "hash", "r2")
    }

    @Before
    fun emptyPlace() {
        kept = context.getSharedPreferences("dropbox-test", Context.MODE_PRIVATE)
        kept.edit().clear().commit()
    }

    @After
    fun dropPlace() {
        context.deleteSharedPreferences("dropbox-test")
    }

    private fun account(key: String = "app-key") = DropboxAccount(kept, key, "/PetaJournal", sdk)

    // App keyなしでビルドしたアプリは、同期の用意ができていない。
    @Test
    fun an_app_without_the_app_key_is_not_set_up() {
        assertFalse(account(key = "").isSetUp)
        assertTrue(account(key = "app-key").isSetUp)
    }

    // サインインは、App keyを付けて、SDKに始めさせる。
    @Test
    fun the_sign_in_starts_in_the_sdk_with_the_app_key() {
        account().signIn(context)

        assertEquals(listOf(context to "app-key"), sdk.started)
    }

    // サインインが終わる前は、サインインしていない。SDKの呼び出しもない。
    @Test
    fun before_a_sign_in_has_ended_the_account_is_signed_out_and_has_no_calls() {
        val account = account()

        account.keepNewSignIn()

        assertFalse(account.isSignedIn)
        assertNull(account.calls())
    }

    // ブラウザーで終わったサインインの資格情報を、アプリが前に出たときに取っておく。
    @Test
    fun the_credential_of_a_sign_in_that_ended_is_kept() {
        val account = account()
        sdk.fresh = "credential-1"

        account.keepNewSignIn()

        assertTrue(account.isSignedIn)
    }

    // 資格情報は、アプリを開き直しても残る。
    @Test
    fun the_sign_in_lasts_when_the_app_opens_again() {
        sdk.fresh = "credential-1"
        account().keepNewSignIn()

        assertTrue(account().isSignedIn)
    }

    // サインインしたあとは、取っておいた資格情報でSDKの呼び出しを同期に渡す。同期が読むのは、設定のフォルダーである。
    @Test
    fun the_account_gives_the_calls_of_the_kept_credential_and_its_folder() {
        val account = account()
        sdk.fresh = "credential-1"
        account.keepNewSignIn()

        val listing = checkNotNull(account.calls()).list(account.folder)

        assertEquals(listOf("credential-1"), sdk.opened)
        assertEquals(
            listOf(PetaDropboxEntry.File(PetaDropboxFile("/PetaJournal/docs/a.md", "hash", "r1"))),
            listing.entries,
        )
    }

    // サインアウトは、資格情報を捨て、Dropboxにも無効にさせる。
    @Test
    fun the_sign_out_lets_go_of_the_credential_and_revokes_it() {
        val account = account()
        sdk.fresh = "credential-1"
        account.keepNewSignIn()

        account.signOut()

        assertFalse(account.isSignedIn)
        assertNull(account.calls())
        assertEquals(listOf("credential-1"), sdk.revoked)
    }

    // サインアウトのあと、アプリが前に出ても、前のサインインの答えを取り直さない。
    @Test
    fun after_the_sign_out_the_answer_of_the_last_sign_in_is_not_taken_again() {
        val account = account()
        sdk.fresh = "credential-1"
        account.keepNewSignIn()
        account.signOut()

        account.keepNewSignIn()

        assertFalse(account.isSignedIn)
    }

    // サインインしていないときのサインアウトは、Dropboxに何も頼まない。
    @Test
    fun a_sign_out_without_a_sign_in_asks_nothing_of_dropbox() {
        account().signOut()

        assertEquals(emptyList<String>(), sdk.revoked)
    }
}
