package com.sosuisha.petajournal.sync

import java.io.File
import java.io.IOException
import java.io.OutputStream
import java.security.MessageDigest

/** A cloud of a test: the versions of its files, and the changes it tells of at each cursor. */
class FakeCloud : Cloud {
    /** The content of each version of a file, by how the cloud finds the version. */
    private val contents = mutableMapOf<String, ByteArray>()

    /** The newest version of each file, by its path in lower case. */
    private val newest = mutableMapOf<String, Change.File>()

    /** What `changes` answers, by the cursor it is asked with. */
    val answers = mutableMapOf<String?, Changes>()
    val askedWith = mutableListOf<String?>()
    val downloaded = mutableListOf<String>()
    val uploaded = mutableListOf<String>()
    var broken: String? = null

    /** Gives the file of the cloud a new version, and answers with the change that tells of it. */
    fun file(path: String, text: String): Change.File {
        val content = text.toByteArray()
        val version = Change.File(path, hashOf(content), remote = "$path#${contents.size}")
        contents[version.remote] = content
        newest[path.lowercase()] = version
        return version
    }

    /** Takes the file, or the folder with all that is in it, out of the cloud. */
    fun gone(path: String): Change.Gone {
        val key = path.lowercase()
        newest.keys.removeAll { it == key || it.startsWith("$key/") }
        return Change.Gone(path)
    }

    /** The text of the newest version of the file, or `null` when the cloud has no such file. */
    fun text(path: String): String? = newest[path.lowercase()]?.let { String(contents.getValue(it.remote)) }

    override fun changes(cursor: String?): Changes {
        askedWith += cursor
        return answers.getValue(cursor)
    }

    override fun download(file: Change.File, into: OutputStream) {
        if (file.path == broken) throw IOException("the network is gone")
        synchronized(downloaded) { downloaded += file.path }
        into.write(contents.getValue(file.remote))
    }

    override fun upload(path: String, from: File, over: String?): Change.File {
        if (path == broken) throw IOException("the network is gone")
        if (newest[path.lowercase()]?.remote != over) throw ChangedInCloud(path)
        uploaded += path
        return file(path, from.readText())
    }

    override fun contentHash(file: File): String = hashOf(file.readBytes())
}

fun sha256(bytes: ByteArray): ByteArray = MessageDigest.getInstance("SHA-256").digest(bytes)

fun hex(bytes: ByteArray): String = bytes.joinToString("") { "%02x".format(it) }

/** The content hash of the cloud of a test. It is not the one of Dropbox: the sync asks the cloud. */
fun hashOf(content: ByteArray): String = hex(sha256(content))
