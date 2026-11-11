package net.sosuisen.petajournal

import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.fromHtml
import net.sosuisen.petajournal.core.itemHtml
import net.sosuisen.petajournal.types.Item
import org.json.JSONArray
import org.json.JSONException
import org.json.JSONObject

/** One part of an item as it is shown, from top to bottom. */
sealed interface ShownBlock {
    /** Text with the decoration of its letters and its links. */
    data class Text(val text: AnnotatedString) : ShownBlock

    /** A block of code: its lines as they were written. */
    data class Code(val lines: String) : ShownBlock

    data class Table(val rows: List<TableRow>) : ShownBlock
}

/** @param header true for the row of the titles of the columns. */
data class TableRow(val header: Boolean, val cells: List<AnnotatedString>)

/**
 * An item as it is shown: with its decoration, not as Markdown (ADR 025).
 *
 * Rust makes HTML of the AST of the item, and Compose reads the HTML: that gives the
 * decoration of letters and the links. Compose loses the line breaks of a code block and the
 * cells of a table, so those two are read from the AST here and get parts of their own (ADR
 * 030). Only the blocks at the top of the item are told apart: a code block inside a list or a
 * quote stays in the text.
 *
 * [onLink] gets the `href` of a link that the user touched. A link to a page and a tag have
 * the URL of their page, such as `/pages/notes`, encoded as a web address is. The page of a
 * tag is under [tagsDir]. An item whose AST cannot be read shows its body as it is.
 */
fun shownBlocks(item: Item, tagsDir: String, onLink: (String) -> Unit): List<ShownBlock> {
    if (item.ast.isEmpty()) return listOf(ShownBlock.Text(AnnotatedString(item.body)))
    return try {
        Reader(tagsDir, onLink).read(JSONObject(item.ast))
    } catch (unreadable: JSONException) {
        listOf(ShownBlock.Text(AnnotatedString(item.body)))
    }
}

private class Reader(private val tagsDir: String, private val onLink: (String) -> Unit) {
    private val blocks = mutableListOf<ShownBlock>()

    /** The nodes since the last block of its own kind, which become one text together. */
    private var others = JSONArray()

    fun read(document: JSONObject): List<ShownBlock> {
        val nodes = document.optJSONArray("children") ?: JSONArray()
        for (node in nodes.objects()) {
            when (node.getString("type")) {
                "code_block" -> {
                    endText()
                    blocks += ShownBlock.Code(node.optString("literal").trimEnd('\n'))
                }
                "table" -> {
                    endText()
                    blocks += ShownBlock.Table(node.children().map(::row))
                }
                else -> others.put(node)
            }
        }
        endText()
        return blocks
    }

    private fun endText() {
        if (others.length() == 0) return
        blocks += ShownBlock.Text(text(others))
        others = JSONArray()
    }

    private fun row(row: JSONObject): TableRow =
        TableRow(
            header = row.optBoolean("header"),
            // What is in a cell is part of a line, and the HTML is made of whole blocks
            cells = row.children().map { cell ->
                val paragraph = JSONObject().put("type", "paragraph").put("children", cell.optJSONArray("children") ?: JSONArray())
                text(JSONArray().put(paragraph))
            },
        )

    /** The text of the blocks, by way of their HTML. */
    private fun text(nodes: JSONArray): AnnotatedString {
        val document = JSONObject().put("type", "document").put("children", nodes)
        val text = AnnotatedString.fromHtml(itemHtml(document.toString(), tagsDir), linkInteractionListener = { link ->
            if (link is LinkAnnotation.Url) onLink(link.url)
        })
        // A paragraph of the HTML ends with line breaks, which would be empty lines under it
        return text.subSequence(0, text.text.trimEnd().length)
    }
}

private fun JSONObject.children(): List<JSONObject> = (optJSONArray("children") ?: JSONArray()).objects()

private fun JSONArray.objects(): List<JSONObject> = (0 until length()).map(::getJSONObject)
