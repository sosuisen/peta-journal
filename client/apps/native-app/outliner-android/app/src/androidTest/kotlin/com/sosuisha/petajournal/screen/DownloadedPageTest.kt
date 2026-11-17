package com.sosuisha.petajournal.screen

import androidx.test.ext.junit.runners.AndroidJUnit4
import java.io.File
import org.junit.runner.RunWith
import org.junit.Test

/** A page that a download changed while it is open. */
@RunWith(AndroidJUnit4::class)
class DownloadedPageTest : JournalScreenFixture() {
    // 開いているページのファイルがダウンロードで変わると、取り込んだあと、触れなくても画面に出る。
    @Test
    fun an_open_page_that_a_download_changed_shows_the_change_without_a_touch() {
        download("notes.md", "first" + NL)
        openNotes()
        waitFor("first")

        download("notes.md", "changed elsewhere" + NL)
        // Later than the file that the core took in, as a download that comes after is
        val file = File(root, "markdown/docs/notes.md")
        file.setLastModified(file.lastModified() + 10_000)
        compose.runOnUiThread { shell.sync() }

        waitFor("changed elsewhere")
    }

    // ほかの端末が項目の本文だけを書き換えたファイルでも、取り込んだあと、触れなくても画面に出る。項目のIDは変わらない。
    @Test
    fun an_open_page_whose_item_changed_elsewhere_shows_the_change_without_a_touch() {
        download("notes.md", "first" + NL + "second" + NL)
        openNotes()
        waitFor("second")
        val file = File(root, "markdown/docs/notes.md")
        // The core writes the ids of the items above them when it has taken the file in
        compose.waitUntil(WAIT_MS) { file.readText().contains("<!--") }

        file.writeText(file.readText().replace("second", "changed elsewhere"))
        file.setLastModified(file.lastModified() + 10_000)
        compose.runOnUiThread { shell.sync() }

        waitFor("changed elsewhere")
    }
}
