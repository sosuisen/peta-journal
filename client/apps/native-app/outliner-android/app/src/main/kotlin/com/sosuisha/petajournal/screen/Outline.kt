package com.sosuisha.petajournal.screen

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TriStateCheckbox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.state.ToggleableState
import androidx.compose.ui.unit.dp
import com.sosuisha.petajournal.shell.Shell
import com.sosuisha.petajournal.types.RichLink
import com.sosuisha.petajournal.types.Row
import com.sosuisha.petajournal.types.TaskState
import java.io.File
import kotlin.math.roundToInt

/** Where the links of an item go: the core decides how each opens (ADR 083). */
internal class Links(val onLink: (RichLink) -> Unit)

@Composable
internal fun Outline(
    title: String,
    rows: List<Row>,
    links: Links,
    image: (String) -> File?,
    shell: Shell,
) {
    val list = rememberLazyListState()
    val focus = shell.view.outline.focusRequest
    // The item that the core moved the edit to may be off the screen, as a new item at the
    // end of a long page. The title is the first entry of the list
    LaunchedEffect(focus?.seq) {
        val at = rows.indexOfFirst { it.item.id == focus?.itemId }
        if (at >= 0 && list.layoutInfo.visibleItemsInfo.none { it.index == at + 1 })
            list.scrollToItem(at + 1)
    }
    LazyColumn(Modifier.fillMaxSize().padding(horizontal = 12.dp), state = list) {
        item(key = "title") {
            Text(
                title,
                style = MaterialTheme.typography.headlineSmall,
                modifier = Modifier.padding(vertical = 8.dp),
            )
        }
        items(rows, key = { it.item.id }) { row -> OutlineRow(row, links, image, shell) }
    }
}

@Composable
private fun OutlineRow(row: Row, links: Links, image: (String) -> File?, shell: Shell) {
    // The blocks are kept while the item stays the same, and must not keep old links
    val now by rememberUpdatedState(links)
    val blocks = remember(row.bodyModel) { shownBlocks(row.bodyModel) { link -> now.onLink(link) } }
    Column {
        row.dateHeader?.let { date ->
            // As large as the date of today at the top: each is the title of a day
            Text(
                date,
                style = MaterialTheme.typography.headlineSmall,
                modifier = Modifier.padding(top = 12.dp),
            )
        }
        // A selected item is coloured behind, so that the user sees what the tools act on
        val selected =
            if (row.isSelected) {
                Modifier.background(MaterialTheme.colorScheme.primaryContainer)
                    .testTag(SELECTED_TAG)
            } else {
                Modifier
            }
        Row(selected.fillMaxWidth().padding(start = row.indentPx.dp, top = 3.dp, bottom = 3.dp)) {
            // The bullet and the mark stand on the line of the first text, and go up to the
            // middle of its letters when they are large. An item that starts with something
            // else, such as a table, has no such line: they stay at the top
            val first = blocks.firstOrNull()
            val onLine =
                row.isEditing ||
                    first is ShownBlock.Text ||
                    first is ShownBlock.Entries ||
                    first is ShownBlock.Quote
            fun Modifier.onLine(): Modifier = if (onLine) alignByBaseline() else this
            // The Markdown of an item that is edited has the size of plain text
            val lift = if (row.isEditing) 0 else liftOfFirstLine(blocks)
            val middleOfLetter =
                with(LocalDensity.current) {
                    (MaterialTheme.typography.bodyLarge.fontSize.toPx() * MIDDLE_OF_A_LETTER)
                        .roundToInt()
                }
            // The bullet is drawn, not a letter: the place of a letter in its line differs
            // from font to font
            val beside =
                if (onLine) Modifier.alignBy { it.measuredHeight / 2 + middleOfLetter + lift }
                else Modifier.padding(top = 9.dp)
            Box(beside.width(22.dp).height(BULLET)) {
                if (row.showBullet) {
                    // A folded item has a large faint disc behind its bullet, as on the web
                    val folded =
                        if (row.isCollapsed) {
                            val disc = LocalContentColor.current.copy(alpha = FOLDED_ALPHA)
                            Modifier.drawBehind { drawCircle(disc, radius = FOLDED_RADIUS.toPx()) }
                                .testTag(FOLDED_TAG)
                        } else {
                            Modifier.testTag(BULLET_TAG)
                        }
                    Box(folded.size(BULLET).background(LocalContentColor.current, CircleShape))
                }
            }
            row.task?.let { state ->
                // The middle of the mark is at the middle of a letter of plain text
                val middle =
                    if (onLine) Modifier.alignBy { it.measuredHeight / 2 + middleOfLetter + lift }
                    else Modifier
                TaskMark(state, middle) { shell.cycleTask(row.item.id) }
            }
            if (row.isEditing) {
                ItemEditor(row, shell, Modifier.weight(1f).onLine())
            } else {
                // A touch on a link or an image opens it, and any other touch edits the item with
                // the cursor where it was touched
                val touches = remember { Touches() }
                val id = row.item.id
                Box(
                    Modifier.weight(1f)
                        .onLine()
                        .onGloballyPositioned { touches.item = it }
                        .pointerInput(id, shell) {
                            detectTapGestures { shell.edit(id, touches.letter(it)) }
                        }
                        // TalkBack has no place in the text to touch: the edit goes on at the end
                        .semantics {
                            onClick {
                                shell.edit(id)
                                true
                            }
                        },
                ) {
                    CompositionLocalProvider(LocalTouches provides touches) {
                        Blocks(blocks, image)
                    }
                }
            }
        }
    }
}

