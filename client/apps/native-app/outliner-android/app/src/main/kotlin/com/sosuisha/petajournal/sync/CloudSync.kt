package com.sosuisha.petajournal.sync

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.sosuisha.petajournal.core.PetaCloudSync
import com.sosuisha.petajournal.core.PetaDropboxSdk
import com.sosuisha.petajournal.core.PetaSyncHost
import com.sosuisha.petajournal.core.PetaSyncState
import com.sosuisha.petajournal.shell.Shell
import java.io.File

/** Where the sync with the cloud stands, for the screen. */
sealed interface SyncState {
    /** The user has not signed in to the cloud yet. */
    data object SignedOut : SyncState

    /** Nothing runs. */
    data object Idle : SyncState

    data class Downloading(val done: Int, val total: Int) : SyncState

    data class Uploading(val done: Int, val total: Int) : SyncState

    /**
     * The last sync failed. What it had downloaded and uploaded stays. [offline] says that the
     * cloud could not be reached. The screen tells only that: the words of the SDK are long, and
     * mean nothing to the user. They go to the log.
     */
    data class Failed(val offline: Boolean) : SyncState
}

/**
 * The syncs with the cloud, for the screen. The core runs them, one at a time, on a thread of its
 * own (ADR 054); this keeps where they stand on the main thread. It is used on the main thread. The
 * downloads go to the Markdown folder of the core of [shell].
 *
 * @param temp where a download is written before it is whole.
 * @param cursor keeps where the last download ended.
 * @param record keeps the versions of the files in the cloud that the app has seen.
 * @param shell takes in the downloaded files, and its core tells when a page changed.
 * @param afterChangeMs how long the sync waits after the last change of a page before it starts
 *   (ADR 055).
 * @param onMainThread runs what it is given on the main thread, later and in order.
 * @param log keeps why a sync failed, for the one who looks into it.
 */
class CloudSync(
    shell: Shell,
    afterChangeMs: Long,
    temp: File,
    cursor: File,
    record: File,
    signedIn: Boolean,
    onMainThread: (() -> Unit) -> Unit,
    log: (String) -> Unit = {},
) {
    private val sync =
        PetaCloudSync(
            shell.core,
            afterChangeMs.toULong(),
            temp.path,
            cursor.path,
            record.path,
            signedIn,
            Host(shell::syncAndWait, { onMainThread(::read) }, log),
        )

    init {
        // A word that the keyboard composes is not saved yet, and the sync waits for it
        shell.onComposing = sync::typing
    }

    var state: SyncState by mutableStateOf(screenState(sync.state()))
        private set

    /**
     * Whether the sync that runs is one the user waits for: the app came to the front, and the
     * pages may be old until the sync rests. The screen then shows where it stands in a dialog. A
     * sync after an edit is not in front: the user goes on typing.
     */
    var inFront: Boolean by mutableStateOf(false)
        private set

    /**
     * Starts a sync with the Markdown folder at the path [folder] of the cloud. While one runs,
     * another starts after it, once. [inFront] says that the user waits for it. [sdk] throws only a
     * `PetaDropboxError`: the core makes any other exception a failure that is not offline.
     */
    fun start(sdk: PetaDropboxSdk, folder: String, inFront: Boolean = false) {
        sync.start(sdk, folder, inFront)
        // The core tells of it a moment later, on the main thread. The screen must not offer
        // the sign-out meanwhile: the core does not take it while a sync runs
        read()
    }

    /**
     * The user has signed out. What was downloaded stays. The core forgets where the last download
     * ended: the next sign-in may be to another account. Not for the time a sync runs: the screen
     * offers no sign-out then.
     */
    fun signOut() {
        sync.signOut()
        read()
    }

    /**
     * Reads where the sync stands from the core. What the core told on its thread may be old by the
     * time it reaches the main thread: a sync may have started since.
     */
    private fun read() {
        state = screenState(sync.state())
        inFront = sync.inFront()
    }
}

private fun screenState(state: PetaSyncState): SyncState =
    when (state) {
        PetaSyncState.SignedOut -> SyncState.SignedOut
        PetaSyncState.Idle -> SyncState.Idle
        is PetaSyncState.Downloading ->
            SyncState.Downloading(state.done.toInt(), state.total.toInt())
        is PetaSyncState.Uploading -> SyncState.Uploading(state.done.toInt(), state.total.toInt())
        is PetaSyncState.Failed -> SyncState.Failed(state.offline)
    }

/** What the sync of the core asks of the app. It is called on the threads of the sync. */
private class Host(
    private val takeIn: () -> Unit,
    private val moved: () -> Unit,
    private val log: (String) -> Unit,
) : PetaSyncHost {
    override fun takeIn() = takeIn.invoke()

    override fun told(state: PetaSyncState, inFront: Boolean) = moved()

    override fun log(failure: String) = log.invoke(failure)
}
