package com.sosuisha.petajournal.screen

import com.sosuisha.petajournal.core.PetaDropboxEntry
import com.sosuisha.petajournal.core.PetaDropboxError
import com.sosuisha.petajournal.core.PetaDropboxFile
import com.sosuisha.petajournal.core.PetaDropboxSdk
import com.sosuisha.petajournal.core.PetaListing
import java.io.File
import java.security.MessageDigest
import java.util.concurrent.CountDownLatch

/** The Markdown folder of the tests in their Dropbox. */
internal const val FOLDER = "/PetaJournal"

/**
 * A Dropbox with one page, or one that cannot be reached, or one that [refuses] the sync for
 * another reason. It keeps what is uploaded. It fails as `SdkDropbox` does, with a failure that the
 * sync knows.
 */
internal class OnePage(var reachable: Boolean = true, private val refuses: Boolean = false) :
    PetaDropboxSdk {
    private val text = "京都で会議がありました。\n".toByteArray()

    /** The text of each uploaded file, by its path under the Markdown folder. */
    val uploaded = mutableMapOf<String, String>()

    /** Holds Dropbox back in its answer, until the test lets it go on. */
    var held: CountDownLatch? = null

    override fun list(folder: String): PetaListing {
        answer()
        val page = PetaDropboxFile("$folder/docs/trip.md", contentHash(text), "first")
        return PetaListing(listOf(PetaDropboxEntry.File(page)), "c1", false)
    }

    override fun listAfter(cursor: String): PetaListing {
        answer()
        return PetaListing(emptyList(), "c1", false)
    }

    private fun answer() {
        held?.await()
        if (!reachable) throw PetaDropboxError.Offline("the network is gone")
        if (refuses) throw PetaDropboxError.Other("the cloud refused the sync")
    }

    override fun download(rev: String, to: String) = File(to).writeBytes(text)

    override fun upload(
        path: String,
        from: String,
        over: String?,
        modifiedMs: Long,
    ): PetaDropboxFile {
        val content = File(from).readBytes()
        uploaded[path.removePrefix("$FOLDER/")] = String(content)
        return PetaDropboxFile(path, contentHash(content), "next")
    }

    /**
     * The content hash of Dropbox, for a file of less than 4 MB: the hash of the hash of its
     * content. The sync compares it with the hash of the file in the app.
     */
    private fun contentHash(content: ByteArray): String {
        val block = sha256(content)
        return sha256(block).joinToString("") { "%02x".format(it) }
    }

    private fun sha256(content: ByteArray) = MessageDigest.getInstance("SHA-256").digest(content)
}
