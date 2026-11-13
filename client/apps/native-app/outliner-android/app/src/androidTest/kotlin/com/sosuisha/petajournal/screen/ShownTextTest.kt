package com.sosuisha.petajournal.screen

import androidx.compose.ui.text.LinkAnnotation
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.sosuisha.petajournal.types.ListDelimiter
import com.sosuisha.petajournal.types.ListType
import com.sosuisha.petajournal.types.Node
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

/** The parts of an item, drawn from the AST that the core gives with the row (ADR 041). */
@RunWith(AndroidJUnit4::class)
class ShownTextTest {
    private fun text(value: String) = Node.Text(value)

    private fun paragraph(vararg inline: Node) = Node.Paragraph(inline.toList())

    private fun shown(vararg blocks: Node) = shownBlocks(Node.Document(blocks.toList()), tagsDir = "tags/") {}

    private fun letters(block: ShownBlock) = (block as ShownBlock.Text).text.text

    // 段落は、文字の部品になる。終わりに改行はない。
    @Test
    fun paragraphs_are_one_text_without_a_line_break_at_its_end() {
        assertEquals(listOf("one\ntwo"), shown(paragraph(text("one")), paragraph(text("two"))).map(::letters))
    }

    // コードブロックは、その行のままの部品になる。
    @Test
    fun a_code_block_is_a_part_with_its_lines() {
        assertEquals(listOf(ShownBlock.Code("a\nb")), shown(Node.CodeBlock(fenced = true, info = "", literal = "a\nb\n")))
    }

    // 箇条書きの項目は、印から始まる。
    @Test
    fun an_entry_of_a_list_starts_with_its_mark() {
        val list = Node.List(
            ListType.ORDERED, 3u, true, ListDelimiter.PERIOD,
            listOf(Node.Item(ListType.ORDERED, 3u, listOf(paragraph(text("c")))), Node.TaskItem("x", listOf(paragraph(text("d"))))),
        )
        assertEquals(listOf("3. c\n☑ d"), shown(list).map(::letters))
    }

    // assetsの画像は、前後の文字のあいだの、画像の部品になる。
    @Test
    fun an_image_of_the_assets_folder_stands_between_the_text_around_it() {
        val image = Node.Embed("../assets/Cat.png", "", emptyMap(), listOf(text("a cat")))
        val blocks = shown(paragraph(text("before "), image, text(" after")))
        assertEquals(ShownBlock.Image("assets/Cat.png", "a cat", null, null), blocks[1])
        assertEquals(listOf("before", "after"), listOf(blocks[0], blocks[2]).map { letters(it).trim() })
    }

    // ページへのリンクとタグの行き先は、そのページのURLである。
    @Test
    fun a_link_to_a_page_and_a_tag_lead_to_the_url_of_their_page() {
        val line = paragraph(Node.Wikilink("other page", listOf(text("other"))), text(" "), Node.Hashtag("topic"))
        val links = (shown(line).single() as ShownBlock.Text).text.getLinkAnnotations(0, 20).map { (it.item as LinkAnnotation.Url).url }
        assertEquals(listOf("/pages/other%20page", "/pages/tags/topic"), links)
    }
}
