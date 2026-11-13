package net.sosuisen.petajournal

import android.os.Bundle
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import java.io.File
import net.sosuisen.petajournal.screen.JournalScreen

class MainActivity : ComponentActivity() {
    private val model: JournalModel by viewModels { JournalModel.factory }

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
                onFile = ::open,
            )
        }
    }

    /** Opens a file of the assets folder with another app, or tells that the device has none for it. */
    private fun open(file: File) {
        if (!viewWithAnotherApp(this, file)) Toast.makeText(this, "No app opens ${file.name}", Toast.LENGTH_LONG).show()
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
