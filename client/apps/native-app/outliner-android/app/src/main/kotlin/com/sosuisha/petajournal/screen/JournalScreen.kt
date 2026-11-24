package com.sosuisha.petajournal.screen

import android.content.ActivityNotFoundException
import android.widget.Toast
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.isImeVisible
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.sosuisha.petajournal.shell.Outside
import com.sosuisha.petajournal.shell.Question
import com.sosuisha.petajournal.shell.Shell
import com.sosuisha.petajournal.sync.SyncState
import com.sosuisha.petajournal.types.Row as OutlineRow
import com.sosuisha.petajournal.types.ScreenView
import com.sosuisha.petajournal.types.SearchEntry
import com.sosuisha.petajournal.types.ViewModel
import java.io.File

/** The name that the tests find the search field by. */
const val SEARCH_TAG = "search"

/** The field of the item that is edited. Only one item is edited at a time. */
const val EDITOR_TAG = "editor"

/** The tag of the bullet of an item, for the tests. */
const val BULLET_TAG = "bullet"

/** The tag of the bullet of an item whose children are hidden, for the tests. */
const val FOLDED_TAG = "folded"

/** The tag of the line of an item that is selected, for the tests. */
const val SELECTED_TAG = "selected"

/**
 * The one screen of the app: a bar with the menu, back and forward, the search field and the button
 * for today, and under it the results of the search or the page that is open. [cloud] is where the
 * sync with the cloud stands, and `null` in an app that syncs nothing. [syncInFront] says that the
 * user waits for the sync.
 */
