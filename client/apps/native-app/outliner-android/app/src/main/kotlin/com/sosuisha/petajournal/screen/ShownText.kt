package com.sosuisha.petajournal.screen

import android.net.Uri
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
import androidx.compose.ui.unit.em
import com.sosuisha.petajournal.core.pageUrl
import com.sosuisha.petajournal.core.tagUrl
import com.sosuisha.petajournal.sync.MarkdownFolder
import com.sosuisha.petajournal.types.AlertType
import com.sosuisha.petajournal.types.ListType
import com.sosuisha.petajournal.types.Node
import com.sosuisha.petajournal.types.PropValue

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
 * An item as it is shown: with its decoration, not as Markdown. It is drawn from the AST that
 * the core gives with the row (ADR 041). A `when` takes every kind of node, so a kind that the
 * AST gets later stops the build until it is drawn here.
 *
 * A code block and a table get parts of their own: one text cannot show them. An image of the
 * assets folder gets a part of its own, between the text before and after it in its
 * paragraph. An image of the web is not drawn: it becomes a link to its address.
 *
 * A list or a quote that holds one of these parts cannot be one text. It is taken apart, and
 * what is in it is read the same way. A list or a quote that holds none stays in the text.
 *
 * [onLink] gets the `href` of a link that the user touched. A link to a page and a tag have
 * the URL of their page, such as `/pages/notes`, encoded as a web address is. The page of a
 * tag is under [tagsDir].
 */
fun shownBlocks(ast: Node, tagsDir: String, onLink: (String) -> Unit): List<ShownBlock> =
    Reader(tagsDir, onLink).read(children(ast))

private class Reader(private val tagsDir: String, private val onLink: (String) -> Unit) {
    private val blocks = mutableListOf<ShownBlock>()

    /** The nodes since the last block of its own kind, which become one text together. */
    private val others = mutableListOf<Node>()

    /** The parts of these nodes: those of the document, of an entry of a list, or of a quote. */
    fun read(nodes: List<Node>): List<ShownBlock> {
        for (node in nodes) {
            when {
                node is Node.List && holdsOwnPart(node) -> {
                    endText()
                    blocks += ShownBlock.Entries(entries(node))
                }
                (node is Node.BlockQuote || node is Node.MultilineBlockQuote) && holdsOwnPart(node) -> {
                    endText()
                    blocks += ShownBlock.Quote(within(children(node)))
                }
                node is Node.CodeBlock -> {
                    endText()
                    blocks += ShownBlock.Code(node.literal.trimEnd('\n'))
                }
                node is Node.Table -> {
                    endText()
                    blocks += ShownBlock.Table(node.children.map(::row))
                }
                node is Node.Paragraph -> paragraph(node.children)
                else -> others += node
            }
        }
        endText()
        return blocks
    }

    private fun within(nodes: List<Node>): List<ShownBlock> = Reader(tagsDir, onLink).read(nodes)

    private fun entries(list: Node.List): List<Entry> =
        list.children.mapIndexed { index, entry ->
            Entry(mark(entry, list.listType == ListType.ORDERED, list.start.toInt() + index), within(children(entry)))
        }

    /** A paragraph is cut at each image of the assets folder: the image stands on its own. */
    private fun paragraph(inline: List<Node>) {
        val run = mutableListOf<Node>()
        fun endRun() {
            if (run.isNotEmpty()) others += Node.Paragraph(run.toList())
            run.clear()
        }
        for (child in inline) {
            val image = assetImage(child)
            if (image == null) {
                run += child
            } else {
                endRun()
                endText()
                blocks += image
            }
        }
        endRun()
    }

    private fun endText() {
        if (others.isEmpty()) return
        val text = TextWriter(tagsDir, onLink).text(others)
        // Between two images there may be only a space
        if (text.isNotEmpty()) blocks += ShownBlock.Text(text)
        others.clear()
    }

