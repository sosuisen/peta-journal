package com.sosuisha.petajournal.screen

import android.graphics.Bitmap
import androidx.compose.ui.layout.FirstBaseline
import androidx.compose.ui.semantics.SemanticsNode
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.SemanticsNodeInteraction
import androidx.compose.ui.test.assertHeightIsEqualTo
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.assertWidthIsEqualTo
import androidx.compose.ui.test.getAlignmentLinePosition
import androidx.compose.ui.test.getUnclippedBoundsInRoot
import androidx.compose.ui.test.hasAnyAncestor
import androidx.compose.ui.test.hasContentDescription
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.isDialog
import androidx.compose.ui.test.onFirst
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.unit.dp
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.sosuisha.petajournal.types.ScreenView
import java.io.File
import kotlin.math.abs
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * How an item looks: its decoration, its code, tables and images, and where its bullet and its mark
 * stand.
 */
@RunWith(AndroidJUnit4::class)
class ItemLookTest : JournalScreenFixture() {
    // 今日のページで下に続く過去の日付は、今日の日付と同じ大きさで見える（#885）。
    @Test
    fun a_past_date_on_the_page_of_today_is_as_large_as_the_date_of_today() {
        File(root, "markdown/journals").mkdirs()
        File(root, "markdown/journals/2020-01-01.md").writeText("an old note" + NL)
        start()
        compose.onNode(hasContentDescription("Today")).performClick()
        waitFor("2020年1月1日")
        val today = (shell.view.screen as ScreenView.Journal).value.displayDate

        val past = compose.onNode(hasText("2020年1月1日", substring = true)).getUnclippedBoundsInRoot()
        val now = compose.onNode(hasText(today)).getUnclippedBoundsInRoot()

        assertEquals(now.bottom - now.top, past.bottom - past.top)
    }

    // 項目は、Markdownの記号ではなく、飾りの付いた文字で見える。
    @Test
    fun an_item_shows_its_decoration_and_not_the_marks_of_markdown() {
        download("notes.md", "some **bold** text\n")
        start()

        type("notes")
        waitFor("notes")
        compose.onAllNodes(shown("notes")).onFirst().performClick()

        waitFor("some bold text")
    }

    // コードブロックは、行が分かれたまま、書かれたとおりに見える。フェンスの記号は見えない。
    @Test
    fun a_code_block_shows_its_lines_as_they_were_written() {
        download("notes.md", "before\n\n```kotlin\nval x = 1\n  val y = 2\n```\n\nafter\n")

        openNotes()

        waitFor("before")
        compose.onNode(hasText("val x = 1\n  val y = 2")).assertExists()
        compose.onNode(hasText("after")).assertExists()
    }

    // 表は、セルごとに分かれて見える。セルの中の文字の飾りも付く。
    @Test
    fun a_table_shows_each_cell_apart() {
        download("notes.md", "| name | count |\n|---|---|\n| **apple** | 2 |\n")

        openNotes()

        waitFor("name")
        for (cell in listOf("name", "count", "apple", "2")) compose
            .onNode(hasText(cell))
            .assertExists()
    }

    /** Puts a picture into the assets folder, as a download does. */
    private fun picture(name: String, width: Int, height: Int) {
        val file = File(root, "markdown/assets/$name").apply { parentFile?.mkdirs() }
        val bitmap = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
        file.outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
    }

    private fun waitForPicture(alt: String) =
        compose.waitUntil(WAIT_MS) {
            compose.onAllNodes(hasContentDescription(alt)).fetchSemanticsNodes().isNotEmpty()
        }

    // assetsの画像は、前後の文字のあいだに、絵として見える。ファイルの名前の大文字と小文字は問わない。
    @Test
    fun an_image_of_the_assets_folder_shows_between_the_text_around_it() {
        picture("cat.png", width = 40, height = 20)
        download("notes.md", "before ![a cat](../assets/Cat.png) after" + NL)

        openNotes()

        waitForPicture("a cat")
        compose.onNode(hasText("before")).assertExists()
        compose.onNode(hasText("after")).assertExists()
    }

    // 画像は、ページが決めた幅で見える。幅がなければ、画像そのものの幅で見える。
    @Test
    fun an_image_has_the_width_that_the_page_asks_for() {
        picture("cat.png", width = 40, height = 20)
        download(
            "notes.md",
            "![asked](../assets/cat.png#w=120)" + NL + NL + "![natural](../assets/cat.png)" + NL,
        )

        openNotes()

        waitForPicture("asked")
        compose
            .onNode(hasContentDescription("asked"))
            .assertWidthIsEqualTo(120.dp)
            .assertHeightIsEqualTo(60.dp)
        compose.onNode(hasContentDescription("natural")).assertWidthIsEqualTo(40.dp)
    }

