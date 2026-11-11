package net.sosuisen.petajournal

import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.fromHtml
import net.sosuisen.petajournal.core.FfiException
import net.sosuisen.petajournal.core.itemHtml
import net.sosuisen.petajournal.types.Item

/**
 * The text of an item as it is shown: with its decoration, not as Markdown (ADR 025).
 *
 * The shell does not read the AST for this. Rust makes HTML of it, and Compose reads the HTML:
 * the decoration of letters and the links (ADR 030). [onLink] gets the `href` of a link that
 * the user touched. An item whose AST cannot be read shows its body as it is.
 */
fun shownText(item: Item, onLink: (String) -> Unit): AnnotatedString {
    if (item.ast.isEmpty()) return AnnotatedString(item.body)
    val html = try {
        itemHtml(item.ast)
    } catch (unreadable: FfiException) {
        return AnnotatedString(item.body)
    }
    val text = AnnotatedString.fromHtml(html, linkInteractionListener = { link ->
        if (link is LinkAnnotation.Url) onLink(link.url)
    })
    // A paragraph of the HTML ends with line breaks, which would be empty lines under the item
    return text.subSequence(0, text.text.trimEnd().length)
}
