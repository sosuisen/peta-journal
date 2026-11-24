package com.sosuisha.petajournal.screen

import androidx.compose.ui.text.LinkAnnotation
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.sosuisha.petajournal.types.Looks
import com.sosuisha.petajournal.types.RichBlock
import com.sosuisha.petajournal.types.RichEntry
import com.sosuisha.petajournal.types.RichImage
import com.sosuisha.petajournal.types.RichLink
import com.sosuisha.petajournal.types.RichParagraph
import com.sosuisha.petajournal.types.RichRun
import com.sosuisha.petajournal.types.RichText
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

/**
 * The parts of an item, written from the parts that the core gives with the row (ADR 042). How the
 * core makes the parts is tested in Rust.
 */
@RunWith(AndroidJUnit4::class)
class ShownTextTest {
    private val plain = Looks(false, false, false, false, false, false, false, false, false)

    private fun run(text: String, link: RichLink? = null) = RichRun(text, plain, link)

    private fun paragraph(
        vararg runs: RichRun,
        mark: String? = null,
        depth: UInt = 0u,
        start: UInt = 0u,
    ) =
        RichParagraph(
            heading = 0u,
            scale = 1.0f,
            depth = depth,
            quoted = 0u,
            indent = 0.0f,
            mark = mark,
            rule = false,
            runs = runs.toList(),
            start = start,
        )

    private fun text(vararg paragraphs: RichParagraph) = RichBlock.Text(paragraphs.toList())

    private fun shown(vararg blocks: RichBlock, onLink: (RichLink) -> Unit = {}) =
        shownBlocks(RichText(blocks.toList()), onLink)

    private fun letters(block: ShownBlock) = (block as ShownBlock.Text).text.text

    // 段落は、文字の部品になる。終わりに改行はない。
    @Test
    fun paragraphs_are_one_text_without_a_line_break_at_its_end() {
        assertEquals(
            listOf("one\ntwo"),
            shown(text(paragraph(run("one")), paragraph(run("two")))).map(::letters),
        )
    }

    // コードブロックは、その行のままの部品になる。
    @Test
    fun a_code_block_is_a_part_with_its_lines() {
        assertEquals(listOf(ShownBlock.Code("a\nb", 4)), shown(RichBlock.Code("a\nb", 4u)))
    }

    // 箇条書きの項目は、印から始まる。
    @Test
    fun an_entry_of_a_list_starts_with_its_mark() {
        val list =
            text(
                paragraph(run("c"), mark = "3.", depth = 1u),
                paragraph(run("d"), mark = "☑", depth = 1u),
            )
        assertEquals(listOf("3. c\n☑ d"), shown(list).map(::letters))
    }

    // 見出しの段落の文字は、コアが決めた倍率（em）の大きさになる（#795）。
    @Test
    fun a_heading_has_the_size_that_the_core_gives() {
        val heading = paragraph(run("h")).copy(heading = 5u, scale = 1.25f)
        val text = (shown(text(heading)).single() as ShownBlock.Text).text
        assertEquals(1.25f, text.spanStyles.single { it.item.fontSize.isEm }.item.fontSize.value)
    }

    // 段落は、コアが決めた字下げ（em）だけ右に寄る（#795）。
    @Test
    fun a_paragraph_is_indented_as_far_as_the_core_says() {
        val entry = paragraph(run("e"), mark = "•", depth = 1u).copy(indent = 1.25f)
        val text = (shown(text(entry)).single() as ShownBlock.Text).text
        val indent = text.paragraphStyles.single().item.textIndent!!
        assertEquals(listOf(1.25f, 1.25f), listOf(indent.firstLine.value, indent.restLine.value))
    }

    // 部品を持つリストと引用は、コアが決めた字下げ（em）を持つ（#795）。
    @Test
    fun a_list_or_a_quote_that_holds_a_part_has_the_indent_that_the_core_gives() {
        val code = RichBlock.Code("c", 0u)
        val list = RichBlock.Entries(listOf(RichEntry("•", listOf(code))), 1.25f)
        val indents =
            shown(list, RichBlock.Quote(listOf(code), 1.75f)).map {
                when (it) {
                    is ShownBlock.Entries -> it.indent
                    is ShownBlock.Quote -> it.indent
                    else -> null
                }
            }
        assertEquals(listOf(1.25f, 1.75f), indents)
    }

    // 文字の位置は、項目の表示の文字のうち、その前に立つ文字の数になる。印の上は、その段落の先頭である。
    @Test
    fun a_place_in_a_text_is_the_count_of_the_shown_letters_before_it() {
        val list =
            text(
                paragraph(run("ab"), mark = "1.", depth = 1u),
                paragraph(run("😀c"), mark = "2.", depth = 1u, start = 3u),
            )
        val shown = shown(list).single() as ShownBlock.Text

        // The text is "1. ab\n2. 😀c"; 😀 is two UTF-16 units and one letter of the core
        assertEquals(0, letterAt(shown.places, 1))
        assertEquals(1, letterAt(shown.places, 4))
        assertEquals(3, letterAt(shown.places, 7))
        assertEquals(5, letterAt(shown.places, 12))
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
        val page = RichLink.Page("/pages/other%20page")
        val touched = mutableListOf<RichLink>()
        val shown =
            shown(text(paragraph(run("other", page), run(" plain")))) { touched += it }.single()
                as ShownBlock.Text

        val links = shown.text.getLinkAnnotations(0, shown.text.length)
        assertEquals(listOf(0 to 5), links.map { it.start to it.end })
        (links.single().item as LinkAnnotation.Clickable)
            .linkInteractionListener
            ?.onClick(links.single().item)
        assertEquals(listOf<RichLink>(page), touched)
    }

    // 空の文字の部品は、出ない。
    @Test
    fun a_text_without_letters_is_not_shown() {
        assertEquals(emptyList<ShownBlock>(), shown(text(paragraph(run("  ")))))
    }
}
