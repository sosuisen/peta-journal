package com.sosuisha.petajournal.dropbox

import com.dropbox.core.NetworkIOException
import com.sosuisha.petajournal.core.PetaDropboxError
import java.io.IOException
import java.net.UnknownHostException
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * What the sync makes of the failures of the Dropbox SDK. The screen tells only whether there was
 * no connection (ADR 054).
 */
class DropboxFailuresTest {
    // SDKが包んだ通信の失敗は、通信の失敗になる。通信を切ったときにSDKが投げる形である。
    @Test
    fun a_failure_of_the_network_that_the_sdk_wrapped_is_offline() {
        val failure = dropboxFailure(NetworkIOException(UnknownHostException("api.dropboxapi.com")))
        assertTrue(failure is PetaDropboxError.Offline)
    }

    // 入出力の失敗も、通信の失敗になる。
    @Test
    fun an_io_failure_is_offline() {
        assertTrue(dropboxFailure(IOException("the network is gone")) is PetaDropboxError.Offline)
    }

    // ほかの失敗は、通信の失敗ではない。言葉はログのために残る。
    @Test
    fun any_other_failure_is_another_failure_with_its_words() {
        val failure = dropboxFailure(IllegalStateException("the cloud refused the sync"))
        assertTrue((failure as PetaDropboxError.Other).why.contains("the cloud refused the sync"))
    }

    // 同期の分かる失敗は、そのまま渡る。
    @Test
    fun a_failure_that_the_sync_knows_goes_on_as_it_is() {
        assertEquals(
            PetaDropboxError.UploadConflict,
            dropboxFailure(PetaDropboxError.UploadConflict),
        )
    }
}