    // ページが高さも決めた画像は、その高さで見える。高さだけなら、幅は画像の形に従う。
    @Test
    fun an_image_has_the_height_that_the_page_asks_for() {
        picture("cat.png", width = 40, height = 20)
        download(
            "notes.md",
            "![both](../assets/cat.png#w=120&h=30)" +
                NL +
                NL +
                "![tall](../assets/cat.png#h=60)" +
                NL,
        )

        openNotes()

        waitForPicture("both")
        compose
            .onNode(hasContentDescription("both"))
            .assertWidthIsEqualTo(120.dp)
            .assertHeightIsEqualTo(30.dp)
        compose
            .onNode(hasContentDescription("tall"))
            .assertWidthIsEqualTo(120.dp)
            .assertHeightIsEqualTo(60.dp)
    }

    // 画像に触れると、その画像が大きく開く。開いた画像に触れると、閉じる。
    @Test
    fun a_touch_opens_an_image_large_and_another_touch_closes_it() {
        picture("cat.png", width = 40, height = 20)
        download("notes.md", "![a cat](../assets/cat.png)" + NL)
        openNotes()
        waitForPicture("a cat")
        val large = hasContentDescription("a cat") and hasAnyAncestor(isDialog())
        compose.onNode(large).assertDoesNotExist()

        compose.onNode(hasContentDescription("a cat")).performClick()

        compose.waitUntil(WAIT_MS) { compose.onAllNodes(large).fetchSemanticsNodes().isNotEmpty() }
        compose.onNode(large).performClick()
        compose.waitUntil(WAIT_MS) { compose.onAllNodes(large).fetchSemanticsNodes().isEmpty() }
        compose.onNode(hasContentDescription("a cat")).assertExists()
    }

    // 画像は、1行目に代わりの文字（alt）が見え、その下に絵が見える（#886）。
    @Test
    fun an_image_shows_its_alt_on_a_line_above_the_picture() {
        picture("cat.png", width = 40, height = 20)
        download("notes.md", "![a cat](../assets/cat.png)" + NL)

        openNotes()

        waitForPicture("a cat")
        val line = compose.onNode(shown("a cat")).getUnclippedBoundsInRoot()
        val picture = compose.onNode(hasContentDescription("a cat")).getUnclippedBoundsInRoot()
        assertTrue(
            "the alt ends at ${line.bottom}, below the top of the picture at ${picture.top}",
            line.bottom <= picture.top,
        )
    }

    // 代わりの文字（alt）のない画像は、1行目にファイルのパスが見える（#886）。
    @Test
    fun an_image_without_an_alt_shows_its_path_on_the_line_above_the_picture() {
        picture("cat.png", width = 40, height = 20)
        download("notes.md", "![](../assets/cat.png)" + NL)

        openNotes()

        waitFor("assets/cat.png")
    }

    // 画像の1行目に触れると、その項目がMarkdownの入力欄になる（#886）。
    @Test
    fun a_touch_on_the_line_above_an_image_edits_the_item() {
        picture("cat.png", width = 40, height = 20)
        download("notes.md", "![a cat](../assets/cat.png)" + NL)
        openNotes()
        waitForPicture("a cat")

        compose.onNode(shown("a cat")).performClick()

        compose.waitUntil(WAIT_MS) {
            compose.onAllNodes(hasTestTag(EDITOR_TAG)).fetchSemanticsNodes().isNotEmpty()
        }
        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals("![a cat](../assets/cat.png)")
    }

    // ファイルのない画像は、代わりの文字（alt）が見える。
    @Test
    fun an_image_whose_file_is_not_here_shows_its_alt() {
        download("notes.md", "![a dog](../assets/dog.png)" + NL)

        openNotes()

        waitFor("a dog")
    }

    // Web上の画像は描かない。代わりの文字のリンクになる。
    @Test
    fun an_image_of_the_web_is_a_link_and_not_a_picture() {
        download("notes.md", "see ![a bird](https://example.com/bird.png) here" + NL)

        openNotes()

        waitFor("see a bird here")
        compose.onNode(hasContentDescription("a bird")).assertDoesNotExist()
    }

    // リストの中のコードブロックも、行が分かれたまま見える。リストの項目は、番号と一緒に残る。
    @Test
    fun a_code_block_inside_a_list_shows_its_lines_as_they_were_written() {
        download(
            "notes.md",
            above('A') +
                "1. step one" +
                NL +
                NL +
                "   ```" +
                NL +
                "   val x = 1" +
                NL +
                "     val y = 2" +
                NL +
                "   ```" +
                NL +
                "2. step two" +
                NL,
        )

        openNotes()

        waitFor("step one")
        compose.onNode(hasText("val x = 1\n  val y = 2")).assertExists()
        for (text in listOf("1.", "step one", "2.", "step two")) compose
            .onNode(hasText(text))
            .assertExists()
    }

    // 引用の中の表も、セルごとに分かれて見える。
    @Test
    fun a_table_inside_a_quote_shows_each_cell_apart() {
        download(
            "notes.md",
            above('A') +
                "> said:" +
                NL +
                ">" +
                NL +
                "> | name | count |" +
                NL +
                "> |---|---|" +
                NL +
                "> | apple | 2 |" +
                NL,
        )

        openNotes()

        waitFor("said:")
        for (cell in listOf("name", "count", "apple", "2")) compose
            .onNode(hasText(cell))
            .assertExists()
    }

