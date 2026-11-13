package com.sosuisha.petajournal.dropbox

import com.sosuisha.petajournal.sync.hex
import com.sosuisha.petajournal.sync.sha256
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class ContentHashTest {
    @get:Rule
    val dir = TemporaryFolder()

    // 4MBを超えるファイルのハッシュは、4MBごとのハッシュをつないだもののハッシュである。
    @Test
    fun the_content_hash_of_a_large_file_is_made_of_the_hashes_of_its_blocks() {
        val block = 4 * 1024 * 1024
        val content = ByteArray(block + 3) { (it % 251).toByte() }
        val file = dir.newFile("large").apply { writeBytes(content) }
        val first = sha256(content.copyOfRange(0, block))
        val second = sha256(content.copyOfRange(block, content.size))

        assertEquals(hex(sha256(first + second)), dropboxContentHash(file))
    }

    // 4MBまでのファイルのハッシュは、中身のハッシュの、そのまたハッシュである。
    @Test
    fun the_content_hash_of_a_small_file_is_the_hash_of_the_hash_of_its_content() {
        val file = dir.newFile("small").apply { writeText("- a") }

        assertEquals(hex(sha256(sha256("- a".toByteArray()))), dropboxContentHash(file))
    }

    // 空のファイルのハッシュは、空の中身のハッシュである。
    @Test
    fun the_content_hash_of_an_empty_file_is_the_hash_of_nothing() {
        assertEquals(hex(sha256(ByteArray(0))), dropboxContentHash(dir.newFile("empty")))
    }
}
