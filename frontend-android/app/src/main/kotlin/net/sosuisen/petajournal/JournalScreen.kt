package net.sosuisen.petajournal

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import net.sosuisen.petajournal.types.Row
import net.sosuisen.petajournal.types.ScreenView
import net.sosuisen.petajournal.types.SearchEntry
import net.sosuisen.petajournal.types.SearchKind
import net.sosuisen.petajournal.types.ViewModel

/** The name that the tests find the search field by. */
const val SEARCH_TAG = "search"

/**
 * The one screen of the app: a search field, and under it the results of the search or the
 * page that is open.
 */
@Composable
fun JournalScreen(shell: Shell) {
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
                if (shell.syncing) {
                    LinearProgressIndicator(Modifier.fillMaxWidth())
                }
                if (view.search.showDropdown) {
                    Results(found(view), shell::pick)
                } else {
                    Outline(title(view), view.outline.rows)
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
private fun Outline(title: String, rows: List<Row>) {
    LazyColumn(Modifier.fillMaxSize().padding(horizontal = 12.dp)) {
        item(key = "title") {
            Text(title, style = MaterialTheme.typography.headlineSmall, modifier = Modifier.padding(vertical = 8.dp))
        }
        items(rows, key = { it.item.id }) { row -> OutlineRow(row) }
    }
}

@Composable
private fun OutlineRow(row: Row) {
    val uriHandler = LocalUriHandler.current
    val text = remember(row.item.ast, row.item.body) {
        // A link to a page or a tag has no address, and opens nothing yet
        shownText(row.item) { href -> if (isWebAddress(href)) uriHandler.openUri(href) }
    }
    Column {
        row.dateHeader?.let { date ->
            Text(date, style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(top = 12.dp))
        }
        Row(Modifier.padding(start = row.indentPx.dp, top = 3.dp, bottom = 3.dp)) {
            Text(if (row.showBullet) "•" else "", modifier = Modifier.width(22.dp))
            Text(text, style = MaterialTheme.typography.bodyLarge)
        }
    }
}

private fun isWebAddress(href: String): Boolean = href.startsWith("https://") || href.startsWith("http://")
