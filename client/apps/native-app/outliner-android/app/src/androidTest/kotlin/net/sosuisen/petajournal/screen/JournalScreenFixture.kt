package net.sosuisen.petajournal.screen

import android.os.Handler
import android.os.Looper
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.UriHandler
import androidx.compose.ui.test.hasSetTextAction
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onFirst
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.test.SemanticsMatcher
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import net.sosuisen.petajournal.shell.Shell
import net.sosuisen.petajournal.sync.Cloud
import net.sosuisen.petajournal.sync.CloudRecord
import net.sosuisen.petajournal.sync.CloudSync
import net.sosuisen.petajournal.sync.Downloader
import net.sosuisen.petajournal.sync.MarkdownFolder
import net.sosuisen.petajournal.sync.SyncRound
import net.sosuisen.petajournal.sync.Uploader
import org.junit.After
import org.junit.Before
import org.junit.Rule
import org.junit.runner.RunWith

/**
 * What the tests of the screen start from: the screen with a real core under it, on the
 * folders of a test.
 */
@RunWith(AndroidJUnit4::class)
abstract class JournalScreenFixture {
    @get:Rule
    val compose = createComposeRule()

    protected lateinit var root: File
    protected lateinit var shell: Shell

    @Before
    fun emptyFolders() {
        val cache = InstrumentationRegistry.getInstrumentation().targetContext.cacheDir
        root = File(cache, "journal-test").apply { deleteRecursively() }
        File(root, "markdown/docs").mkdirs()
    }

    @After
    fun close() {
        compose.runOnUiThread { shell.close() }
    }

    /**
     * Puts a doc page into the Markdown folder, as a download does. A file without the lines
     * that the core writes above each item is one item, with the whole text as its body.
     */
    protected fun download(name: String, text: String) = File(root, "markdown/docs/$name").writeText(text)

    /**
     * Opens the app on the folders, and waits until the files are taken in. With a cloud, the
     * app syncs with it when "Sync" is touched; [signedIn] says whether the user has signed in.
     */
    protected fun start(cloud: Cloud? = null, signedIn: Boolean = true) {
        val main = Handler(Looper.getMainLooper())
        compose.runOnUiThread {
            shell = Shell(File(root, "data"), File(root, "markdown"), { main.post(it) }) { edits += 1 }
            shell.sync()
        }
        val folder = MarkdownFolder(File(root, "markdown"), File(root, "temp"))
        val record = CloudRecord(File(root, "record"))
        sync = cloud?.let {
            val round = { from: Cloud ->
                SyncRound(Downloader(from, folder, File(root, "cursor"), record), shell::syncAndWait, Uploader(from, folder, record))
            }
            CloudSync(signedIn, round) { main.post(it) }
        }
        val browser = object : UriHandler {
            override fun openUri(uri: String) {
                browsed += uri
            }
        }
        compose.setContent {
            CompositionLocalProvider(LocalUriHandler provides browser) {
                JournalScreen(
                    shell,
                    sync?.state,
                    onSignIn = { signIns += 1 },
                    onSync = { sync?.start(checkNotNull(cloud)) },
                    onSignOut = {
                        signOuts += 1
                        sync?.signOut()
                    },
                    onFile = { opened += it },
                )
            }
        }
        compose.waitUntil(WAIT_MS) { !shell.syncing }
    }

    /** How often the shell told that the user changed a page. */
    protected var edits = 0
    protected var signIns = 0
    protected var signOuts = 0
    protected val opened = mutableListOf<File>()

    /** The web addresses that the screen asked the browser to open. */
    protected val browsed = mutableListOf<String>()
    protected var sync: CloudSync? = null

    protected fun type(query: String) = compose.onNodeWithTag(SEARCH_TAG).performTextInput(query)

    /** A text on the screen that is not what the search field holds. */
    protected fun shown(text: String): SemanticsMatcher = hasText(text, substring = true) and !hasSetTextAction()

    protected fun waitFor(text: String) =
        compose.waitUntil(WAIT_MS) { compose.onAllNodes(shown(text)).fetchSemanticsNodes().isNotEmpty() }

    /** Opens the page `notes`, which a test has put into the Markdown folder. */
    protected fun openNotes() {
        start()
        type("notes")
        waitFor("notes")
        compose.onAllNodes(shown("notes")).onFirst().performClick()
    }

    /** The line that the core writes above an item in a file, with the properties of the item. */
    protected fun above(id: Char, properties: String = "") =
        "<!-- @ 2020-01-01-00-00-00-AAAAAA$id idx=\"a${id}\"$properties -->" + NL

    /** Lets the core write the pages that changed, and says whether a file is at the path then. */
    protected fun isWritten(path: String): Boolean {
        compose.runOnUiThread { shell.sync() }
        compose.waitUntil(WAIT_MS) { !shell.syncing }
        return File(root, "markdown/$path").isFile
    }
}

/** The search waits a moment after the last key, and the first sync opens the index. */
internal const val WAIT_MS = 10_000L
internal const val NL = "\n"
