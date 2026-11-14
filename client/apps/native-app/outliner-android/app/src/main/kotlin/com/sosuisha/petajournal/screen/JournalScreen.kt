package com.sosuisha.petajournal.screen

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import java.io.File
import com.sosuisha.petajournal.shell.Question
import com.sosuisha.petajournal.shell.Shell
import com.sosuisha.petajournal.sync.SyncState
import com.sosuisha.petajournal.types.ScreenView
import com.sosuisha.petajournal.types.SearchEntry
import com.sosuisha.petajournal.types.SearchKind
import com.sosuisha.petajournal.types.ViewModel

/** The name that the tests find the search field by. */
const val SEARCH_TAG = "search"

/** The field of the item that is edited. Only one item is edited at a time. */
const val EDITOR_TAG = "editor"

/** The tag of the bullet of an item, for the tests. */
const val BULLET_TAG = "bullet"

/**
 * The one screen of the app: a bar with the menu, the search field and the button for today,
 * and under it the results of the search or the page that is open. [cloud] is where the sync
 * with the cloud stands, and `null` in an app that syncs nothing. [syncInFront] says that the user waits for the sync.
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
    MaterialTheme {
        Surface(Modifier.fillMaxSize()) {
            // A new app before the sign-in: the bar would make pages before the first download
            if (shell.unopened && cloud == SyncState.SignedOut) {
                Column(Modifier.safeDrawingPadding()) {
                    Button(onSignIn, Modifier.padding(8.dp)) { Text("Sign in to Dropbox") }
                }
                return@Surface
            }
            Column(Modifier.safeDrawingPadding()) {
                // At the top, where it stays while an item is edited and the keyboard is up
                Row(Modifier.fillMaxWidth().padding(horizontal = 4.dp, vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
                    cloud?.let { SyncMenu(it, onSignIn, onSync, onSignOut) }
                    OutlinedTextField(
                        value = view.search.query,
                        onValueChange = shell::search,
                        singleLine = true,
                        placeholder = { Text("Search") },
                        modifier = Modifier.weight(1f).padding(horizontal = 4.dp).testTag(SEARCH_TAG),
                    )
                    TodayButton(shell::today)
                }
                // The room of the line is always there: the app syncs while the user types, and
                // the page must not jump
                Box(Modifier.fillMaxWidth().height(4.dp)) {
                    if (shell.syncing) LinearProgressIndicator(Modifier.fillMaxSize())
                }
                if (cloud == SyncState.SignedOut) {
                    Button(onSignIn, Modifier.padding(horizontal = 8.dp)) { Text("Sign in to Dropbox") }
                }
                val waiting = if (syncInFront) progress(cloud) else null
                waiting?.let { Waiting(it) }
                val searching = view.search.showDropdown
                // The rest of the height: the tools stay above the keyboard
                Box(Modifier.weight(1f)) {
                    Column {
                        if (searching) {
                            Results(view.search.results, shell::pick)
                        } else {
                            shell.question?.let { Asked(it, shell::answer) }
                            shell.missing?.let { MissingPage(it, shell::make) }
                            shell.missingFile?.let { Text("No file yet: $it", Modifier.padding(horizontal = 12.dp, vertical = 4.dp)) }
                            val links = Links(shell::follow) { path -> shell.attached(path)?.let(onFile) }
                            Outline(title(view), view.outline.rows, links, shell::image, shell)
                        }
                    }
                    // Over the page, so that it takes no room. Not while the dialog says the same
                    if (waiting == null) cloud?.let { SyncCallout(it, onSync, Modifier.align(Alignment.BottomCenter)) }
                }
                val edited = view.outline.rows.firstOrNull { it.isEditing }?.item?.id
                if (!searching && edited != null) Tools(edited, shell)
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
 * The button that opens the journal of today. Its face is the icon of the web app: a book
 * with a ribbon.
 */
@Composable
private fun TodayButton(onTouch: () -> Unit) {
    val ink = MaterialTheme.colorScheme.onSurfaceVariant
    Box(
        Modifier.size(48.dp).clickable(onClick = onTouch).semantics { contentDescription = "Today" },
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

@Composable
private fun Results(entries: List<SearchEntry>, onPick: (SearchEntry) -> Unit) {
    LazyColumn(Modifier.fillMaxSize()) {
        items(entries, key = { it.position.toLong() }) { entry ->
            Column(
                Modifier.fillMaxWidth().clickable { onPick(entry) }.padding(horizontal = 16.dp, vertical = 8.dp),
            ) {
                Text(label(entry), style = MaterialTheme.typography.titleMedium)
                if (entry.snippet.isNotEmpty()) {
                    Text(entry.snippet, style = MaterialTheme.typography.bodyMedium, maxLines = 2)
                }
            }
        }
    }
}

/** What a search result shows as its first line. Two kinds of result make a page. */
private fun label(entry: SearchEntry): String =
    when (entry.kind) {
        SearchKind.TODAY -> "Open Today"
        SearchKind.CREATE -> "New page: ${entry.title}"
        SearchKind.PAGE, SearchKind.ITEM -> entry.title
    }

/**
 * A question of the core, over the page. A touch outside the dialog answers no. The button
 * for yes names what a question about a loss does, and has the colour of an error.
 */
@Composable
private fun Asked(question: Question, onAnswer: (Boolean) -> Unit) {
    AlertDialog(
        onDismissRequest = { onAnswer(false) },
        text = { Text(question.text) },
        confirmButton = {
            if (question.destructive) {
                TextButton({ onAnswer(true) }) { Text("Delete", color = MaterialTheme.colorScheme.error) }
            } else {
                TextButton({ onAnswer(true) }) { Text("OK") }
            }
        },
        dismissButton = { TextButton({ onAnswer(false) }) { Text("Cancel") } },
    )
}

/**
 * A link named a page that is not there. The page is made only when the user asks for it
 * here (ADR 031).
 */
@Composable
private fun MissingPage(name: String, onMake: () -> Unit) {
    Row(Modifier.fillMaxWidth().padding(horizontal = 12.dp), verticalAlignment = Alignment.CenterVertically) {
        Text("No page yet: $name", Modifier.weight(1f))
        TextButton(onMake) { Text("Make it") }
    }
}
