package com.sosuisha.petajournal.shell

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.sosuisha.petajournal.types.Event
import com.sosuisha.petajournal.types.Panel
import java.io.File
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/** The changes of the screen that the core sends, instead of the whole view model (ADR 076). */
@RunWith(AndroidJUnit4::class)
class ChangesTest {
    private lateinit var root: File
    private lateinit var border: Border

    @Before
    fun startTheCore() {
        val cache = InstrumentationRegistry.getInstrumentation().targetContext.cacheDir
        root = File(cache, "changes-test").apply { deleteRecursively() }
        File(root, "markdown").mkdirs()
        border = Border(File(root, "data"), File(root, "markdown")) {}
    }

    @After fun closeTheCore() = border.close()

    // Rustが書いた画面の差分を、Kotlinが読める。最初の差分には、Kotlinが送ったイベントの中身が入っている。
    @Test
    fun the_changes_of_the_screen_written_by_rust_are_read_in_kotlin() {
        border.send(Event.SearchQueryChanged("apple 日本語"))

        val search = border.changes().panels.filterIsInstance<Panel.Search>().single()
        assertEquals("apple 日本語", search.value.query)
    }
}
