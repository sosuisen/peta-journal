package net.sosuisen.petajournal.sync

import com.dropbox.core.v2.DbxClientV2
import com.dropbox.core.v2.files.DeletedMetadata
import com.dropbox.core.v2.files.FileMetadata
import com.dropbox.core.v2.files.ListFolderContinueErrorException
import com.dropbox.core.v2.files.ListFolderResult
import com.dropbox.core.v2.files.Metadata
import java.io.OutputStream

/**
 * The Markdown folder in Dropbox, read with the Dropbox SDK (ADR 030).
 *
 * @param folder the path of the folder in Dropbox, such as `/PetaJournal`.
 */
class DropboxCloud(private val client: DbxClientV2, folder: String) : Cloud {
    private val folder = folder.trimEnd('/')

    override fun changes(cursor: String?): Changes {
        if (cursor != null) {
            try {
                return read(client.files().listFolderContinue(cursor), whole = false)
            } catch (lost: ListFolderContinueErrorException) {
                // Dropbox forgets a cursor that is too old, and wants the list made anew
                if (!lost.errorValue.isReset) throw lost
            }
        }
        return read(client.files().listFolderBuilder(folder).withRecursive(true).start(), whole = true)
    }

    /** Dropbox answers in parts. */
    private fun read(first: ListFolderResult, whole: Boolean): Changes {
        var result = first
        val entries = mutableListOf<Change>()
        while (true) {
            entries += result.entries.mapNotNull(::change)
            if (!result.hasMore) return Changes(entries, result.cursor, whole)
            result = client.files().listFolderContinue(result.cursor)
        }
    }

    /** Nothing for a folder that is there: its files tell of it. */
    private fun change(entry: Metadata): Change? {
        // The path as it is shown keeps the case of the name, which names the page
        val path = entry.pathDisplay?.drop(folder.length)?.trimStart('/')
        if (path.isNullOrEmpty()) return null
        return when (entry) {
            is FileMetadata -> Change.File(path, entry.contentHash.orEmpty(), remote = "rev:${entry.rev}")
            is DeletedMetadata -> Change.Gone(path)
            else -> null
        }
    }

    override fun download(file: Change.File, into: OutputStream) {
        client.files().download(file.remote).download(into)
    }
}
