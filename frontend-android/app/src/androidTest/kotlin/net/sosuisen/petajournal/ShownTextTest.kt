package net.sosuisen.petajournal

import androidx.compose.ui.text.AnnotatedString
import androidx.test.ext.junit.runners.AndroidJUnit4
import net.sosuisen.petajournal.types.Item
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

/** The text of an item whose AST gives nothing to show. */
@RunWith(AndroidJUnit4::class)
class ShownTextTest {
    private fun item(body: String, ast: String) = Item(
        id = "i",
        pageId = "p",
        dirPath = "",
        name = "notes",
        itemType = "doc",
        createdDate = 0,
        modifiedDate = 0,
        idx = "a",
        level = 1,
        body = body,
        ast = ast,
        collapsed = false,
        types = emptyList(),
        properties = emptyMap(),
    )

    private fun shown(body: String, ast: String) = shownBlocks(item(body, ast), tagsDir = "") {}

    private val asItIs = listOf(ShownBlock.Text(AnnotatedString("**body**")))

    // ASTのない項目は、本文がそのまま見える。
    @Test
    fun an_item_without_an_ast_shows_its_body_as_it_is() {
        assertEquals(asItIs, shown("**body**", ast = ""))
    }

    // ASTがJSONとして読めない項目は、本文がそのまま見える。
    @Test
    fun an_item_whose_ast_is_not_json_shows_its_body_as_it_is() {
        assertEquals(asItIs, shown("**body**", ast = "{"))
    }
}