@Composable
fun JournalScreen(
    shell: Shell,
    cloud: SyncState? = null,
    syncInFront: Boolean = false,
    onSignIn: () -> Unit = {},
    onSync: () -> Unit = {},
    onSignOut: () -> Unit = {},
    onFile: (File) -> Unit = {},
) {
    val view = shell.view
    val uriHandler = LocalUriHandler.current
    val context = LocalContext.current
    // What a link opens outside the app, which the core ordered (ADR 083)
    LaunchedEffect(shell.outside) {
        when (val outside = shell.outside) {
            // A link may have any scheme (#1071), and the device may have no app for it
            is Outside.Web ->
                try {
                    uriHandler.openUri(outside.address)
                } catch (none: IllegalArgumentException) {
                    Toast.makeText(context, "No app opens ${outside.address}", Toast.LENGTH_LONG)
                        .show()
                } catch (none: ActivityNotFoundException) {
                    Toast.makeText(context, "No app opens ${outside.address}", Toast.LENGTH_LONG)
                        .show()
                }
            is Outside.Document -> onFile(outside.file)
            null -> return@LaunchedEffect
        }
        shell.handedOn()
    }
    MaterialTheme {
        Surface(Modifier.fillMaxSize()) {
            // A new app before the sign-in: the bar would make pages before the first download
            if (shell.unopened && cloud == SyncState.SignedOut) {
                Column(Modifier.safeDrawingPadding()) {
                    Button(onSignIn, Modifier.padding(8.dp)) { Text("Sign in to Dropbox") }
                }
                return@Surface
            }
            CompositionLocalProvider(LocalCaret provides remember { Caret() }) {
                Column(Modifier.safeDrawingPadding()) {
                    // At the top, where it stays while an item is edited and the keyboard is up
                    Row(
                        Modifier.fillMaxWidth().padding(horizontal = 4.dp, vertical = 4.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        cloud?.let { SyncMenu(it, onSignIn, onSync, onSignOut) }
                        HistoryButton("Back", view.history.canGoBack, shell::back, left = true)
                        HistoryButton(
                            "Forward",
                            view.history.canGoForward,
                            shell::forward,
                            left = false,
                        )
                        OutlinedTextField(
                            value = view.search.query,
                            onValueChange = shell::search,
                            singleLine = true,
                            placeholder = { Text("Search") },
                            modifier =
                                Modifier.weight(1f).padding(horizontal = 4.dp).testTag(SEARCH_TAG),
                        )
                        TodayButton(shell::today)
                    }
                    // The room of the line is always there: the app syncs while the user types, and
                    // the page must not jump
                    Box(Modifier.fillMaxWidth().height(4.dp)) {
                        if (shell.syncing) LinearProgressIndicator(Modifier.fillMaxSize())
                    }
                    if (cloud == SyncState.SignedOut) {
                        Button(onSignIn, Modifier.padding(horizontal = 8.dp)) {
                            Text("Sign in to Dropbox")
                        }
                    }
                    val waiting = if (syncInFront) progress(cloud) else null
                    waiting?.let { Waiting(it) }
                    val searching = view.search.showDropdown
                    val rows = view.outline.rows
                    val edited = rows.firstOrNull { it.isEditing }
                    // The tools act on the item that is edited, or on the first that is selected
                    val target =
                        (edited ?: rows.firstOrNull { it.isSelected })?.takeUnless { searching }
                    // Full or shrunk, whatever is edited or selected. Full at first
                    var full by rememberSaveable { mutableStateOf(true) }
                    KeyboardWatch(edited, shell)
                    BackHandler(enabled = edited == null && rows.any { it.isSelected }) {
                        shell.unselect()
                    }
                    // The rest of the height: the tools stay above the keyboard
                    Box(Modifier.weight(1f)) {
                        Column {
                            if (searching) {
                                Results(view.search.results, shell::pick)
                            } else {
                                shell.question?.let { Asked(it, shell::answer) }
                                shell.missingFile?.let {
                                    Text(
                                        "No file yet: $it",
                                        Modifier.padding(horizontal = 12.dp, vertical = 4.dp),
                                    )
                                }
                                val links = Links(shell::follow)
                                Outline(title(view), view.outline.rows, links, shell::image, shell)
                            }
                        }
                        // Over the top of the page, so that it takes no room. Not at the bottom:
                        // the
                        // user types on the last lines most of the time, and it hid them. Not in
                        // the
                        // middle: it hid the line that is edited (#884). Not while the dialog says
                        // the same
                        if (waiting == null)
                            cloud?.let { SyncCallout(it, onSync, Modifier.align(Alignment.TopEnd)) }
                        if (target != null && !full)
                            ShrunkTools({ full = true }, Modifier.align(Alignment.BottomEnd))
                    }
                    if (target != null && full) Tools(target, rows, shell) { full = false }
                }
            }
        }
    }
}

/**
 * The keyboard went down while an item was edited: by the tool, or by the button of the system. The
 * edit ends, and the item stays selected, so that the tools still act on it. The app hears of the
 * system button only through the room the keyboard takes.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun KeyboardWatch(edited: OutlineRow?, shell: Shell) {
    val focus = LocalFocusManager.current
    val up = WindowInsets.isImeVisible
    val now by rememberUpdatedState(edited)
    // Set while the keyboard is up for an edit that has not ended. The keyboard of an edit
    // that ended is still going down when the next edit starts, and must not end that one
    var armed by remember { mutableStateOf(false) }
    LaunchedEffect(edited == null) { if (edited == null) armed = false }
    LaunchedEffect(up) {
        if (up) {
            armed = now != null
        } else if (armed) {
            armed = false
            now?.let {
                focus.clearFocus()
                shell.select(it.item.id)
            }
        }
    }
}

private fun title(view: ViewModel): String =
    when (val screen = view.screen) {
        is ScreenView.Page -> screen.value?.title.orEmpty()
        is ScreenView.Journal -> screen.value.displayDate
        else -> ""
    }

/**
 * The button that opens the journal of today. Its face is the icon of the web app: a book with a
 * ribbon.
 */
@Composable
private fun TodayButton(onTouch: () -> Unit) {
    val ink = MaterialTheme.colorScheme.onSurfaceVariant
    Box(
        Modifier.size(48.dp).clickable(onClick = onTouch).semantics {
            contentDescription = "Today"
        },
        contentAlignment = Alignment.Center,
    ) {
        Canvas(Modifier.size(24.dp)) {
            // The icon is drawn in a square of 24 units
            val unit = size.width / 24f
            fun at(x: Float, y: Float) = Offset(x * unit, y * unit)
            drawRoundRect(
                ink,
                topLeft = at(3f, 1f),
                size = Size(16f * unit, 20f * unit),
                cornerRadius = CornerRadius(1.5f * unit),
                style = Stroke(2f * unit),
            )
            drawLine(ink, at(3f, 13f), at(19f, 13f), strokeWidth = unit)
            drawLine(ink, at(3f, 17f), at(19f, 17f), strokeWidth = unit)
            drawLine(ink, at(14f, 2f), at(14f, 20f), strokeWidth = 3f * unit)
            drawRect(ink, topLeft = at(5f, 21f), size = Size(3f * unit, 2f * unit))
        }
    }
}

/**
 * The button that goes back or forward in the history of the pages. It is narrower than the others,
 * as two of them stand beside the search field. Without a page in its direction, it is there but
 * cannot be touched, so the bar does not move.
 */
@Composable
private fun HistoryButton(name: String, enabled: Boolean, onTouch: () -> Unit, left: Boolean) {
    val ink =
        MaterialTheme.colorScheme.onSurfaceVariant.let {
            if (enabled) it else it.copy(alpha = 0.38f)
        }
    Box(
        Modifier.size(width = 36.dp, height = 48.dp)
            .clickable(enabled = enabled, onClick = onTouch)
            .semantics { contentDescription = name },
        contentAlignment = Alignment.Center,
    ) {
        Canvas(Modifier.size(24.dp)) {
            val unit = size.width / 24f
            fun at(x: Float, y: Float) = Offset((if (left) x else 24f - x) * unit, y * unit)
            val width = 2.6f * unit
            drawLine(ink, at(15f, 5f), at(8f, 12f), strokeWidth = width, cap = StrokeCap.Round)
            drawLine(ink, at(8f, 12f), at(15f, 19f), strokeWidth = width, cap = StrokeCap.Round)
        }
    }
}

@Composable
private fun Results(entries: List<SearchEntry>, onPick: (SearchEntry) -> Unit) {
    LazyColumn(Modifier.fillMaxSize()) {
        items(entries, key = { it.position.toLong() }) { entry ->
            Column(
                Modifier.fillMaxWidth()
                    .clickable { onPick(entry) }
                    .padding(horizontal = 16.dp, vertical = 8.dp),
            ) {
                Text(entry.label, style = MaterialTheme.typography.titleMedium)
                if (entry.snippet.isNotEmpty()) {
                    Text(entry.snippet, style = MaterialTheme.typography.bodyMedium, maxLines = 2)
                }
            }
        }
    }
}

/**
 * A question of the core, over the page. A touch outside the dialog answers no. The button for yes
 * names what a question about a loss does, and has the colour of an error.
 */
@Composable
private fun Asked(question: Question, onAnswer: (Boolean) -> Unit) {
    AlertDialog(
        onDismissRequest = { onAnswer(false) },
        text = { Text(question.text) },
        confirmButton = {
            if (question.destructive) {
                TextButton({ onAnswer(true) }) {
                    Text("Delete", color = MaterialTheme.colorScheme.error)
                }
            } else {
                TextButton({ onAnswer(true) }) { Text("OK") }
            }
        },
        // A notice has nothing to cancel
        dismissButton = { if (question.waits) TextButton({ onAnswer(false) }) { Text("Cancel") } },
    )
}