    private fun row(row: Node): TableRow {
        val cells = (row as? Node.TableRow)?.children ?: emptyList()
        // What is in a cell is part of a line: it is one paragraph
        return TableRow(
            header = (row as? Node.TableRow)?.header == true,
            cells = cells.map { cell -> TextWriter(tagsDir, onLink).text(listOf(Node.Paragraph(children(cell)))) },
        )
    }
}

/**
 * Writes blocks of an item into one text. Each block is a paragraph of the text: a list moves
 * its entries to the right, and the first line of an entry starts with its mark. This is the
 * look of this shell: it is not kept the same as the stylesheet of the web frontend.
 */
private class TextWriter(private val tagsDir: String, private val onLink: (String) -> Unit) {
    private val out = AnnotatedString.Builder()

    /** How many lists and quotes the block is inside. */
    private var depth = 0
    private var quoted = 0

    /** What the first line of the next block starts with: the mark of an entry or the name of a footnote. */
    private var lead: String? = null

    fun text(nodes: List<Node>): AnnotatedString {
        nodes.forEach(::block)
        val text = out.toAnnotatedString()
        // Every block ends its line, the last one too
        return text.subSequence(0, text.text.trimEnd().length)
    }

    private fun block(node: Node) {
        when (node) {
            is Node.Document -> node.children.forEach(::block)
            is Node.Item -> node.children.forEach(::block)
            is Node.TaskItem -> node.children.forEach(::block)
            is Node.Unknown -> node.children.forEach(::block)
            is Node.Paragraph -> line(node.children)
            is Node.Heading -> line(node.children, SpanStyle(fontWeight = FontWeight.Bold, fontSize = headingSize(node.level.toInt()).em))
            is Node.CodeBlock -> line(listOf(Node.Code(node.literal.trimEnd('\n'))))
            is Node.BlockQuote -> quote(node.children)
            is Node.MultilineBlockQuote -> quote(node.children)
            is Node.Alert -> {
                quoted++
                line(listOf(Node.Strong(listOf(Node.Text(node.title ?: name(node.alertType))))))
                node.children.forEach(::block)
                quoted--
            }
            is Node.List -> {
                depth++
                node.children.forEachIndexed { index, entry ->
                    lead = mark(entry, node.listType == ListType.ORDERED, node.start.toInt() + index) + " "
                    block(entry)
                    lead = null
                }
                depth--
            }
            is Node.Table -> node.children.forEach(::block)
            is Node.TableRow -> line(node.children.flatMap { children(it) + Node.Text("  ") })
            is Node.TableCell -> line(node.children)
            is Node.ThematicBreak -> line(listOf(Node.Text("────────")))
            is Node.HtmlBlock -> line(listOf(Node.Text(node.literal.trimEnd('\n'))))
            is Node.FootnoteDefinition -> {
                lead = "[${footnoteLabel(node.name)}]: "
                node.children.forEach(::block)
                lead = null
            }
            is Node.Subtext -> line(listOf(Node.SubscriptText(node.children)))
            // A line outside a block, as in a list without room between its entries
            is Node.Text, Node.Softbreak, Node.Linebreak, is Node.Emph, is Node.Strong, is Node.Strikethrough,
            is Node.Underline, is Node.Highlight, is Node.Superscript, is Node.SubscriptText, is Node.SpoileredText,
            is Node.Code, is Node.Link, is Node.Embed, is Node.FootnoteReference, is Node.Shortcode, is Node.Math,
            is Node.HtmlInline, is Node.Wikilink, is Node.Hashtag -> line(listOf(node))
        }
    }

    private fun quote(children: List<Node>) {
        quoted++
        children.forEach(::block)
        quoted--
    }

