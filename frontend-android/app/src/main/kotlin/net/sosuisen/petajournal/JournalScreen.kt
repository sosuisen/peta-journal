package net.sosuisen.petajournal

import android.graphics.BitmapFactory
import android.net.Uri
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.rememberTransformableState
import androidx.compose.foundation.gestures.transformable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Spacer
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Size
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalWindowInfo
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File
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
fun JournalScreen(
    shell: Shell,
    cloud: SyncState? = null,
    onSignIn: () -> Unit = {},
    onSync: () -> Unit = {},
    onSignOut: () -> Unit = {},
    onFile: (File) -> Unit = {},
) {
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
                cloud?.let { SyncLine(it, onSignIn, onSync, onSignOut) }
                if (shell.syncing) {
                    LinearProgressIndicator(Modifier.fillMaxWidth())
                }
                if (view.search.showDropdown) {
                    Results(found(view), shell::pick)
                } else {
                    shell.missing?.let { Text("No page yet: $it", Modifier.padding(horizontal = 12.dp, vertical = 4.dp)) }
                    shell.missingFile?.let { Text("No file yet: $it", Modifier.padding(horizontal = 12.dp, vertical = 4.dp)) }
                    val links = Links(shell::follow) { path -> shell.attached(path)?.let(onFile) }
                    Outline(title(view), view.outline.rows, view.config.tagsDir, links, shell::image)
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
private fun SyncLine(state: SyncState, onSignIn: () -> Unit, onSync: () -> Unit, onSignOut: () -> Unit) {
    Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp), verticalAlignment = Alignment.CenterVertically) {
        when (state) {
            SyncState.SignedOut -> Button(onSignIn) { Text("Sign in to Dropbox") }
            SyncState.Idle -> {
                TextButton(onSync) { Text("Sync") }
                Spacer(Modifier.weight(1f))
                TextButton(onSignOut) { Text("Sign out") }
            }
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

/**
 * Where the links of an item lead.
 *
 * @param onPage gets the URL of a page, such as `/pages/notes`.
 * @param onFile gets the path of a file of the assets folder, such as `assets/report.pdf`.
 */
private class Links(val onPage: (String) -> Unit, val onFile: (String) -> Unit)

@Composable
private fun Outline(title: String, rows: List<Row>, tagsDir: String, links: Links, image: (String) -> File?) {
    LazyColumn(Modifier.fillMaxSize().padding(horizontal = 12.dp)) {
        item(key = "title") {
            Text(title, style = MaterialTheme.typography.headlineSmall, modifier = Modifier.padding(vertical = 8.dp))
        }
        items(rows, key = { it.item.id }) { row -> OutlineRow(row, tagsDir, links, image) }
    }
}

@Composable
private fun OutlineRow(row: Row, tagsDir: String, links: Links, image: (String) -> File?) {
    val uriHandler = LocalUriHandler.current
    // The blocks are kept while the item stays the same, and must not keep old links
    val now by rememberUpdatedState(links)
    val blocks = remember(row.item.ast, row.item.body, tagsDir) {
        shownBlocks(row.item, tagsDir) { href ->
            val file = assetPath(href)
            when {
                isWebAddress(href) -> uriHandler.openUri(href)
                // The core takes the URL of a page as it is written, not encoded
                href.startsWith(Shell.PAGES) -> now.onPage(Uri.decode(href))
                file != null -> now.onFile(file)
                // Any other link, such as one to a file of the PC, opens nothing
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
            Blocks(blocks, image)
        }
    }
}

/** The parts of an item, or of an entry of a list or a quote in it, from top to bottom. */
@Composable
private fun Blocks(blocks: List<ShownBlock>, image: (String) -> File?) {
    Column {
        for (block in blocks) {
            when (block) {
                is ShownBlock.Text -> Text(block.text, style = MaterialTheme.typography.bodyLarge)
                is ShownBlock.Code -> CodeBlock(block)
                is ShownBlock.Table -> TableBlock(block)
                is ShownBlock.Image -> ImageBlock(block, image)
                is ShownBlock.Entries -> EntriesBlock(block, image)
                is ShownBlock.Quote -> QuoteBlock(block, image)
            }
        }
    }
}

/** The entries of a list, each after its mark. */
@Composable
private fun EntriesBlock(list: ShownBlock.Entries, image: (String) -> File?) {
    Column {
        for (entry in list.entries) {
            Row {
                Text(entry.mark, style = MaterialTheme.typography.bodyLarge, modifier = Modifier.padding(end = 8.dp))
                Blocks(entry.blocks, image)
            }
        }
    }
}

/** What is quoted, with a line down its left side. */
@Composable
private fun QuoteBlock(quote: ShownBlock.Quote, image: (String) -> File?) {
    val line = MaterialTheme.colorScheme.outline
    Box(
        Modifier
            .padding(vertical = 4.dp)
            .drawBehind { drawRect(line, size = Size(3.dp.toPx(), size.height)) }
            .padding(start = 12.dp),
    ) {
        Blocks(quote.blocks, image)
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

/** What is known of the file of an image. */
private sealed interface Picture {
    /** Not read yet. Nothing is drawn, so that the alt does not flash before the image. */
    data object Reading : Picture

    data object Missing : Picture

    /** @param width the width of the image in its file, which [bitmap] may be smaller than. */
    data class Here(val bitmap: ImageBitmap, val width: Int) : Picture
}

/**
 * An image, as wide as the page asks, or else as wide as it is in its file. A pixel of the
 * page is one `dp`, as in a browser. It is never wider than the room it has. An image whose
 * file is not here, or cannot be read, shows what stands for it.
 */
@Composable
private fun ImageBlock(image: ShownBlock.Image, file: (String) -> File?) {
    val roomPx = LocalWindowInfo.current.containerSize.width
    val picture by produceState<Picture>(Picture.Reading, image.path) {
        value = withContext(Dispatchers.IO) { file(image.path)?.let { read(it, roomPx) } ?: Picture.Missing }
    }
    when (val known = picture) {
        Picture.Reading -> Unit
        Picture.Missing -> Text(image.alt.ifEmpty { image.path }, style = MaterialTheme.typography.bodyLarge)
        is Picture.Here -> {
            var large by remember { mutableStateOf(false) }
            val shape = known.bitmap.width.toFloat() / known.bitmap.height
            // The side that the page does not ask for follows the shape of the image
            val width = image.width?.toFloat() ?: image.height?.let { it * shape } ?: known.width.toFloat()
            val height = image.height?.toFloat() ?: (width / shape)
            Image(
                known.bitmap,
                contentDescription = image.alt,
                // A page that asks for both sides may ask for another shape, as in a browser
                contentScale = ContentScale.FillBounds,
                modifier = Modifier
                    .padding(vertical = 4.dp)
                    .width(width.dp)
                    .aspectRatio(width / height)
                    .clickable { large = true },
            )
            if (large) LargeImage(image, known.bitmap, file) { large = false }
        }
    }
}

/** How many times the width of the screen an image is read for its large view, at least. */
private const val LARGE_WIDTHS = 2

/** How far two fingers make the large view larger. */
private const val MOST_ZOOM = 8f

/**
 * An image over the whole screen, to read what is small in it. Two fingers make it larger and
 * move it. A touch, or the back key, closes it.
 *
 * The image is read again with more of its pixels. Until then, [small] is shown.
 */
@Composable
private fun LargeImage(image: ShownBlock.Image, small: ImageBitmap, file: (String) -> File?, onClose: () -> Unit) {
    val roomPx = LocalWindowInfo.current.containerSize.width
    val sharp by produceState<ImageBitmap?>(null, image.path) {
        value = withContext(Dispatchers.IO) { file(image.path)?.let { read(it, roomPx * LARGE_WIDTHS) }?.bitmap }
    }
    var zoom by remember { mutableFloatStateOf(1f) }
    var moved by remember { mutableStateOf(Offset.Zero) }
    var room by remember { mutableStateOf(IntSize.Zero) }
    val fingers = rememberTransformableState { zoomBy, panBy, _ ->
        zoom = (zoom * zoomBy).coerceIn(1f, MOST_ZOOM)
        // The image never leaves the screen: it moves only as far as it is larger
        val farX = room.width * (zoom - 1) / 2
        val farY = room.height * (zoom - 1) / 2
        moved = Offset((moved.x + panBy.x).coerceIn(-farX, farX), (moved.y + panBy.y).coerceIn(-farY, farY))
    }
    Dialog(onClose, DialogProperties(usePlatformDefaultWidth = false)) {
        Image(
            sharp ?: small,
            contentDescription = image.alt,
            contentScale = ContentScale.Fit,
            modifier = Modifier
                .fillMaxSize()
                .background(Color.Black)
                .onSizeChanged { room = it }
                .clickable(interactionSource = null, indication = null, onClick = onClose)
                .transformable(fingers)
                .graphicsLayer {
                    scaleX = zoom
                    scaleY = zoom
                    translationX = moved.x
                    translationY = moved.y
                },
        )
    }
}

/**
 * Reads the image of the file. A large image is read smaller, by whole halves, down to no
 * less than [roomPx] wide: a photo at its full size takes the memory of many screens.
 */
private fun read(file: File, roomPx: Int): Picture.Here? {
    val size = BitmapFactory.Options().apply { inJustDecodeBounds = true }
    BitmapFactory.decodeFile(file.path, size)
    val halves = generateSequence(1) { it * 2 }.takeWhile { size.outWidth / it >= roomPx }.lastOrNull() ?: 1
    val bitmap = BitmapFactory.decodeFile(file.path, BitmapFactory.Options().apply { inSampleSize = halves })
    return bitmap?.let { Picture.Here(it.asImageBitmap(), size.outWidth) }
}
