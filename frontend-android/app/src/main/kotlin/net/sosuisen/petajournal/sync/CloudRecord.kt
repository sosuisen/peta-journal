package net.sosuisen.petajournal.sync

import java.io.File
import java.util.Properties

/**
 * What the app knows of the files in the cloud: the version and the content hash of each, as
 * the last download or upload left it (ADR 031). A file here whose hash differs from it has
 * changed in the app. An upload names the version, and the cloud refuses when it has another.
 *
 * A path is looked up without regard to case, as the cloud does.
 *
 * @param file keeps the record between two runs of the app.
 */
class CloudRecord(private val file: File) {
    private val known: MutableMap<String, Change.File> by lazy(::read)

    private fun read(): MutableMap<String, Change.File> {
        if (!file.exists()) return mutableMapOf()
        val kept = Properties().apply { file.inputStream().use(::load) }
        return kept.stringPropertyNames().associateWithTo(mutableMapOf()) { key ->
            // The path is the last part: only it can hold the mark that parts them
            val (remote, contentHash, path) = kept.getProperty(key).split(PARTS, limit = 3)
            Change.File(path, contentHash, remote)
        }
    }

    /** The file as the cloud had it when the app last heard of it. `null` for a file it has not. */
    @Synchronized
    operator fun get(path: String): Change.File? = known[path.lowercase()]

    @Synchronized
    fun put(version: Change.File) {
        known[version.path.lowercase()] = version
    }

    /** Forgets the file, or the folder with all that is in it: it has gone from the cloud. */
    @Synchronized
    fun remove(path: String) {
        val key = path.lowercase()
        known.keys.removeAll { it == key || it.startsWith("$key/") }
    }

    /** Forgets every file whose path is not in [paths], which are in lower case. */
    @Synchronized
    fun keepOnly(paths: Set<String>) {
        known.keys.retainAll(paths)
    }

    /**
     * False for an app that has never written a record: a version of the app before the
     * upload downloaded its files, and nothing is known of their versions.
     */
    val isKept: Boolean get() = file.exists()

    /** Writes the record to its file. */
    @Synchronized
    fun save() {
        val kept = Properties()
        for ((key, version) in known) {
            kept.setProperty(key, listOf(version.remote, version.contentHash, version.path).joinToString(PARTS))
        }
        file.outputStream().use { kept.store(it, null) }
    }

    private companion object {
        const val PARTS = "|"
    }
}
