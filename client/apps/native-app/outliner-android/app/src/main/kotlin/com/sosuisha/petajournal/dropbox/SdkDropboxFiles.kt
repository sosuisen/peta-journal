package com.sosuisha.petajournal.dropbox

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
 * Dropbox itself, through the Dropbox SDK. It only passes each call on, and turns the types
 * of the SDK into the types of [DropboxFiles]. No test runs it: all that can be tested
 * without Dropbox is in [DropboxCloud].
 */
internal class SdkDropboxFiles(private val client: DbxClientV2) : DropboxFiles {
    override fun list(folder: String): Listing =
        listing(client.files().listFolderBuilder(folder).withRecursive(true).start())

    override fun listAfter(cursor: String): Listing =
        try {
            listing(client.files().listFolderContinue(cursor))
        } catch (lost: ListFolderContinueErrorException) {
            if (lost.errorValue.isReset) throw CursorForgotten()
            throw lost
        }

    private fun listing(result: ListFolderResult) = Listing(result.entries.map(::entry), result.cursor, result.hasMore)

    private fun entry(metadata: Metadata): Entry =
        when (metadata) {
            is FileMetadata -> file(metadata)
            is DeletedMetadata -> Entry.Deleted(metadata.pathDisplay)
            else -> Entry.Folder(metadata.pathDisplay)
        }

    private fun file(metadata: FileMetadata) = Entry.File(metadata.pathDisplay, metadata.contentHash, metadata.rev)

    override fun download(rev: String, into: OutputStream) {
        // Dropbox takes a version in place of a path, after this word
        client.files().download("rev:$rev").download(into)
    }

    override fun upload(path: String, from: File, over: String?, modified: Date): Entry.File {
        // Dropbox takes the content only over the version it is told, or where no file is
        val mode = if (over == null) WriteMode.ADD else WriteMode.update(over)
        val upload = client.files().uploadBuilder(path)
            .withMode(mode)
            .withAutorename(false)
            .withClientModified(modified)
        try {
            return file(from.inputStream().use(upload::uploadAndFinish))
        } catch (refused: UploadErrorException) {
            val error = refused.errorValue
            if (error.isPath && error.pathValue.reason.isConflict) throw UploadConflict()
            throw refused
        }
    }
}