/**
 * How much of the size of a letter the middle of its height is above the line it stands on. A
 * Japanese letter reaches from about 0.12 under the line to 0.88 above it.
 */
private const val MIDDLE_OF_A_LETTER = 0.38f

/** How wide and how high the bullet of an item is. */
private val BULLET = 7.dp

/**
 * The disc behind the bullet of a folded item. The web draws a dot of 5px on a disc of 13px, in
 * `#888` and `#ddd`: the disc here keeps that size to the bullet, and that faintness.
 */
private val FOLDED_RADIUS = 9.dp
private const val FOLDED_ALPHA = 0.13f

/**
 * How far above the line of the first text the bullet and the mark of a task stand, in pixels, to
 * be beside the middle of its letters. A heading has larger letters than plain text, so their
 * middle is higher above the line.
 */
@Composable
private fun liftOfFirstLine(blocks: List<ShownBlock>): Int {
    val first = (blocks.firstOrNull() as? ShownBlock.Text)?.text ?: return 0
    val larger =
        first.spanStyles
            .firstOrNull { it.start == 0 && it.item.fontSize.isEm }
            ?.item
            ?.fontSize
            ?.value ?: return 0
    val plain = with(LocalDensity.current) { MaterialTheme.typography.bodyLarge.fontSize.toPx() }
    return (plain * (larger - 1f) * MIDDLE_OF_A_LETTER).roundToInt()
}

/**
 * Where a task stands, as a checkbox. A touch moves the task on: not started, running, done, and
 * then no task.
 */
@Composable
private fun TaskMark(state: TaskState, modifier: Modifier, onTouch: () -> Unit) {
    val (box, name) =
        when (state) {
            TaskState.NOTSTARTED -> ToggleableState.Off to "Task not started"
            TaskState.RUNNING -> ToggleableState.Indeterminate to "Task running"
            TaskState.DONE -> ToggleableState.On to "Task done"
        }
    // As high as a line of the text, so the box is beside the first line. A box that takes a
    // touch by itself is 48dp high, and would stand below the line
    val line = with(LocalDensity.current) { MaterialTheme.typography.bodyLarge.lineHeight.toDp() }
    Box(
        modifier
            .padding(end = 6.dp)
            .size(width = 28.dp, height = line)
            .clickable(onClick = onTouch)
            .semantics { contentDescription = name },
        contentAlignment = Alignment.Center,
    ) {
        TriStateCheckbox(state = box, onClick = null)
    }
}
