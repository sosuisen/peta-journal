package net.sosuisen.petajournal.sync

import java.io.File
import java.io.OutputStream
import java.security.MessageDigest

/**
 * The Markdown folder of the app: a copy of the folder in the cloud (ADR 030).
 *
 * The cloud does not tell `Notes.md` from `notes.md`, and the file system of a phone does. So
 * a path of the cloud is looked up here without regard to case, and a file never lies here
 * under two spellings.
 *
 * @param root the folder the journal core takes the files in from.
 * @param temp where a file is written before it is whole. It is not under [root]: the core
 *   must never take in half a file.
 */
class MarkdownFolder(private val root: File, private val temp: File) {
    private fun find(path: String): File? = findWithoutCase(root, path)

    /** Deletes the file, or the folder with all that is in it. Nothing there is fine. */
    fun remove(path: String) {
        find(path)?.deleteRecursively()
    }

    /** The hash that Dropbox makes of the content of the file. `null` when it is not here. */
    fun contentHash(path: String): String? = find(path)?.takeIf { it.isFile }?.let(::dropboxContentHash)

    /**
     * Puts a file at the path, with the spelling of the path. [write] gets the stream to write
     * its content into. The file appears whole or not at all.
     */
    fun put(path: String, write: (OutputStream) -> Unit) {
        temp.mkdirs()
        val part = File.createTempFile("download", ".part", temp)
        try {
            part.outputStream().use(write)
            place(part, path)
        } finally {
            part.delete()
        }
    }

    /** One file at a time: two downloads may need the same new folder. */
    @Synchronized
    private fun place(part: File, path: String) {
        val names = path.split('/')
        val dir = names.dropLast(1).fold(root) { dir, name ->
            child(dir, name) ?: File(dir, name).apply { mkdir() }
        }
        // The same file under another spelling, as after it was renamed in the cloud
        child(dir, names.last())?.delete()
        check(part.renameTo(File(dir, names.last()))) { "cannot put $path into the Markdown folder" }
    }

    /**
     * Deletes every file that the app copies and whose path is not in [paths], which are in
     * lower case. It is for a list of all that the cloud holds: what is not in it has gone.
     */
    fun keepOnly(paths: Set<String>) {
        root.walkBottomUp()
            .filter { it.isFile }
            .map { it to it.relativeTo(root).invariantSeparatorsPath.lowercase() }
            .filter { (_, path) -> keeps(path) && path !in paths }
            .forEach { (file, _) -> file.delete() }
    }

    companion object {
        const val MARKDOWN = ".md"

        /** The folder of the images of the pages, with the `/` after it. */
        const val ASSETS = "assets/"

        /**
         * True for a file that the app copies from the cloud: a page, or an image of a page.
         * [path] is in lower case. The cloud holds more, such as the settings of other apps.
         */
        fun keeps(path: String): Boolean = path.endsWith(MARKDOWN) || path.startsWith(ASSETS)
    }
}

/**
 * The file or folder at the path under [root], however its name is spelled there. The path of
 * the cloud, and the path that a page writes for its image, may differ from it in case.
 */
fun findWithoutCase(root: File, path: String): File? =
    path.split('/').fold<String, File?>(root) { dir, name -> dir?.let { child(it, name) } }

private fun child(dir: File, name: String): File? =
    dir.listFiles()?.firstOrNull { it.name.equals(name, ignoreCase = true) }

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
