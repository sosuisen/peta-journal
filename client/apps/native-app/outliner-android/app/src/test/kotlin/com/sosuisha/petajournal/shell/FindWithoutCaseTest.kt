package com.sosuisha.petajournal.shell

import java.nio.file.Files
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Before
import org.junit.Test

/** How the shell finds the file of an image, whatever case its name has. */
class FindWithoutCaseTest {
    private val root = Files.createTempDirectory("find").toFile()

    @Before
    fun putAnImage() {
        root.resolve("assets").mkdirs()
        root.resolve("assets/Picture.PNG").writeText("an image")
    }

    // 画像のファイルは、名前の大文字と小文字を区別せずに見つかる。
    @Test
    fun the_file_of_an_image_is_found_without_regard_to_case() {
        assertEquals("Picture.PNG", findWithoutCase(root, "ASSETS/picture.png")?.name)
    }

    // 空の区切りは飛ばす。先頭が「/」のパスでも、iOSと同じく見つかる。
    @Test
    fun empty_parts_of_the_path_are_skipped_as_on_ios() {
        assertEquals("Picture.PNG", findWithoutCase(root, "/assets//picture.png")?.name)
    }

    // ないファイルは見つからない。
    @Test
    fun a_file_that_is_not_there_is_not_found() {
        assertNull(findWithoutCase(root, "assets/other.png"))
    }
}
