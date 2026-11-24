package com.sosuisha.petajournal.dropbox

import com.dropbox.core.v2.DbxClientV2
import com.dropbox.core.v2.files.DeletedMetadata
import com.dropbox.core.v2.files.FileMetadata
import com.dropbox.core.v2.files.ListFolderContinueErrorException
import com.dropbox.core.v2.files.ListFolderResult
import com.dropbox.core.v2.files.Metadata
import com.dropbox.core.v2.files.UploadErrorException
import com.dropbox.core.v2.files.WriteMode
import com.sosuisha.petajournal.core.PetaDropboxEntry
import com.sosuisha.petajournal.core.PetaDropboxError
import com.sosuisha.petajournal.core.PetaDropboxFile
import com.sosuisha.petajournal.core.PetaDropboxSdk
import com.sosuisha.petajournal.core.PetaListing
import java.io.File
import java.util.Date

/**
 * Dropbox itself, through the Dropbox SDK, for the sync of the core (ADR 054). It only passes each
 * call on, and turns the types and the failures of the SDK into those of the core: it throws only a
 * [PetaDropboxError]. No test runs it: the rules of the sync are tested in Rust, and the reading of
 * the failures in `DropboxFailuresTest`.
 */
internal class SdkDropbox(private val client: DbxClientV2) : PetaDropboxSdk {
    override fun list(folder: String): PetaListing = failing {
        listing(client.files().listFolderBuilder(folder).withRecursive(true).start())
    }

    override fun listAfter(cursor: String): PetaListing = failing {
        try {
            listing(client.files().listFolderContinue(cursor))
        } catch (lost: ListFolderContinueErrorException) {
            if (lost.errorValue.isReset) throw PetaDropboxError.CursorForgotten
            throw lost
        }
    }

    private fun listing(result: ListFolderResult) =
        PetaListing(result.entries.map(::entry), result.cursor, result.hasMore)

    private fun entry(metadata: Metadata): PetaDropboxEntry =
        when (metadata) {
            is FileMetadata -> PetaDropboxEntry.File(file(metadata))
            is DeletedMetadata -> PetaDropboxEntry.Deleted(metadata.pathDisplay)
            else -> PetaDropboxEntry.Folder(metadata.pathDisplay)
        }

    private fun file(metadata: FileMetadata) =
        PetaDropboxFile(metadata.pathDisplay, metadata.contentHash, metadata.rev)

    override fun download(rev: String, to: String) = failing {
        // Dropbox takes a version in place of a path, after this word
        File(to).outputStream().use { client.files().download("rev:$rev").download(it) }
        Unit
    }

    override fun upload(
        path: String,
        from: String,
        over: String?,
        modifiedMs: Long,
    ): PetaDropboxFile {
        // Dropbox takes the content only over the version it is told, or where no file is
        val mode = if (over == null) WriteMode.ADD else WriteMode.update(over)
        val upload =
            client
                .files()
                .uploadBuilder(path)
                .withMode(mode)
                .withAutorename(false)
                .withClientModified(Date(modifiedMs))
        return failing {
            try {
                file(File(from).inputStream().use(upload::uploadAndFinish))
            } catch (refused: UploadErrorException) {
                val error = refused.errorValue
                if (error.isPath && error.pathValue.reason.isConflict)
                    throw PetaDropboxError.UploadConflict
                throw refused
            }
        }
    }

    /** Runs a call. What the SDK throws becomes a failure that the core knows. */
    private fun <Answer> failing(call: () -> Answer): Answer =
        try {
            call()
        } catch (failed: Exception) {
            throw dropboxFailure(failed)
        }
}
