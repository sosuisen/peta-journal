package com.sosuisha.petajournal.screen

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.RoundRect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Outline
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import androidx.compose.ui.window.PopupProperties
import com.sosuisha.petajournal.sync.SyncState

/**
 * The menu at the left of the bar, with what is done to the sync. There is none before the sign-in:
 * the button for the sign-in is on the screen then.
 */
@Composable
internal fun SyncMenu(
    state: SyncState,
    onSignIn: () -> Unit,
    onSync: () -> Unit,
    onSignOut: () -> Unit,
) {
    if (state == SyncState.SignedOut) return
    var open by remember { mutableStateOf(false) }
    // Nothing is asked of a sync that runs
    val resting = state is SyncState.Idle || state is SyncState.Failed
    fun entry(does: () -> Unit): () -> Unit = {
        open = false
        does()
    }
    Box {
        TextButton({ open = true }, Modifier.semantics { contentDescription = "Menu" }) {
            Text("☰", style = MaterialTheme.typography.titleLarge)
        }
        // The menu takes no focus: an item that is edited keeps it, and the keyboard stays
        DropdownMenu(open, { open = false }, properties = PopupProperties(focusable = false)) {
            DropdownMenuItem({ Text("Sync") }, entry(onSync), enabled = resting)
            // A credential that Dropbox no longer takes is replaced by a new sign-in
            if (state is SyncState.Failed)
                DropdownMenuItem({ Text("Sign in to Dropbox") }, entry(onSignIn))
            DropdownMenuItem({ Text("Sign out") }, entry(onSignOut), enabled = resting)
        }
    }
}

/** How far a sync that moves files is, in words and as a part of the whole, or null. */
internal fun progress(state: SyncState?): Pair<String, Float>? =
    when (state) {
        is SyncState.Downloading ->
            if (state.total > 0)
                "Downloading ${state.done} of ${state.total}" to state.done.toFloat() / state.total
            else null
        is SyncState.Uploading ->
            if (state.total > 0)
                "Uploading ${state.done} of ${state.total}" to state.done.toFloat() / state.total
            else null
        else -> null
    }

/**
 * Where the sync stands, over the page, while the user waits for it: the app came to the front, and
 * the pages may be old until the files have moved. The dialog cannot be closed: an edit of an old
 * page would be merged with the new one later.
 */
@Composable
internal fun Waiting(progress: Pair<String, Float>) {
    Dialog({}, DialogProperties(dismissOnBackPress = false, dismissOnClickOutside = false)) {
        Surface(shape = MaterialTheme.shapes.large, tonalElevation = 6.dp) {
            Column(Modifier.padding(24.dp)) {
                Text(progress.first, Modifier.padding(bottom = 16.dp))
                LinearProgressIndicator({ progress.second }, Modifier.fillMaxWidth())
            }
        }
    }
}

/**
 * Where a sync stands that the user does not wait for, such as the one after an edit, and why the
 * last one failed. It lies over the top of the page, and takes no room there.
 */
@Composable
internal fun SyncCallout(state: SyncState, onSync: () -> Unit, modifier: Modifier) {
    val (words, again) =
        when (state) {
            is SyncState.Downloading,
            is SyncState.Uploading -> (progress(state)?.first ?: "Syncing…") to false
            is SyncState.Failed ->
                (if (state.offline) "Sync failed: no connection" else "Sync failed") to true
            SyncState.SignedOut,
            SyncState.Idle -> return
        }
    Surface(
        modifier.padding(12.dp),
        shape = Callout,
        color = MaterialTheme.colorScheme.inverseSurface,
        contentColor = MaterialTheme.colorScheme.inverseOnSurface,
        shadowElevation = 6.dp,
    ) {
        Row(
            Modifier.padding(start = 16.dp, end = 16.dp + TAIL),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(words, Modifier.padding(vertical = 12.dp).weight(1f, fill = false))
            if (again) {
                TextButton(onSync) {
                    Text("Try again", color = MaterialTheme.colorScheme.inversePrimary)
                }
            }
        }
    }
}

/** How far the tail of the callout of the sync reaches out of its right side. */
private val TAIL = 8.dp

/** A speech balloon with its tail on the right side, as the callout of the sync. */
private object Callout : Shape {
    override fun createOutline(
        size: Size,
        layoutDirection: LayoutDirection,
        density: Density,
    ): Outline {
        val tail = with(density) { TAIL.toPx() }
        val corner = with(density) { 4.dp.toPx() }
        val side = size.width - tail
        val middle = size.height / 2
        val balloon =
            Path().apply {
                addRoundRect(RoundRect(0f, 0f, side, size.height, CornerRadius(corner)))
                moveTo(side, middle - tail)
                lineTo(size.width, middle)
                lineTo(side, middle + tail)
                close()
            }
        return Outline.Generic(balloon)
    }
}
