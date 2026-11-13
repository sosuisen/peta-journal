package net.sosuisen.petajournal.dropbox

import java.io.File
import java.io.IOException
import java.io.OutputStream
import java.util.Date

/**
 * The calls of Dropbox that [DropboxCloud] makes, without the types of the Dropbox SDK. A
 * test puts its own Dropbox here. The app puts [SdkDropboxFiles] here.
 */
internal interface DropboxFiles {
    /** The first part of all that is in the folder at the path, and in the folders in it. */
    fun list(folder: String): Listing

    /**
     * The part after [cursor]: the next part of a list, or what changed since a list ended.
     *
     * @throws CursorForgotten when Dropbox no longer knows the cursor.
     */
    fun listAfter(cursor: String): Listing

    /** Writes the content of the version [rev] of a file into [into]. */
    fun download(rev: String, into: OutputStream)

    /**
     * Puts the content of [from] at the path. [over] is the version that this content
     * replaces, and it is `null` for a file that Dropbox has not. Dropbox gives the file no
     * other name. [modified] is the time of the last change, as the file has it here.
     *
     * @throws UploadConflict when Dropbox has another version, or a file where none was expected.
     */
    fun upload(path: String, from: File, over: String?, modified: Date): Entry.File
}

/** One part of a list. [cursor] names the state after it. [hasMore] says that a part follows. */
internal class Listing(val entries: List<Entry>, val cursor: String, val hasMore: Boolean)

/** What Dropbox tells of one path. [path] is the path as it is shown, with the case of the names. */
internal sealed interface Entry {
    val path: String?

    /** [contentHash] is `null` when Dropbox gave none. */
    data class File(override val path: String?, val contentHash: String?, val rev: String) : Entry

    data class Deleted(override val path: String?) : Entry

    data class Folder(override val path: String?) : Entry
}

/** Dropbox forgot a cursor that was too old. It wants the list made anew. */
internal class CursorForgotten : IOException("Dropbox forgot the cursor")

/** Dropbox did not take an upload: the file there is not the version that was to be replaced. */
internal class UploadConflict : IOException("the file changed in Dropbox")
