package com.sosuisha.petajournal

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.webkit.MimeTypeMap
import androidx.core.content.FileProvider
import java.io.File

/**
 * Opens a file of the assets folder with the app that the device has for its kind. The other app
 * may only read the file, and only this one. Answers false when the device has no app for the file.
 *
 * @param from what starts the other app: the activity.
 */
internal fun viewWithAnotherApp(from: Context, file: File): Boolean {
    val lent = FileProvider.getUriForFile(from, "${from.packageName}.files", file)
    val kind =
        MimeTypeMap.getSingleton().getMimeTypeFromExtension(file.extension.lowercase()) ?: "*/*"
    val view =
        Intent(Intent.ACTION_VIEW)
            .setDataAndType(lent, kind)
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
    return try {
        from.startActivity(view)
        true
    } catch (none: ActivityNotFoundException) {
        false
    }
}
