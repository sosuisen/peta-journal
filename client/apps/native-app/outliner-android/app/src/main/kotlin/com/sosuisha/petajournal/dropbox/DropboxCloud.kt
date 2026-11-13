package com.sosuisha.petajournal.dropbox

import com.dropbox.core.v2.DbxClientV2
import com.sosuisha.petajournal.sync.Change
import com.sosuisha.petajournal.sync.ChangedInCloud
import com.sosuisha.petajournal.sync.Changes
import com.sosuisha.petajournal.sync.Cloud
import java.io.File
import java.io.OutputStream
import java.util.Date

/**
 * The Markdown folder in Dropbox (ADR 030, ADR 031).
 *
 * @param folder the path of the folder in Dropbox, such as `/PetaJournal`.
 */
class DropboxCloud internal constructor(private val files: DropboxFiles, folder: String) : Cloud {
    /** The folder, read and written with the Dropbox SDK. */
    constructor(client: DbxClientV2, folder: String) : this(SdkDropboxFiles(client), folder)

    private val folder = folder.trimEnd('/')

    override fun changes(cursor: String?): Changes {
        if (cursor != null) {
            try {
                return read(files.listAfter(cursor), whole = false)
            } catch (forgotten: CursorForgotten) {
                // Dropbox wants the list made anew
            }
        }
        return read(files.list(folder), whole = true)
    }

    /** Dropbox answers in parts. */
    private fun read(first: Listing, whole: Boolean): Changes {
        var part = first
        val entries = mutableListOf<Change>()
        while (true) {
            entries += part.entries.mapNotNull(::change)
            if (!part.hasMore) return Changes(entries, part.cursor, whole)
            part = files.listAfter(part.cursor)
        }
    }

    /** Nothing for a folder that is there: its files tell of it. */
    private fun change(entry: Entry): Change? {
        // The path as it is shown keeps the case of the name, which names the page
        val path = entry.path?.drop(folder.length)?.trimStart('/')
        if (path.isNullOrEmpty()) return null
        return when (entry) {
            is Entry.File -> version(path, entry)
            is Entry.Deleted -> Change.Gone(path)
            is Entry.Folder -> null
        }
    }

    private fun version(path: String, file: Entry.File) =
        Change.File(path, file.contentHash.orEmpty(), remote = "$REV${file.rev}")

    override fun download(file: Change.File, into: OutputStream) = files.download(file.remote.removePrefix(REV), into)

    override fun upload(path: String, from: File, over: String?): Change.File {
        // The Dropbox app of the desk gives its copy this time, as the file has it here.
        // Dropbox keeps no parts of a second
        val modified = Date(from.lastModified() / 1000 * 1000)
        try {
            return version(path, files.upload("$folder/$path", from, over?.removePrefix(REV), modified))
        } catch (refused: UploadConflict) {
            throw ChangedInCloud(path)
        }
    }

    override fun contentHash(file: File): String = dropboxContentHash(file)

    private companion object {
        /** Before the version of a file in [Change.File.remote]. */
        const val REV = "rev:"
    }
}
