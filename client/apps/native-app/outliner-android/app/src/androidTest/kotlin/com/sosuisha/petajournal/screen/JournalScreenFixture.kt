package com.sosuisha.petajournal.screen

import android.content.ClipData
import android.content.ClipboardManager
import android.os.Handler
import android.os.Looper
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.UriHandler
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.TouchInjectionScope
import androidx.compose.ui.test.click
import androidx.compose.ui.test.hasContentDescription
import androidx.compose.ui.test.hasSetTextAction
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onFirst
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.percentOffset
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.test.performTouchInput
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.sosuisha.petajournal.core.PetaDropboxSdk
import com.sosuisha.petajournal.shell.Shell
import com.sosuisha.petajournal.sync.CloudSync
import java.io.File
import org.junit.After
import org.junit.Before
import org.junit.Rule
import org.junit.runner.RunWith

/**
 * What the tests of the screen start from: the screen with a real core under it, on the folders of
 * a test.
 */
@RunWith(AndroidJUnit4::class)
abstract class JournalScreenFixture {
    @get:Rule val compose = createComposeRule()

    protected lateinit var root: File
    protected lateinit var shell: Shell

    /**
     * What the shell put on the clipboard, the last last. It is on the clipboard of the system too.
     */
    protected val copied = mutableListOf<String>()

    private fun copy(text: String) {
        copied.add(text)
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        context
            .getSystemService(ClipboardManager::class.java)
            .setPrimaryClip(ClipData.newPlainText("test", text))
    }

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
     * Puts a doc page into the Markdown folder, as a download does. A file without the lines that
     * the core writes above each item is one item, with the whole text as its body.
     */
    protected fun download(name: String, text: String) =
        File(root, "markdown/docs/$name").writeText(text)

    /**
     * Opens the app on the folders, and waits until the files are taken in. With a cloud, the app
     * syncs with it when "Sync" is touched; [signedIn] says whether the user has signed in. It
     * syncs after an edit only once no page has changed for [afterChangeMs].
     */
    protected fun start(
        cloud: PetaDropboxSdk? = null,
        signedIn: Boolean = true,
        afterChangeMs: Long = 600_000,
    ) {
        val main = Handler(Looper.getMainLooper())
        compose.runOnUiThread {
            shell =
                Shell(File(root, "data"), File(root, "markdown"), { main.post(it) }, copy = ::copy)
            shell.sync()
        }
        sync = cloud?.let {
            CloudSync(
                shell,
                // Most tests of the screen sync only when "Sync" is touched
                afterChangeMs,
                File(root, "temp"),
                File(root, "cursor"),
                File(root, "record"),
                signedIn,
                onMainThread = { main.post(it) },
            )
        }
        val browser =
            object : UriHandler {
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
                    onSync = { sync?.start(checkNotNull(cloud), FOLDER) },
                    onSignOut = {
                        signOuts += 1
                        sync?.signOut()
                    },
                    onFile = { opened += it },
                )
                // Read here, so that this is drawn again with the screen whenever the view changes
                shell.view
                SideEffect { afterDrawn() }
            }
        }
        compose.waitUntil(WAIT_MS) { !shell.syncing }
    }

    /**
     * Runs on the main thread each time the screen is drawn again after a change of the view,
     * before the effects of that drawing start. On a slow phone the keyboard may come in there.
     */
    protected var afterDrawn: () -> Unit = {}

    protected var signIns = 0
    protected var signOuts = 0
    protected val opened = mutableListOf<File>()

    /** The web addresses that the screen asked the browser to open. */
    protected val browsed = mutableListOf<String>()
    protected var sync: CloudSync? = null

    protected fun type(query: String) = compose.onNodeWithTag(SEARCH_TAG).performTextInput(query)

    /** A text on the screen that is not what the search field holds. */
    protected fun shown(text: String): SemanticsMatcher =
        hasText(text, substring = true) and !hasSetTextAction()

    protected fun waitFor(text: String) =
        compose.waitUntil(WAIT_MS) {
            compose.onAllNodes(shown(text)).fetchSemanticsNodes().isNotEmpty()
        }

    /** Opens the page `notes`, which a test has put into the Markdown folder. */
    protected fun openNotes() {
        start()
        type("notes")
        waitFor("notes")
        compose.onAllNodes(shown("notes")).onFirst().performClick()
    }

    /**
     * Touches the pad [name] near the edge of [arrow], as a thumb does: not on the drawn arrow, but
     * in the part of the pad that belongs to it.
     */
    internal fun pad(name: String, arrow: Arrow) =
        compose.onNode(hasContentDescription(name)).performTouchInput { click(nearEdge(arrow)) }

    /** A place of a pad near the edge of [arrow], away from the drawn arrow. */
    internal fun TouchInjectionScope.nearEdge(arrow: Arrow): Offset =
        when (arrow) {
            Arrow.Left -> percentOffset(0.05f, 0.2f)
            Arrow.Right -> percentOffset(0.95f, 0.8f)
            Arrow.Up -> percentOffset(0.8f, 0.05f)
            Arrow.Down -> percentOffset(0.2f, 0.95f)
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
