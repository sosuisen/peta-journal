package com.sosuisha.petajournal.screen

import androidx.compose.ui.text.LinkAnnotation
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.sosuisha.petajournal.types.Looks
import com.sosuisha.petajournal.types.RichBlock
import com.sosuisha.petajournal.types.RichImage
import com.sosuisha.petajournal.types.RichLink
import com.sosuisha.petajournal.types.RichParagraph
import com.sosuisha.petajournal.types.RichRun
import com.sosuisha.petajournal.types.RichText
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

/**
 * The parts of an item, written from the parts that the core gives with the row (ADR 042). How
 * the core makes the parts is tested in Rust.
 */
@RunWith(AndroidJUnit4::class)
class ShownTextTest {
    private val plain = Looks(false, false, false, false, false, false, false, false, false)

    private fun run(text: String, link: RichLink? = null) = RichRun(text, plain, link)

    private fun paragraph(vararg runs: RichRun, mark: String? = null, depth: UInt = 0u) =
        RichParagraph(heading = 0u, depth = depth, quoted = 0u, mark = mark, rule = false, runs = runs.toList())

    private fun text(vararg paragraphs: RichParagraph) = RichBlock.Text(paragraphs.toList())

    private fun shown(vararg blocks: RichBlock, onLink: (RichLink) -> Unit = {}) =
        shownBlocks(RichText(blocks.toList()), onLink)

    private fun letters(block: ShownBlock) = (block as ShownBlock.Text).text.text

    // 段落は、文字の部品になる。終わりに改行はない。
    @Test
    fun paragraphs_are_one_text_without_a_line_break_at_its_end() {
        assertEquals(listOf("one\ntwo"), shown(text(paragraph(run("one")), paragraph(run("two")))).map(::letters))
    }

    // コードブロックは、その行のままの部品になる。
    @Test
    fun a_code_block_is_a_part_with_its_lines() {
        assertEquals(listOf(ShownBlock.Code("a\nb")), shown(RichBlock.Code("a\nb")))
    }

    // 箇条書きの項目は、印から始まる。
    @Test
    fun an_entry_of_a_list_starts_with_its_mark() {
        val list = text(paragraph(run("c"), mark = "3.", depth = 1u), paragraph(run("d"), mark = "☑", depth = 1u))
        assertEquals(listOf("3. c\n☑ d"), shown(list).map(::letters))
    }

    // assetsの画像は、画像の部品になる。ページが求める大きさを持つ。
    @Test
    fun an_image_of_the_assets_folder_is_a_part_with_the_size_that_the_page_asks_for() {
        val image = RichBlock.Image(RichImage("assets/Cat.png", "a cat", 120u, null))
        assertEquals(listOf(ShownBlock.Image("assets/Cat.png", "a cat", 120, null)), shown(image))
    }

    // リンクに触れると、coreが決めた行き先が届く。
    @Test
    fun a_touch_on_a_link_gives_where_the_core_says_it_leads() {
        val page = RichLink.Page("/pages/other page")
        val touched = mutableListOf<RichLink>()
        val shown = shown(text(paragraph(run("other", page), run(" plain")))) { touched += it }.single() as ShownBlock.Text

        val links = shown.text.getLinkAnnotations(0, shown.text.length)
        assertEquals(listOf(0 to 5), links.map { it.start to it.end })
        (links.single().item as LinkAnnotation.Clickable).linkInteractionListener?.onClick(links.single().item)
        assertEquals(listOf<RichLink>(page), touched)
    }

    // 空の文字の部品は、出ない。
    @Test
    fun a_text_without_letters_is_not_shown() {
        assertEquals(emptyList<ShownBlock>(), shown(text(paragraph(run("  ")))))
    }
}
