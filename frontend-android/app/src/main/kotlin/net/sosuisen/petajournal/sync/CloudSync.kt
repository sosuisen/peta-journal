package net.sosuisen.petajournal.sync

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlin.concurrent.thread

/** Where the sync with the cloud stands, for the screen. */
sealed interface SyncState {
    /** The user has not signed in to the cloud yet. */
    data object SignedOut : SyncState

    /** Nothing runs. */
    data object Idle : SyncState

    data class Downloading(val done: Int, val total: Int) : SyncState

    /** The last download failed. What it had downloaded stays. */
    data class Failed(val reason: String) : SyncState
}

/**
 * Runs the downloads from the cloud, one at a time, and tells the screen where they stand. It
 * is used on the main thread; a download runs on a thread of its own.
 *
 * @param onMainThread runs what it is given on the main thread, later and in order.
 * @param onDownloaded is called on the main thread after a download that ended well: the
 *   files of the Markdown folder are then to be taken in.
 */
class CloudSync(
    signedIn: Boolean,
    private val downloader: (Cloud) -> Downloader,
    private val onMainThread: (() -> Unit) -> Unit,
    private val onDownloaded: () -> Unit,
) {
    var state: SyncState by mutableStateOf(if (signedIn) SyncState.Idle else SyncState.SignedOut)
        private set

    /** Starts a download from the cloud. Nothing happens while one runs. */
    fun start(cloud: Cloud) {
        if (state is SyncState.Downloading) return
        state = SyncState.Downloading(0, 0)
        thread(name = "cloud-sync") {
            try {
                downloader(cloud).run { done, total -> onMainThread { state = SyncState.Downloading(done, total) } }
                onMainThread {
                    state = SyncState.Idle
                    onDownloaded()
                }
            } catch (failed: Exception) {
                onMainThread { state = SyncState.Failed(failed.message ?: failed.javaClass.simpleName) }
            }
        }
    }
}
