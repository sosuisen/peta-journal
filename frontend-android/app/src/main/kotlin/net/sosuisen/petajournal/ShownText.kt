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
     */
    data class Image(val path: String, val alt: String, val width: Int?) : ShownBlock
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
 * Compose draws no image of the HTML either. An image of the assets folder in a paragraph at
 * the top of the item gets a part of its own, between the text before and after it. An image
 * of the web is not drawn: it becomes a link to its address.
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
                "paragraph" -> paragraph(node)
                else -> others.put(node)
            }
        }
        endText()
        return blocks
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

/**
 * The image that the node is, when its file is in the assets folder. A page writes the address
 * as `../assets/picture.png`, from the URL of the page in the web app.
 */
private fun assetImage(node: JSONObject): ShownBlock.Image? {
    if (node.optString("type") != IMAGE) return null
    val path = Uri.decode(node.optString("url")).trimStart('.', '/')
    if (!path.startsWith(MarkdownFolder.ASSETS, ignoreCase = true)) return null
    val width = node.optJSONObject("props")?.optString("w")?.toIntOrNull()?.takeIf { it > 0 }
    return ShownBlock.Image(path, plainText(node), width)
}

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
