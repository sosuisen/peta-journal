package net.sosuisen.petajournal.sync

import java.io.File
import java.io.IOException
import java.io.OutputStream

/**
 * What the sync needs of the cloud storage: the Markdown folder there, as Dropbox tells of
 * it. A path is relative to that folder, with `/` between its parts and none at its start.
 * The cloud does not tell names apart by upper and lower case.
 */
interface Cloud {
    /**
     * What changed in the folder since [cursor], in the order it happened. Without a cursor,
     * or when the cloud no longer knows the cursor, it is everything that is in the folder.
     */
    fun changes(cursor: String?): Changes

    /** Writes the content of the file into [into]. */
    fun download(file: Change.File, into: OutputStream)

    /**
     * Puts the content of [from] at the path. [over] names the version that this content
     * replaces, as [Change.File.remote] does, and it is `null` for a file that the cloud has
     * not. Answers with the file as the cloud has it now.
     *
     * @throws ChangedInCloud when the cloud has another version than [over], or a file where
     *   none was expected. Nothing is written then.
     */
    fun upload(path: String, from: File, over: String?): Change.File
}

/** The cloud has a version of the file that the app has not seen yet. */
class ChangedInCloud(val path: String) : IOException("$path changed in the cloud")

/**
 * @param cursor names the state after the entries, to ask for the changes after it.
 * @param whole true when the entries are all that the folder holds, and not changes: a file
 *   that is not among them is not in the cloud.
 */
class Changes(val entries: List<Change>, val cursor: String, val whole: Boolean)

sealed interface Change {
    val path: String

    /**
     * A file that is new or has new content.
     *
     * @param contentHash the hash that Dropbox makes of the content.
     * @param remote how the cloud finds this very version of the file.
     */
    data class File(override val path: String, val contentHash: String, val remote: String) : Change

    /** A file, or a folder with all that was in it, that is no longer there. */
    data class Gone(override val path: String) : Change
}
