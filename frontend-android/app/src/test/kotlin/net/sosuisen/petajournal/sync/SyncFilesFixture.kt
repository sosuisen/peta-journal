package net.sosuisen.petajournal.sync

import java.io.File
import org.junit.Before
import org.junit.Rule
import org.junit.rules.TemporaryFolder

/** What the tests of the download and the upload start from: a cloud of a test, and the folder of the app. */
abstract class SyncFilesFixture {
    @get:Rule
    val dir = TemporaryFolder()

    protected val cloud = FakeCloud()
    protected lateinit var root: File
    protected lateinit var downloader: Downloader
    protected lateinit var uploader: Uploader

    @Before
    fun folders() {
        root = dir.newFolder("markdown")
        open()
    }

    /** Makes the parts of the sync, as the app does when it opens. */
    protected fun open() {
        val folder = MarkdownFolder(root, File(dir.root, "temp"))
        val record = CloudRecord(File(dir.root, "record"))
        downloader = Downloader(cloud, folder, File(dir.root, "cursor"), record)
        uploader = Uploader(cloud, folder, record)
    }

    /** Downloads the files, as the first sync of the app does. */
    protected fun download(vararg files: Pair<String, String>) {
        cloud.answers[null] = Changes(files.map { (path, text) -> cloud.file(path, text) }, "c1", whole = true)
        downloader.run { _, _ -> }
    }

    /** Downloads what changed in the cloud after the cursor [since]. */
    protected fun download(since: String, next: String, whole: Boolean = false, vararg changes: Change) {
        cloud.answers[since] = Changes(changes.toList(), next, whole)
        downloader.run { _, _ -> }
    }

    /** Writes the file of a page, as the core does after an edit in the app. */
    protected fun write(path: String, text: String) {
        File(root, path).apply { parentFile.mkdirs() }.writeText(text)
    }

    protected fun upload() = uploader.run { _, _ -> }

    /** The paths of the files under the Markdown folder. */
    protected fun here(): List<String> =
        root.walk().filter { it.isFile }.map { it.relativeTo(root).invariantSeparatorsPath }.sorted().toList()
}
