package com.sosuisha.petajournal.dropbox

import com.sosuisha.petajournal.core.PetaDropboxError
import java.io.IOException

/**
 * What the sync of the core makes of a failure of the Dropbox SDK (ADR 054). One that came from the
 * network, such as no connection or no answer, is [PetaDropboxError.Offline]: the screen tells the
 * user that there is no connection. The Dropbox SDK wraps an `IOException` of the network in an
 * exception of its own, so the causes count too. Any other is [PetaDropboxError.Other], with its
 * words for the log.
 */
internal fun dropboxFailure(failed: Exception): PetaDropboxError =
    when {
        failed is PetaDropboxError -> failed
        generateSequence<Throwable>(failed) { it.cause }.any { it is IOException } ->
            PetaDropboxError.Offline(failed.toString())
        else -> PetaDropboxError.Other(failed.toString())
    }
