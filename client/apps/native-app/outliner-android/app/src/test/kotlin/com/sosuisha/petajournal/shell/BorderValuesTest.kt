package com.sosuisha.petajournal.shell

import com.sosuisha.petajournal.types.Changes
import com.sosuisha.petajournal.types.Event
import java.io.File
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test

/**
 * The examples of the core of the values that cross the border of a native shell (ADR 090 D2). They
 * are in `client/tools/shell-check/common/border-native.json`, which the core writes: every variant
 * of `Changes` and `Event`, as their bytes in hexadecimal.
 */
class BorderValuesTest {
    /** One example: a value of the border, as its bytes in hexadecimal. */
    private class Example(val id: String, val bytes: String)

    /** The examples of the section `section` of the file of the examples. */
    private fun examples(section: String): List<Example> {
        val array: JSONArray = JSONObject(examplesFile().readText()).getJSONArray(section)
        return (0 until array.length()).map {
            val example = array.getJSONObject(it)
            Example(example.getString("id"), example.getString("bytes"))
        }
    }

    /**
     * The file of the examples, found in a folder above the one Gradle runs the tests in: the
     * folder of the app module.
     */
    private fun examplesFile(): File =
        generateSequence(File("").absoluteFile) { it.parentFile }
            .map { File(it, "tools/shell-check/common/border-native.json") }
            .first { it.isFile }

    /** The bytes in the hexadecimal `hex`. */
    private fun bytes(hex: String): ByteArray =
        hex.chunked(2).map { it.toInt(16).toByte() }.toByteArray()

    /**
     * Reads each example as the shell reads the border, writes it back as the shell writes it, and
     * expects the bytes it began with. Gives the ids of the examples it ran.
     */
    private fun readAndWriteBack(
        examples: List<Example>,
        readAndWrite: (ByteArray) -> ByteArray,
    ): List<String> = examples.map { example ->
        val read = bytes(example.bytes)

        val written = readAndWrite(read)

        assertArrayEquals(example.id, read, written)
        example.id
    }

    /**
     * Every example of `Changes`, read and written back by the generated types, is the bytes it
     * was. The examples that ran are all those of the file.
     */
    @Test
    fun theShellReadsAndWritesBackEveryExampleOfTheChangesAsItWas() {
        val examples = examples("changes")

        val ran = readAndWriteBack(examples) { Changes.bincodeDeserialize(it).bincodeSerialize() }

        assertFalse(ran.isEmpty())
        assertEquals(examples.map { it.id }, ran)
    }

    /**
     * Every example of `Event`, read and written back by the generated types, is the bytes it was.
     * The examples that ran are all those of the file.
     */
    @Test
    fun theShellReadsAndWritesBackEveryExampleOfTheEventsAsItWas() {
        val examples = examples("events")

        val ran = readAndWriteBack(examples) { Event.bincodeDeserialize(it).bincodeSerialize() }

        assertFalse(ran.isEmpty())
        assertEquals(examples.map { it.id }, ran)
    }
}
