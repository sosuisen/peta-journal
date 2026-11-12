package net.sosuisen.petajournal

import android.net.Uri
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.fromHtml
import net.sosuisen.petajournal.core.itemHtml
import net.sosuisen.petajournal.sync.MarkdownFolder
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
 * An item as it is shown: with its decoration, not as Markdown (ADR 025).
 *
 * Rust makes HTML of the AST of the item, and Compose reads the HTML: that gives the
 * decoration of letters and the links. Compose loses the line breaks of a code block and the
 * cells of a table, so those two are read from the AST here and get parts of their own (ADR
 * 030).
 *
 * Compose draws no image of the HTML either. An image of the assets folder gets a part of its
 * own, between the text before and after it in its paragraph. An image of the web is not
 * drawn: it becomes a link to its address.
 *
 * A list or a quote that holds one of these parts cannot be one text. It is taken apart, and
 * what is in it is read the same way. A list or a quote that holds none stays in the text.
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

    /** The parts of what is in the node: the document, an entry of a list, or a quote. */
    fun read(holder: JSONObject): List<ShownBlock> {
        for (node in holder.children()) {
            val type = node.getString("type")
            when {
                type == "list" && holdsOwnPart(node) -> {
                    endText()
                    blocks += ShownBlock.Entries(entries(node))
                }
                type in QUOTES && holdsOwnPart(node) -> {
                    endText()
                    blocks += ShownBlock.Quote(within(node))
                }
                else -> part(node, type)
            }
        }
        endText()
        return blocks
    }

    private fun within(holder: JSONObject): List<ShownBlock> = Reader(tagsDir, onLink).read(holder)

    private fun entries(list: JSONObject): List<Entry> {
        val first = list.optInt("start", 1)
        return list.children().mapIndexed { index, entry ->
            val mark = when {
                entry.optString("type") == "task_item" -> if (entry.isNull("symbol")) "☐" else "☑"
                list.optString("list_type") == "ordered" -> "${first + index}."
                else -> "•"
            }
            Entry(mark, within(entry))
        }
    }

    private fun part(node: JSONObject, type: String) {
        when (type) {
            "code_block" -> {
                endText()
                blocks += ShownBlock.Code(node.optString("literal").trimEnd('\n'))
            }
            "table" -> {
                endText()
                blocks += ShownBlock.Table(node.children().map(::row))
            }
            "paragraph" -> paragraph(node)
            else -> others.put(node)
        }
    }

    /** A paragraph is cut at each image of the assets folder: the image stands on its own. */
    private fun paragraph(node: JSONObject) {
        var run = JSONArray()
        fun endRun() {
            if (run.length() > 0) others.put(JSONObject().put("type", "paragraph").put("children", run))
            run = JSONArray()
        }
        for (child in node.children()) {
            val image = assetImage(child)
            if (image == null) {
                run.put(child)
            } else {
                endRun()
                endText()
                blocks += image
            }
        }
        endRun()
    }

    private fun endText() {
        if (others.length() == 0) return
        val text = text(others)
        // Between two images there may be only a space
        if (text.isNotEmpty()) blocks += ShownBlock.Text(text)
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
        val document = webImagesAsLinks(JSONObject().put("type", "document").put("children", nodes))
        val text = AnnotatedString.fromHtml(itemHtml(document.toString(), tagsDir), linkInteractionListener = { link ->
            if (link is LinkAnnotation.Url) onLink(link.url)
        })
        // A paragraph of the HTML ends with line breaks, which would be empty lines under it
        return text.subSequence(0, text.text.trimEnd().length)
    }
}

private const val IMAGE = "embed"

private val QUOTES = setOf("block_quote", "multiline_block_quote")

/** True when a part of its own kind is somewhere under the node. */
private fun holdsOwnPart(node: JSONObject): Boolean =
    node.children().any { it.optString("type") in setOf("code_block", "table") || assetImage(it) != null || holdsOwnPart(it) }

/**
 * The image that the node is, when its file is in the assets folder. A page writes the address
 * as `../assets/picture.png`, from the URL of the page in the web app.
 */
private fun assetImage(node: JSONObject): ShownBlock.Image? {
    if (node.optString("type") != IMAGE) return null
    val path = assetPath(node.optString("url")) ?: return null
    val props = node.optJSONObject("props")
    // A side that is not a number above zero is not asked for, as in the web app
    fun side(name: String): Int? = props?.optString(name)?.toIntOrNull()?.takeIf { it > 0 }
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
private fun plainText(node: JSONObject): String =
    node.optString("value") + node.children().joinToString("", transform = ::plainText)

/**
 * The same tree, with a link in the place of each image of the web. The text of the link is
 * what stands for the image, or the address when nothing does.
 */
private fun webImagesAsLinks(node: JSONObject): JSONObject {
    val url = node.optString("url")
    if (node.optString("type") == IMAGE && isWebAddress(url)) {
        val text = JSONObject().put("type", "text").put("value", plainText(node).ifEmpty { url })
        return JSONObject().put("type", "link").put("url", url).put("children", JSONArray().put(text))
    }
    val children = node.optJSONArray("children") ?: return node
    return JSONObject(node.toString()).put("children", JSONArray(children.objects().map(::webImagesAsLinks)))
}

/** True for an address that the browser opens. */
fun isWebAddress(href: String): Boolean = href.startsWith("https://") || href.startsWith("http://")

private fun JSONObject.children(): List<JSONObject> = (optJSONArray("children") ?: JSONArray()).objects()

private fun JSONArray.objects(): List<JSONObject> = (0 until length()).map(::getJSONObject)