    /** One paragraph of the text, with the lead of the block before it. */
    private fun line(inline: List<Node>, style: SpanStyle? = null) {
        val indent = (depth + quoted) * 1.5
        out.pushStyle(ParagraphStyle(textIndent = TextIndent(firstLine = indent.em, restLine = indent.em)))
        if (quoted > 0) out.pushStyle(SpanStyle(color = QUOTED))
        style?.let(out::pushStyle)
        lead?.let(out::append)
        lead = null
        inline.forEach(::write)
        out.append("\n")
        if (style != null) out.pop()
        if (quoted > 0) out.pop()
        out.pop()
    }

    /** The letters of an inline node, with the looks of the nodes around it. */
    private fun write(node: Node) {
        fun inside(children: List<Node>, style: SpanStyle) {
            out.pushStyle(style)
            children.forEach(::write)
            out.pop()
        }
        when (node) {
            is Node.Text -> out.append(node.value)
            is Node.Softbreak -> out.append(" ")
            is Node.Linebreak -> out.append("\n")
            is Node.Emph -> inside(node.children, SpanStyle(fontStyle = FontStyle.Italic))
            is Node.Strong -> inside(node.children, SpanStyle(fontWeight = FontWeight.Bold))
            is Node.Strikethrough -> inside(node.children, SpanStyle(textDecoration = TextDecoration.LineThrough))
            is Node.Underline -> inside(node.children, SpanStyle(textDecoration = TextDecoration.Underline))
            is Node.Highlight -> inside(node.children, SpanStyle(background = MARKED))
            is Node.Superscript -> inside(node.children, SpanStyle(baselineShift = BaselineShift.Superscript, fontSize = 0.75.em))
            is Node.SubscriptText -> inside(node.children, SpanStyle(baselineShift = BaselineShift.Subscript, fontSize = 0.75.em))
            is Node.SpoileredText -> inside(node.children, SpanStyle(background = HIDDEN))
            is Node.Code -> inside(listOf(Node.Text(node.literal)), SpanStyle(fontFamily = FontFamily.Monospace))
            is Node.Link -> link(node.url, node.children)
            is Node.Wikilink -> link(pageUrl(node.url), node.children)
            is Node.Hashtag -> link(tagUrl(tagsDir, node.value), listOf(Node.Text("#" + node.value)))
            // An image of the web is a link to its address. The text of the link is what
            // stands for the image, or the address when nothing does
            is Node.Embed ->
                if (isWebAddress(node.url)) {
                    link(node.url, listOf(Node.Text(plainText(node).ifEmpty { node.url })))
                } else {
                    out.append("[${plainText(node).ifEmpty { node.url }}]")
                }
            is Node.FootnoteReference ->
                inside(listOf(Node.Text("[${footnoteLabel(node.name)}]")), SpanStyle(baselineShift = BaselineShift.Superscript, fontSize = 0.75.em))
            is Node.Shortcode -> out.append(node.emoji)
            is Node.Math -> out.append(node.literal)
            is Node.HtmlInline -> out.append(node.value)
            is Node.Unknown -> node.children.forEach(::write)
            // A block inside a line: only its letters
            is Node.Document, is Node.Paragraph, is Node.Heading, is Node.CodeBlock, is Node.BlockQuote,
            is Node.MultilineBlockQuote, is Node.List, is Node.Item, is Node.TaskItem, is Node.Table,
            is Node.TableRow, is Node.TableCell, Node.ThematicBreak, is Node.HtmlBlock, is Node.FootnoteDefinition,
            is Node.Alert, is Node.Subtext -> out.append(plainText(node))
        }
    }

    private fun link(href: String, children: List<Node>) {
        out.pushLink(LinkAnnotation.Url(href, TextLinkStyles(SpanStyle(color = LINK))) { onLink(href) })
        children.forEach(::write)
        out.pop()
    }
}

private val LINK = Color(0xFF0A58CA)
private val MARKED = Color(0xFFFFF3A3)
private val QUOTED = Color(0xFF555555)
private val HIDDEN = Color(0xFFDDDDDD)

/** How much larger than the text a heading of this level is, as a web browser draws it. */
private fun headingSize(level: Int): Double = listOf(2.0, 1.5, 1.17, 1.0, 0.83, 0.67)[level.coerceIn(1, 6) - 1]

