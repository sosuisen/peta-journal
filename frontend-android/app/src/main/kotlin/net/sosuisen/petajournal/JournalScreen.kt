package net.sosuisen.petajournal

import android.net.Uri
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TriStateCheckbox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.state.ToggleableState
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import net.sosuisen.petajournal.sync.SyncState
import net.sosuisen.petajournal.types.Row
import net.sosuisen.petajournal.types.ScreenView
import net.sosuisen.petajournal.types.SearchEntry
import net.sosuisen.petajournal.types.SearchKind
import net.sosuisen.petajournal.types.TaskState
import net.sosuisen.petajournal.types.ViewModel

/** The name that the tests find the search field by. */
const val SEARCH_TAG = "search"

/**
 * The one screen of the app: a search field, and under it the results of the search or the
 * page that is open. Between them is where the sync with the cloud stands; [cloud] is `null`
 * in an app that syncs nothing.
 */
@Composable
fun JournalScreen(shell: Shell, cloud: SyncState? = null, onSignIn: () -> Unit = {}, onSync: () -> Unit = {}) {
    val view = shell.view
    MaterialTheme {
        Surface(Modifier.fillMaxSize()) {
            Column(Modifier.safeDrawingPadding()) {
                OutlinedTextField(
                    value = view.search.query,
                    onValueChange = shell::search,
                    singleLine = true,
                    placeholder = { Text("Search") },
                    modifier = Modifier.fillMaxWidth().padding(8.dp).testTag(SEARCH_TAG),
                )
                cloud?.let { SyncLine(it, onSignIn, onSync) }
                if (shell.syncing) {
                    LinearProgressIndicator(Modifier.fillMaxWidth())
                }
                if (view.search.showDropdown) {
                    Results(found(view), shell::pick)
                } else {
                    shell.missing?.let { Text("No page yet: $it", Modifier.padding(horizontal = 12.dp, vertical = 4.dp)) }
                    Outline(title(view), view.outline.rows, view.config.tagsDir, shell::follow)
                }
            }
        }
    }
}

/**
 * The results that open a page. The first version changes nothing, so it leaves out the
 * entries that make a page: the journal of today, and a page with the name that was typed.
 */
private fun found(view: ViewModel): List<SearchEntry> =
    view.search.results.filter { it.kind == SearchKind.PAGE || it.kind == SearchKind.ITEM }

private fun title(view: ViewModel): String =
    when (val screen = view.screen) {
        is ScreenView.Page -> screen.value?.title.orEmpty()
        is ScreenView.Journal -> screen.value.displayDate
        else -> ""
    }

@Composable
private fun SyncLine(state: SyncState, onSignIn: () -> Unit, onSync: () -> Unit) {
    Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp), verticalAlignment = Alignment.CenterVertically) {
        when (state) {
            SyncState.SignedOut -> Button(onSignIn) { Text("Sign in to Dropbox") }
            SyncState.Idle -> TextButton(onSync) { Text("Sync") }
            is SyncState.Downloading -> Text("Downloading ${state.done} of ${state.total}", Modifier.padding(8.dp))
            is SyncState.Failed -> {
                Text("Sync failed: ${state.reason}", Modifier.weight(1f).padding(8.dp))
                TextButton(onSync) { Text("Try again") }
                // A credential that Dropbox no longer takes is replaced by a new sign-in
                TextButton(onSignIn) { Text("Sign in") }
            }
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
                Text(entry.title, style = MaterialTheme.typography.titleMedium)
                if (entry.snippet.isNotEmpty()) {
                    Text(entry.snippet, style = MaterialTheme.typography.bodyMedium, maxLines = 2)
                }
            }
        }
    }
}

@Composable
private fun Outline(title: String, rows: List<Row>, tagsDir: String, onPage: (String) -> Unit) {
    LazyColumn(Modifier.fillMaxSize().padding(horizontal = 12.dp)) {
        item(key = "title") {
            Text(title, style = MaterialTheme.typography.headlineSmall, modifier = Modifier.padding(vertical = 8.dp))
        }
        items(rows, key = { it.item.id }) { row -> OutlineRow(row, tagsDir, onPage) }
    }
}

@Composable
private fun OutlineRow(row: Row, tagsDir: String, onPage: (String) -> Unit) {
    val uriHandler = LocalUriHandler.current
    val blocks = remember(row.item.ast, row.item.body, tagsDir) {
        shownBlocks(row.item, tagsDir) { href ->
            when {
                isWebAddress(href) -> uriHandler.openUri(href)
                // The core takes the URL of a page as it is written, not encoded
                href.startsWith(Shell.PAGES) -> onPage(Uri.decode(href))
                // Any other link, such as one to a file, opens nothing
            }
        }
    }
    Column {
        row.dateHeader?.let { date ->
            Text(date, style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(top = 12.dp))
        }
        Row(Modifier.padding(start = row.indentPx.dp, top = 3.dp, bottom = 3.dp)) {
            Text(if (row.showBullet) "•" else "", modifier = Modifier.width(22.dp))
            row.task?.let { TaskMark(it) }
            Column {
                for (block in blocks) {
                    when (block) {
                        is ShownBlock.Text -> Text(block.text, style = MaterialTheme.typography.bodyLarge)
                        is ShownBlock.Code -> CodeBlock(block)
                        is ShownBlock.Table -> TableBlock(block)
                    }
                }
            }
        }
    }
}

/**
 * Where a task stands, as a checkbox. It only shows: the first version changes nothing, so a
 * touch does not move the task on.
 */
@Composable
private fun TaskMark(state: TaskState) {
    val (box, name) = when (state) {
        TaskState.NOTSTARTED -> ToggleableState.Off to "Task not started"
        TaskState.RUNNING -> ToggleableState.Indeterminate to "Task running"
        TaskState.DONE -> ToggleableState.On to "Task done"
    }
    TriStateCheckbox(
        state = box,
        onClick = null,
        modifier = Modifier.padding(end = 6.dp, top = 2.dp).semantics { contentDescription = name },
    )
}

/** The lines of the code as they were written. A long line is scrolled sideways, not broken. */
@Composable
private fun CodeBlock(code: ShownBlock.Code) {
    Text(
        code.lines,
        style = MaterialTheme.typography.bodyMedium,
        fontFamily = FontFamily.Monospace,
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp)
            .background(MaterialTheme.colorScheme.surfaceVariant)
            .horizontalScroll(rememberScrollState())
            .padding(8.dp),
    )
}

/** The cells of a table, each in a box, in columns of the same width. */
@Composable
private fun TableBlock(table: ShownBlock.Table) {
    val line = MaterialTheme.colorScheme.outlineVariant
    Column(Modifier.padding(vertical = 4.dp)) {
        for (row in table.rows) {
            Row(Modifier.height(IntrinsicSize.Min)) {
                for (cell in row.cells) {
                    Text(
                        cell,
                        style = MaterialTheme.typography.bodyMedium,
                        fontWeight = if (row.header) FontWeight.Bold else null,
                        modifier = Modifier.weight(1f).fillMaxHeight().border(0.5.dp, line).padding(6.dp),
                    )
                }
            }
        }
    }
}

private fun isWebAddress(href: String): Boolean = href.startsWith("https://") || href.startsWith("http://")
