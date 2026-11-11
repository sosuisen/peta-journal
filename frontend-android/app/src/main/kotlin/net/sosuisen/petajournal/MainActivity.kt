package net.sosuisen.petajournal

import android.app.Application
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import androidx.lifecycle.AndroidViewModel
import java.io.File

/**
 * Keeps the shell while the screen is made again, as when the device is turned. The journal
 * can be open only once, and closes when the app is left for good.
 */
class JournalModel(app: Application) : AndroidViewModel(app) {
    private val main = Handler(Looper.getMainLooper())

    // The folders of ADR 030. Kotlin makes the folders of the pages: the core takes in
    // nothing from a Markdown folder that has neither of them
    private val markdown = File(app.filesDir, "markdown").also { root ->
        for (name in listOf("docs", "journals", "assets")) File(root, name).mkdirs()
    }

    val shell = Shell(File(app.filesDir, "data"), markdown) { main.post(it) }

    init {
        shell.sync()
    }

    override fun onCleared() = shell.close()
}

class MainActivity : ComponentActivity() {
    private val model: JournalModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent { JournalScreen(model.shell) }
    }
}
