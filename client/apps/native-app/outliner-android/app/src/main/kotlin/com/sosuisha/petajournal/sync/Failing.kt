package com.sosuisha.petajournal.sync

import com.sosuisha.petajournal.core.PetaDropboxError
import com.sosuisha.petajournal.core.PetaDropboxFile
import com.sosuisha.petajournal.core.PetaDropboxSdk
import com.sosuisha.petajournal.core.PetaListing
import java.io.IOException

/**
 * The calls of the SDK, with each failure as one that the sync of the core knows (ADR 054).
 * The core hears only of a [PetaDropboxError]: any other exception would not reach it, and
 * would end the app.
 */
internal class Failing(private val sdk: PetaDropboxSdk) : PetaDropboxSdk {
    override fun list(folder: String): PetaListing = failing { sdk.list(folder) }

    override fun listAfter(cursor: String): PetaListing = failing { sdk.listAfter(cursor) }

    override fun download(rev: String, to: String) = failing { sdk.download(rev, to) }

    override fun upload(path: String, from: String, over: String?, modifiedMs: Long): PetaDropboxFile =
        failing { sdk.upload(path, from, over, modifiedMs) }

    private fun <Answer> failing(call: () -> Answer): Answer =
        try {
            call()
        } catch (failed: Exception) {
            throw dropboxFailure(failed)
        }
}

/**
 * What the sync makes of a failure of the SDK. One that came from the network, such as no
 * connection or no answer, is [PetaDropboxError.Offline]: the screen tells the user that there
 * is no connection. The Dropbox SDK wraps an `IOException` of the network in an exception of
 * its own, so the causes count too. Any other is [PetaDropboxError.Other], with its words for
 * the log.
 */
internal fun dropboxFailure(failed: Exception): PetaDropboxError =
    when {
        failed is PetaDropboxError -> failed
        generateSequence<Throwable>(failed) { it.cause }.any { it is IOException } -> PetaDropboxError.Offline(failed.toString())
        else -> PetaDropboxError.Other(failed.toString())
    }
