package net.sosuisen.petajournal.screen

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import java.io.File

/** The parts of an item, or of an entry of a list or a quote in it, from top to bottom. */
@Composable
internal fun Blocks(blocks: List<ShownBlock>, image: (String) -> File?) {
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
