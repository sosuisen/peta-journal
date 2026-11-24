package com.sosuisha.petajournal

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.ContextWrapper
import android.content.Intent
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/** How a file of the assets folder is handed to another app. No other app opens in the test. */
@RunWith(AndroidJUnit4::class)
class OtherAppTest {
    private val context: Context = InstrumentationRegistry.getInstrumentation().targetContext

    /** A folder of the test in the assets folder: the app lends only files of the assets folder. */
    private val assets = File(context.filesDir, "markdown/assets/other-app-test")

    /**
     * Starts no app. It keeps what was to be started, or it is a device without an app for the
     * file.
     */
    private class Device(base: Context, private val hasApp: Boolean = true) : ContextWrapper(base) {
        val started = mutableListOf<Intent>()

        override fun startActivity(intent: Intent) {
            if (!hasApp) throw ActivityNotFoundException()
            started += intent
        }
    }

    @Before
    fun emptyFolder() {
        assets.deleteRecursively()
        assets.mkdirs()
    }

    @After
    fun dropFolder() {
        assets.deleteRecursively()
    }

    private fun file(name: String) = File(assets, name).apply { writeText("content") }

    // ファイルは、その種類を付けて、ほかのアプリに見せる。ほかのアプリは、そのファイルを読むことだけができる。
    @Test
    fun a_file_is_shown_to_another_app_with_its_kind_and_only_to_be_read() {
        val device = Device(context)

        val shown = viewWithAnotherApp(device, file("paper.pdf"))

        assertTrue(shown)
        val view = device.started.single()
        assertEquals(Intent.ACTION_VIEW, view.action)
        assertEquals("application/pdf", view.type)
        assertEquals("content", view.data?.scheme)
        assertEquals("${context.packageName}.files", view.data?.authority)
        assertEquals(
            Intent.FLAG_GRANT_READ_URI_PERMISSION,
            view.flags and Intent.FLAG_GRANT_READ_URI_PERMISSION,
        )
        assertEquals(0, view.flags and Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
    }

    // 拡張子が大文字でも、種類は分かる。
    @Test
    fun the_kind_of_a_file_is_known_from_an_extension_in_upper_case() {
        val device = Device(context)

        viewWithAnotherApp(device, file("PAPER.PDF"))

        assertEquals("application/pdf", device.started.single().type)
    }

    // 種類の分からないファイルは、どの種類でもよいものとして見せる。
    @Test
    fun a_file_of_an_unknown_kind_is_shown_as_any_kind() {
        val device = Device(context)

        viewWithAnotherApp(device, file("data.zzzz"))

        assertEquals("*/*", device.started.single().type)
    }

    // 開けるアプリが端末になければ、落ちずに、開けなかったと答える。
    @Test
    fun a_device_without_an_app_for_the_file_answers_that_nothing_opened() {
        val shown = viewWithAnotherApp(Device(context, hasApp = false), file("paper.pdf"))

        assertFalse(shown)
    }

    // アセットのフォルダーの外のファイルは、ほかのアプリに貸さない。
    @Test
    fun a_file_outside_the_assets_folder_is_not_lent() {
        val device = Device(context)
        val page = File(context.filesDir, "markdown/docs/other-app-test.md")

        assertThrows(IllegalArgumentException::class.java) { viewWithAnotherApp(device, page) }

        assertEquals(emptyList<Intent>(), device.started)
    }
}
