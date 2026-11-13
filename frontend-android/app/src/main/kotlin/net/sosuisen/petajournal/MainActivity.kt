package net.sosuisen.petajournal

import android.content.ActivityNotFoundException
import android.content.Intent
import android.os.Bundle
import android.webkit.MimeTypeMap
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import androidx.core.content.FileProvider
import java.io.File

class MainActivity : ComponentActivity() {
    private val model: JournalModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            JournalScreen(
                model.shell,
                model.cloud?.state,
                syncInFront = model.cloud?.inFront == true,
                onSignIn = { model.signIn(this) },
                onSync = { model.sync() },
                onSignOut = { model.signOut() },
                onFile = ::openWithAnotherApp,
            )
        }
    }

    /**
     * Opens a file of the assets folder with the app that the device has for its kind. The
     * other app may only read the file, and only this one.
     */
    private fun openWithAnotherApp(file: File) {
        val lent = FileProvider.getUriForFile(this, "$packageName.files", file)
        val kind = MimeTypeMap.getSingleton().getMimeTypeFromExtension(file.extension.lowercase()) ?: "*/*"
        val view = Intent(Intent.ACTION_VIEW).setDataAndType(lent, kind).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        try {
            startActivity(view)
        } catch (none: ActivityNotFoundException) {
            Toast.makeText(this, "No app opens ${file.name}", Toast.LENGTH_LONG).show()
        }
    }

    /**
     * The app syncs when it goes to the back: what was written goes up before Android may
     * stop the app. A sync that Android stops goes on when the app opens again.
     */
    override fun onStop() {
        super.onStop()
        // Not when the screen is only made again, as when the device is turned
        if (!isChangingConfigurations) model.sync()
    }

    /** The app syncs whenever it comes to the front, also back from the sign-in. */
    override fun onResume() {
        super.onResume()
        model.syncInFront()
    }
}
