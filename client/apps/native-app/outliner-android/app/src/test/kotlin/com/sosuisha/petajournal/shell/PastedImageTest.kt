package com.sosuisha.petajournal.shell

import java.io.File
import java.nio.file.Files
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Test

/** The file that a pasted image is written to, for the core to save it (ADR 071). */
class PastedImageTest {
    private val cache: File = Files.createTempDirectory("pasted-image-test").toFile()

    @After
    fun deleteTheCache() {
        cache.deleteRecursively()
    }

    private fun write(name: String?, type: String?, content: ByteArray = DRAWING) =
        PastedImage.write(content.inputStream(), cache, name, type)

    // 名前のある画像は、その名前と中身のファイルに書かれる。
    @Test
    fun an_image_with_a_name_is_written_to_a_file_of_that_name() {
        val file = write("pasted.png", "image/png")

        assertEquals("pasted.png", file.name)
        assertArrayEquals(DRAWING, file.readBytes())
    }

    // 名前のない画像は、種類から拡張子を決めた名前のファイルになる。
    @Test
    fun an_image_without_a_name_is_named_by_its_type() {
        assertEquals("image.png", write(null, "image/png").name)
        assertEquals("image.jpg", write(null, "image/jpeg").name)
        assertEquals("image", write(null, null).name)
    }

    // 種類が画像でなければ、名前は拡張子のない image である。
    @Test
    fun an_image_whose_type_is_not_an_image_is_named_image_with_no_extension() {
        assertEquals("image", write(null, "application/octet-stream").name)
    }

    // 同じ名前の2つの画像は、別々のファイルに書かれる。前の画像の保存が終わる前に、次を貼れるためである。
    @Test
    fun two_images_of_one_name_are_written_to_two_files() {
        val first = write("pasted.png", "image/png", "first".toByteArray())
        val second = write("pasted.png", "image/png", "second".toByteArray())

        assertNotEquals(first, second)
        assertEquals("first", first.readText())
        assertEquals("second", second.readText())
    }

    private companion object {
        val DRAWING =
            byteArrayOf(0x89.toByte(), 'P'.code.toByte(), 'N'.code.toByte(), 'G'.code.toByte())
    }
}