    // リストの中の画像も、絵として見える。リストの中のリストでも同じである。
    @Test
    fun an_image_inside_a_list_shows_as_a_picture() {
        picture("cat.png", width = 40, height = 20)
        download(
            "notes.md",
            above('A') +
                "- outer" +
                NL +
                "  - look ![a cat](../assets/cat.png)" +
                NL +
                "- next" +
                NL,
        )

        openNotes()

        waitForPicture("a cat")
        for (text in listOf("outer", "look", "next")) compose.onNode(hasText(text)).assertExists()
    }

    // タスクの項目には、その状態の印が付く。まだ始めていない、進行中、済みの3つである。
    @Test
    fun a_task_shows_the_mark_of_its_state() {
        download(
            "notes.md",
            above('A') +
                "- [ ] buy milk" +
                NL +
                NL +
                above('B', " begin=\"2020-01-01T00:00:00Z\"") +
                "- [ ] write the report" +
                NL +
                NL +
                above('C', " end=\"2020-01-02T00:00:00Z\"") +
                "- [x] call home" +
                NL +
                NL +
                above('D') +
                "not a task" +
                NL,
        )

        openNotes()

        waitFor("call home")
        // The mark of the file is not part of the text
        compose.onNode(hasText("buy milk")).assertExists()
        val marks =
            compose
                .onAllNodes(hasContentDescription("Task", substring = true))
                .fetchSemanticsNodes()
        assertEquals(
            listOf("Task not started", "Task running", "Task done"),
            marks.map { it.config[SemanticsProperties.ContentDescription].single() },
        )
    }

    /** How far down the screen the middle of a node is, in pixels. */
    private fun middle(node: SemanticsNode): Float = node.boundsInRoot.center.y

    // タスクの印は、項目の文字と同じ高さにある。
    @Test
    fun the_mark_of_a_task_is_at_the_height_of_its_text() {
        download("notes.md", above('A') + "- [ ] buy milk" + NL)
        openNotes()
        waitFor("buy milk")

        val mark = compose.onNode(hasContentDescription("Task not started")).fetchSemanticsNode()
        val text = compose.onNode(hasText("buy milk"), useUnmergedTree = true).fetchSemanticsNode()

        val apart = abs(middle(mark) - middle(text)) / compose.density.density
        assertTrue("the mark is ${apart}dp from the middle of the text", apart < 3f)
    }

    /** How far down the screen the middle of the letters of the first line of a text is, in dp. */
    private fun middleOfLetters(text: SemanticsNodeInteraction, sizeSp: Float): Float {
        val line =
            text.getUnclippedBoundsInRoot().top + text.getAlignmentLinePosition(FirstBaseline)
        // The middle of the height of a letter is this much of its size above the line it stands on
        return line.value - 0.38f * sizeSp * compose.density.fontScale
    }

    private fun bullet(): SemanticsNode =
        compose.onNodeWithTag(BULLET_TAG, useUnmergedTree = true).fetchSemanticsNode()

    // 見出しのバレットは、見出しの大きな文字の真ん中の高さにある。
    @Test
    fun the_bullet_of_a_heading_is_at_the_middle_of_its_large_letters() {
        download("notes.md", above('A') + "# Big" + NL)
        openNotes()
        waitFor("Big")

        val bullet = middle(bullet()) / compose.density.density
        // The first heading is 1.6 times as large as plain text, as the core says (#795)
        val heading =
            middleOfLetters(compose.onNode(hasText("Big"), useUnmergedTree = true), sizeSp = 25.6f)

        assertTrue(
            "the bullet is at ${bullet}dp, the letters at ${heading}dp",
            abs(bullet - heading) < 1f,
        )
    }

    // ふつうの項目のバレットは、文字の高さの真ん中にある。大きさは7dpである。
    @Test
    fun the_bullet_of_plain_text_is_at_the_middle_of_its_letters() {
        download("notes.md", above('A') + "plain" + NL)
        openNotes()
        waitFor("plain")

        val bullet = middle(bullet()) / compose.density.density
        val letters =
            middleOfLetters(compose.onNode(hasText("plain"), useUnmergedTree = true), sizeSp = 16f)

        assertTrue(
            "the bullet is at ${bullet}dp, the letters at ${letters}dp",
            abs(bullet - letters) < 1f,
        )
        assertEquals(7f, bullet().boundsInRoot.height / compose.density.density, 0.6f)
    }

    // 画像のアドレスは、Webのアドレスと同じ書き方でもよい。空白や日本語の名前のファイルが見える。
    @Test
    fun an_image_whose_address_is_encoded_shows_as_a_picture() {
        picture("猫 の絵.png", width = 40, height = 20)
        download("notes.md", "![a cat](../assets/%E7%8C%AB%20%E3%81%AE%E7%B5%B5.png)" + NL)

        openNotes()

        waitForPicture("a cat")
    }
}
