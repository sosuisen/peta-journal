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

    data class Uploading(val done: Int, val total: Int) : SyncState

    /** The last sync failed. What it had downloaded and uploaded stays. */
    data class Failed(val reason: String) : SyncState
}

/**
 * Runs the syncs with the cloud, one at a time, and tells the screen where they stand. It is
 * used on the main thread; a sync runs on a thread of its own.
 *
 * @param round makes one sync with the cloud: download, work of the core, upload.
 * @param onMainThread runs what it is given on the main thread, later and in order.
 */
class CloudSync(
    signedIn: Boolean,
    private val round: (Cloud) -> SyncRound,
    private val onMainThread: (() -> Unit) -> Unit,
) {
    var state: SyncState by mutableStateOf(if (signedIn) SyncState.Idle else SyncState.SignedOut)
        private set

    private val running: Boolean get() = state is SyncState.Downloading || state is SyncState.Uploading

    /**
     * The user has signed out of the cloud. What was downloaded stays. It is not for the time
     * a sync runs: the screen offers no sign-out then.
     */
    fun signOut() {
        check(!running) { "a sync runs" }
        state = SyncState.SignedOut
    }

    /** The cloud of a sync that was asked for while another ran. */
    private var next: Cloud? = null

    /**
     * Whether the sync that runs is one the user waits for: the app came to the front, and
     * the pages may be old until the sync rests. The screen then shows where it stands in a
     * dialog. A sync after an edit is not in front: the user goes on typing.
     */
    var inFront: Boolean by mutableStateOf(false)
        private set

    /**
     * Starts a sync with the cloud. While one runs, another starts after it, once: what was
     * typed since the running one read the pages is not left behind. A sync that runs comes
     * to the front when [inFront] asks for it.
     */
    fun start(cloud: Cloud, inFront: Boolean = false) {
        if (inFront) this.inFront = true
        if (running) {
            next = cloud
            return
        }
        next = null
        state = SyncState.Downloading(0, 0)
        thread(name = "cloud-sync") {
            try {
                round(cloud).run(
                    onDownload = { done, total -> onMainThread { state = SyncState.Downloading(done, total) } },
                    // Most syncs have nothing to upload, and say nothing of it
                    onUpload = { done, total -> if (total > 0) onMainThread { state = SyncState.Uploading(done, total) } },
                )
                onMainThread {
                    state = SyncState.Idle
                    val waits = next
                    // The sync that waited goes on in front, when this one was
                    if (waits == null) this.inFront = false else start(waits)
                }
            } catch (failed: Exception) {
                onMainThread {
                    this.inFront = false
                    state = SyncState.Failed(failed.message ?: failed.javaClass.simpleName)
                }
            }
        }
    }
}
