package net.sosuisen.petajournal.screen

import java.io.File
import java.io.IOException
import java.io.OutputStream
import java.security.MessageDigest
import java.util.concurrent.CountDownLatch
import net.sosuisen.petajournal.sync.Change
import net.sosuisen.petajournal.sync.Changes
import net.sosuisen.petajournal.sync.Cloud

/** A cloud with one page, or one that cannot be reached. It keeps what is uploaded. */
internal class OnePage(var reachable: Boolean = true) : Cloud {
    private val text = "京都で会議がありました。\n".toByteArray()

    /** The text of each uploaded file, by its path. */
    val uploaded = mutableMapOf<String, String>()

    /** Holds the cloud back in its answer, until the test lets it go on. */
    var held: CountDownLatch? = null

    override fun changes(cursor: String?): Changes {
        held?.await()
        if (!reachable) throw IOException("the network is gone")
        val whole = cursor == null
        val page = Change.File("docs/trip.md", hashOf(text), remote = "first")
        return Changes(if (whole) listOf(page) else emptyList(), "c1", whole)
    }

    override fun download(file: Change.File, into: OutputStream) = into.write(text)

    override fun upload(path: String, from: File, over: String?): Change.File {
        uploaded[path] = from.readText()
        return Change.File(path, contentHash(from), remote = "next")
    }

    override fun contentHash(file: File): String = hashOf(file.readBytes())

    /** The content hash of this cloud. It is not the one of Dropbox: the sync asks the cloud. */
    private fun hashOf(content: ByteArray): String =
        MessageDigest.getInstance("SHA-256").digest(content).joinToString("") { "%02x".format(it) }
}
