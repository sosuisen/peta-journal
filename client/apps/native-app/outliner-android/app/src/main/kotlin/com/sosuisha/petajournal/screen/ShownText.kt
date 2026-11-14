package com.sosuisha.petajournal.screen

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.ParagraphStyle
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextLinkStyles
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.BaselineShift
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextIndent
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.em
import com.sosuisha.petajournal.types.Looks
import com.sosuisha.petajournal.types.RichBlock
import com.sosuisha.petajournal.types.RichLink
import com.sosuisha.petajournal.types.RichParagraph
import com.sosuisha.petajournal.types.RichText

/** One part of an item as it is shown, from top to bottom. */
sealed interface ShownBlock {
    /** Text with the decoration of its letters and its links. */
    data class Text(val text: AnnotatedString) : ShownBlock

    /** A block of code: its lines as they were written. */
    data class Code(val lines: String) : ShownBlock

    data class Table(val rows: List<TableRow>) : ShownBlock

    /**
     * An image of a page.
     *
     * @param path where its file is under the Markdown folder, such as `assets/picture.png`.
     * @param alt what stands for the image where it cannot be seen.
     * @param width the width that the page asks for, in pixels of a web page. `null` when it
     *   asks for none.
     * @param height the height that the page asks for, the same way.
     */
    data class Image(val path: String, val alt: String, val width: Int?, val height: Int?) : ShownBlock

    /** A list that holds a part of its own kind, such as a code block: its entries apart. */
    data class Entries(val entries: List<Entry>) : ShownBlock

    /** A quote that holds a part of its own kind. */
    data class Quote(val blocks: List<ShownBlock>) : ShownBlock
}

/** @param header true for the row of the titles of the columns. */
data class TableRow(val header: Boolean, val cells: List<AnnotatedString>)

/** @param mark what stands before the entry of a list: a bullet, its number or a checkbox. */
data class Entry(val mark: String, val blocks: List<ShownBlock>)

/**
 * An item as it is shown: with its decoration, not as Markdown. The core makes the parts of the
 * item and gives them with the row (ADR 042); this writes the text of each part as Compose
 * draws it. A `when` takes every kind of part, so a kind that the core makes later stops the
 * build until it is drawn here.
 *
 * [onLink] gets where a link that the user touched leads, as the core says.
 */
fun shownBlocks(rich: RichText, onLink: (RichLink) -> Unit): List<ShownBlock> = blocks(rich.blocks, onLink)

private fun blocks(blocks: List<RichBlock>, onLink: (RichLink) -> Unit): List<ShownBlock> =
    blocks.mapNotNull { block ->
        when (block) {
            is RichBlock.Text -> TextWriter(onLink).text(block.paragraphs).takeIf { it.isNotEmpty() }?.let(ShownBlock::Text)
            is RichBlock.Code -> ShownBlock.Code(block.lines)
            is RichBlock.Table -> ShownBlock.Table(
                block.rows.map { row -> TableRow(row.header, row.cells.map { TextWriter(onLink).text(listOf(it)) }) },
            )
            is RichBlock.Image -> with(block.image) { ShownBlock.Image(path, alt, width?.toInt(), height?.toInt()) }
            is RichBlock.Entries -> ShownBlock.Entries(block.entries.map { Entry(it.mark, blocks(it.blocks, onLink)) })
            is RichBlock.Quote -> ShownBlock.Quote(blocks(block.blocks, onLink))
        }
    }

/**
 * Writes paragraphs into one text. Each is a paragraph of the text: a list moves its entries to
 * the right, and the first line of an entry starts with its mark. This is the look of this
 * shell: it is not kept the same as the stylesheet of the web frontend.
 */
private class TextWriter(private val onLink: (RichLink) -> Unit) {
    private val out = AnnotatedString.Builder()

    fun text(paragraphs: List<RichParagraph>): AnnotatedString {
        paragraphs.forEach(::paragraph)
        val text = out.toAnnotatedString()
        // Every paragraph ends its line, the last one too
        return text.subSequence(0, text.text.trimEnd().length)
    }

    private fun paragraph(paragraph: RichParagraph) {
        val quoted = paragraph.quoted.toInt()
        val indent = (paragraph.depth.toInt() + quoted) * 1.5
        out.pushStyle(ParagraphStyle(textIndent = TextIndent(firstLine = indent.em, restLine = indent.em)))
        if (quoted > 0) out.pushStyle(SpanStyle(color = QUOTED))
        val heading = paragraph.heading.toInt()
        if (heading > 0) out.pushStyle(SpanStyle(fontWeight = FontWeight.Bold, fontSize = headingSize(heading).em))
        paragraph.mark?.let { out.append("$it ") }
        if (paragraph.rule) out.append("────────")
        for (run in paragraph.runs) {
            val link = run.link
            if (link != null) {
                out.pushLink(LinkAnnotation.Clickable(LINK_TAG, TextLinkStyles(SpanStyle(color = LINK))) { onLink(link) })
            }
            out.pushStyle(style(run.looks))
            out.append(run.text)
            out.pop()
            if (link != null) out.pop()
        }
        out.append("\n")
        if (heading > 0) out.pop()
        if (quoted > 0) out.pop()
        out.pop()
    }
}

/** How the letters of a run look. */
private fun style(looks: Looks): SpanStyle {
    val decorations = listOfNotNull(
        TextDecoration.LineThrough.takeIf { looks.struck },
        TextDecoration.Underline.takeIf { looks.underlined },
    )
    val small = looks.raised || looks.lowered
    return SpanStyle(
        fontWeight = if (looks.bold) FontWeight.Bold else null,
        fontStyle = if (looks.italic) FontStyle.Italic else null,
        fontFamily = if (looks.code) FontFamily.Monospace else null,
        textDecoration = if (decorations.isEmpty()) null else TextDecoration.combine(decorations),
        background = when {
            looks.hidden -> HIDDEN
            looks.marked -> MARKED
            else -> Color.Unspecified
        },
        baselineShift = when {
            looks.raised -> BaselineShift.Superscript
            looks.lowered -> BaselineShift.Subscript
            else -> null
        },
        fontSize = if (small) 0.75.em else TextUnit.Unspecified,
    )
}

/** The tag of the annotation of a link, which a test looks for. */
const val LINK_TAG = "link"

private val LINK = Color(0xFF0A58CA)
private val MARKED = Color(0xFFFFF3A3)
private val QUOTED = Color(0xFF555555)
private val HIDDEN = Color(0xFFDDDDDD)

/** How much larger than the text a heading of this level is, as a web browser draws it. */
private fun headingSize(level: Int): Double = listOf(2.0, 1.5, 1.17, 1.0, 0.83, 0.67)[level.coerceIn(1, 6) - 1]
