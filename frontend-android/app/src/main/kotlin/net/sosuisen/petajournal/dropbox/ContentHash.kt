package net.sosuisen.petajournal.dropbox

import java.io.File
import java.security.MessageDigest

private const val BLOCK = 4 * 1024 * 1024

/**
 * The content hash of Dropbox: the SHA-256 of the SHA-256 of each block of 4MB, as hex. It
 * tells whether a file here has the content of the file in the cloud, with no download.
 */
fun dropboxContentHash(file: File): String {
    val whole = MessageDigest.getInstance("SHA-256")
    val block = ByteArray(BLOCK)
    file.inputStream().use { input ->
        while (true) {
            val read = input.readNBytes(block, 0, BLOCK)
            if (read == 0) break
            whole.update(MessageDigest.getInstance("SHA-256").apply { update(block, 0, read) }.digest())
        }
    }
    return whole.digest().joinToString("") { "%02x".format(it) }
}