private fun name(type: AlertType): String = when (type) {
    AlertType.NOTE -> "Note"
    AlertType.TIP -> "Tip"
    AlertType.IMPORTANT -> "Important"
    AlertType.WARNING -> "Warning"
    AlertType.CAUTION -> "Caution"
}

/**
 * What stands before an entry of a list: a checkbox for a task, its number in a numbered list,
 * and a bullet otherwise.
 */
private fun mark(entry: Node, ordered: Boolean, number: Int): String = when {
    entry is Node.TaskItem -> if (entry.symbol == null) "☐" else "☑"
    ordered -> "$number."
    else -> "•"
}

/**
 * The text that a reader sees for a footnote. An inline footnote has a name that the parser
 * made (`__inline_1`), so its number is shown instead.
 */
private fun footnoteLabel(name: String): String = name.removePrefix("__inline_")

/** True when a part of its own kind is somewhere under the node. */
private fun holdsOwnPart(node: Node): Boolean =
    children(node).any { it is Node.CodeBlock || it is Node.Table || assetImage(it) != null || holdsOwnPart(it) }

/**
 * The image that the node is, when its file is in the assets folder. A page writes the address
 * as `../assets/picture.png`, from the URL of the page in the web app.
 */
private fun assetImage(node: Node): ShownBlock.Image? {
    if (node !is Node.Embed) return null
    val path = assetPath(node.url) ?: return null
    // A side that is not a number above zero is not asked for, as in the web app
    fun side(name: String): Int? = (node.props[name] as? PropValue.Text)?.value?.toIntOrNull()?.takeIf { it > 0 }
    return ShownBlock.Image(path, plainText(node), side("w"), side("h"))
}

/**
 * The path under the Markdown folder of the file that the address names, when the file is in
 * the assets folder: `assets/picture.png` for `../assets/picture.png`. `null` for any other
 * address. The address may be encoded as a web address is.
 */
fun assetPath(address: String): String? =
    Uri.decode(address.substringBefore('#')).trimStart('.', '/').takeIf { it.startsWith(MarkdownFolder.ASSETS, ignoreCase = true) }

/** The text under the node, without its decoration. */
private fun plainText(node: Node): String =
    if (node is Node.Text) node.value else children(node).joinToString("", transform = ::plainText)

/** The children of a node, and none for a leaf. */
private fun children(node: Node): List<Node> = when (node) {
    is Node.Document -> node.children
    is Node.Paragraph -> node.children
    is Node.Heading -> node.children
    is Node.BlockQuote -> node.children
    is Node.MultilineBlockQuote -> node.children
    is Node.List -> node.children
    is Node.Item -> node.children
    is Node.TaskItem -> node.children
    is Node.Table -> node.children
    is Node.TableRow -> node.children
    is Node.TableCell -> node.children
    is Node.FootnoteDefinition -> node.children
    is Node.Alert -> node.children
    is Node.Subtext -> node.children
    is Node.Emph -> node.children
    is Node.Strong -> node.children
    is Node.Strikethrough -> node.children
    is Node.Underline -> node.children
    is Node.Highlight -> node.children
    is Node.Superscript -> node.children
    is Node.SubscriptText -> node.children
    is Node.SpoileredText -> node.children
    is Node.Link -> node.children
    is Node.Embed -> node.children
    is Node.Wikilink -> node.children
    is Node.Unknown -> node.children
    is Node.CodeBlock, Node.ThematicBreak, is Node.HtmlBlock, is Node.Text, Node.Softbreak, Node.Linebreak,
    is Node.Code, is Node.FootnoteReference, is Node.Shortcode, is Node.Math, is Node.HtmlInline, is Node.Hashtag,
    -> emptyList()
}

/** True for an address that the browser opens. */
fun isWebAddress(href: String): Boolean = href.startsWith("https://") || href.startsWith("http://")
