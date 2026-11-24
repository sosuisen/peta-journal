package com.sosuisha.petajournal.shell

import java.io.File
import java.io.InputStream
import java.util.UUID

/**
 * The file that a pasted image is written to. The core saves it among the assets and then deletes
 * it (ADR 071): an image of the clipboard is only a `content://` address, which Rust cannot read.
 */
object PastedImage {
    /**
     * Writes [content] into a folder of its own under [cache], and gives the file. [name] is the
     * name the image came with, and [type] its media type, such as `image/png`. The core takes the
     * name for the text of the link, and the end of the name for that of the saved file.
     */
    fun write(content: InputStream, cache: File, name: String?, type: String?): File {
        // A folder of its own: an image of the same name may be pasted before the first is saved
        val folder = File(cache, "pasted/${UUID.randomUUID()}").apply { mkdirs() }
        val file = File(folder, name?.takeIf { it.isNotBlank() } ?: nameOf(type))
        content.use { input -> file.outputStream().use { input.copyTo(it) } }
        return file
    }

    /** A name for an image that came without one. */
    private fun nameOf(type: String?): String {
        val extension =
            when (type) {
                "image/jpeg" -> "jpg"
                null -> null
                else -> type.substringAfter("image/", "").takeIf { it.isNotEmpty() }
            }
        return extension?.let { "image.$it" } ?: "image"
    }
}
