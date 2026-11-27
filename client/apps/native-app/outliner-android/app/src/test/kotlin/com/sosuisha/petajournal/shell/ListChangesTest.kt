package com.sosuisha.petajournal.shell

import com.sosuisha.petajournal.types.Changes
import com.sosuisha.petajournal.types.Row
import java.io.File
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test

/**
 * The examples of the core that every shell applies as the core does (ADR 089, ADR 090 D1). They
 * are in `client/tools/shell-check/common/list-changes.json`, which the core writes: a list before,
 * the changes, and the list after, each as the bytes of a `Changes` in hexadecimal.
 */
class ListChangesTest {
    /** One example: the rows before, the changes and the rows after, as bytes in hexadecimal. */
    private class Example(
        val id: String,
        val before: String,
        val changes: String,
        val after: String,
    )

    /** The examples of the rows of the border of a native shell. */
    private fun examples(): List<Example> {
        val array: JSONArray =
            JSONObject(examplesFile().readText()).getJSONObject("native").getJSONArray("rows")
        return (0 until array.length()).map {
            val example = array.getJSONObject(it)
            Example(
                id = example.getString("id"),
                before = example.getString("before"),
                changes = example.getString("changes"),
                after = example.getString("after"),
            )
        }
    }

    /**
     * The file of the examples, found in a folder above the one Gradle runs the tests in: the
     * folder of the app module.
     */
    private fun examplesFile(): File =
        generateSequence(File("").absoluteFile) { it.parentFile }
            .map { File(it, "tools/shell-check/common/list-changes.json") }
            .first { it.isFile }

    /** The bytes in the hexadecimal `hex`. */
    private fun bytes(hex: String): ByteArray =
        hex.chunked(2).map { it.toInt(16).toByte() }.toByteArray()

    /** The rows that the changes in `hex` make of these rows, as the shell makes them. */
    private fun List<Row>.applied(hex: String): List<Row> =
        applied(Changes.bincodeDeserialize(bytes(hex)).rows)

    /** The rows as the bytes that cross the border, in hexadecimal. */
    private fun hex(rows: List<Row>): List<String> = rows.map { row ->
        row.bincodeSerialize().joinToString("") { "%02x".format(it) }
    }

    /**
     * Every example of the core, applied by the shell, is the rows of the core. The examples that
     * ran are all those of the file.
     */
    @Test
    fun theShellAppliesEveryExampleOfTheCoreAsTheCoreDoes() {
        val examples = examples()

        val ran = examples.map { example ->
            val before = emptyList<Row>().applied(example.before)
            val after = emptyList<Row>().applied(example.after)

            val applied = before.applied(example.changes)

            assertEquals(example.id, hex(after), hex(applied))
            example.id
        }

        assertFalse(ran.isEmpty())
        assertEquals(examples.map { it.id }, ran)
    }
}
