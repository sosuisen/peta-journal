package com.sosuisha.petajournal.screen

import android.content.ContentProvider
import android.content.ContentValues
import android.database.Cursor
import android.database.MatrixCursor
import android.net.Uri
import android.os.ParcelFileDescriptor
import android.provider.OpenableColumns
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File

/**
 * The images of the tests, lent by a `content://` address as another app lends the image it
 * put on the clipboard. The files are in the cache of the app: the tests run in its process.
 */
class TestImages : ContentProvider() {
    override fun onCreate() = true

    override fun getType(uri: Uri) = "image/png"

    override fun openFile(uri: Uri, mode: String): ParcelFileDescriptor =
        ParcelFileDescriptor.open(fileOf(uri), ParcelFileDescriptor.MODE_READ_ONLY)

    override fun query(uri: Uri, projection: Array<out String>?, selection: String?, args: Array<out String>?, order: String?): Cursor =
        MatrixCursor(arrayOf(OpenableColumns.DISPLAY_NAME)).apply { addRow(arrayOf(uri.lastPathSegment)) }

    override fun insert(uri: Uri, values: ContentValues?): Uri? = null

    override fun delete(uri: Uri, selection: String?, args: Array<out String>?) = 0

    override fun update(uri: Uri, values: ContentValues?, selection: String?, args: Array<out String>?) = 0

    companion object {
        private const val AUTHORITY = "com.sosuisha.petajournal.test.images"

        private val folder: File
            get() = File(InstrumentationRegistry.getInstrumentation().targetContext.cacheDir, "test-images")

        private fun fileOf(uri: Uri) = File(folder, checkNotNull(uri.lastPathSegment))

        /** Lends an image of [content] under [name], and gives its address. */
        fun lend(name: String, content: ByteArray): Uri {
            folder.mkdirs()
            File(folder, name).writeBytes(content)
            return Uri.parse("content://$AUTHORITY/$name")
        }
    }
}
