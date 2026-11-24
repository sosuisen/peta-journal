package com.sosuisha.petajournal.screen

import android.content.ClipData
import android.content.ClipDescription
import android.content.ClipboardManager
import android.net.Uri
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputContentInfo
import androidx.compose.ui.platform.ViewRootForTest
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.click
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performSemanticsAction
import androidx.compose.ui.test.performTouchInput
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/** An image pasted into an item goes into the assets, and its link into the item (ADR 071). */
class PasteImageTest : JournalScreenFixture() {
    private val context
        get() = InstrumentationRegistry.getInstrumentation().targetContext

    /** Opens a page of one item, and edits it at its end. */
    private fun editNotes() {
        download("notes.md", above('A') + "first" + NL)
        openNotes()
        waitFor("first")
        compose.onNodeWithText("first").performTouchInput { click(centerRight) }
        compose.onNodeWithTag(EDITOR_TAG).assertTextEquals("first")
    }

    private fun body(): String = shell.view.outline.rows.first().item.body

    /** The file of the assets that the link in the item names, once the link is there. */
    private fun linkedAsset(): File {
        compose.waitUntil(WAIT_MS) { body().contains("../assets/") }
        val name = body().substringAfter("../assets/").substringBefore(')')
        return File(root, "markdown/assets/$name")
    }

    // クリップボードの画像を貼ると、画像は本文フォルダのassetsに写り、項目の終わりにそのリンクが入る。
    @Test
    fun an_image_of_the_clipboard_goes_into_the_assets_and_its_link_into_the_item() {
        editNotes()
        val image = TestImages.address("pasted.png")
        compose.runOnUiThread {
            context
                .getSystemService(ClipboardManager::class.java)
                .setPrimaryClip(
                    ClipData(ClipDescription("image", arrayOf("image/png")), ClipData.Item(image)),
                )
        }

        compose.onNodeWithTag(EDITOR_TAG).performSemanticsAction(SemanticsActions.PasteText)

        assertArrayEquals(TestImages.DRAWING, linkedAsset().readBytes())
        assertTrue(body(), body().startsWith("first![pasted.png]("))
    }

    // 画面のキーボードが入れた画像も、assetsに写り、項目にそのリンクが入る。
    @Test
    fun an_image_that_the_keyboard_on_the_screen_puts_in_goes_into_the_assets_and_its_link_into_the_item() {
        editNotes()
        val image = TestImages.address("keyboard.png")

        commitOnTheScreen(image)

        assertArrayEquals(TestImages.DRAWING, linkedAsset().readBytes())
        assertTrue(body(), body().startsWith("first![keyboard.png]("))
    }

    private fun commitOnTheScreen(image: Uri) {
        val root = compose.onNodeWithTag(EDITOR_TAG).fetchSemanticsNode().root as ViewRootForTest
        compose.runOnUiThread {
            val attributes = EditorInfo()
            val keyboard = checkNotNull(root.view.onCreateInputConnection(attributes))
            assertTrue(attributes.contentMimeTypes.orEmpty().contains("image/*"))
            val content = InputContentInfo(image, ClipDescription("image", arrayOf("image/png")))
            keyboard.commitContent(content, 0, null)
        }
    }
}
