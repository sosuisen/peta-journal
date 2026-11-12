package net.sosuisen.petajournal.sync

import com.dropbox.core.v2.DbxClientV2
import com.dropbox.core.v2.files.DeletedMetadata
import com.dropbox.core.v2.files.FileMetadata
import com.dropbox.core.v2.files.ListFolderContinueErrorException
import com.dropbox.core.v2.files.ListFolderResult
import com.dropbox.core.v2.files.Metadata
import com.dropbox.core.v2.files.UploadErrorException
import com.dropbox.core.v2.files.WriteMode
import java.io.File
import java.io.OutputStream
import java.util.Date

/**
 * The Markdown folder in Dropbox, read and written with the Dropbox SDK (ADR 030, ADR 031).
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
            is FileMetadata -> version(path, entry)
            is DeletedMetadata -> Change.Gone(path)
            else -> null
        }
    }

    private fun version(path: String, file: FileMetadata) =
        Change.File(path, file.contentHash.orEmpty(), remote = "$REV${file.rev}")

    override fun download(file: Change.File, into: OutputStream) {
        client.files().download(file.remote).download(into)
    }

    override fun upload(path: String, from: File, over: String?): Change.File {
        // Dropbox takes the content only over the version it is told, or where no file is
        val mode = if (over == null) WriteMode.ADD else WriteMode.update(over.removePrefix(REV))
        val upload = client.files().uploadBuilder("$folder/$path")
            .withMode(mode)
            .withAutorename(false)
            // The Dropbox app of the desk gives its copy this time, as the file has it here
            .withClientModified(Date(from.lastModified() / 1000 * 1000))
        try {
            return version(path, from.inputStream().use(upload::uploadAndFinish))
        } catch (refused: UploadErrorException) {
            val error = refused.errorValue
            if (error.isPath && error.pathValue.reason.isConflict) throw ChangedInCloud(path)
            throw refused
        }
    }

    private companion object {
        /** Before the version of a file, where Dropbox takes a version in place of a path. */
        const val REV = "rev:"
    }
}
