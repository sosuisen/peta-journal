package com.sosuisha.petajournal.shell

import com.sosuisha.petajournal.types.Item
import com.sosuisha.petajournal.types.ListChange
import com.sosuisha.petajournal.types.RichText
import com.sosuisha.petajournal.types.Row
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * The rows that the changes of the core make (ADR 076). That the changes make the screen of the
 * core is tested in Rust, in `client-native-bridge`.
 */
class AppliedTest {
    private fun row(id: String, body: String = "") =
        Row(
            item =
                Item(
                    id = id,
                    pageId = "page",
                    dirPath = "",
                    name = "",
                    itemType = "",
                    createdDate = 0,
                    modifiedDate = 0,
                    idx = "",
                    level = 1,
                    body = body,
                    collapsed = false,
                    types = emptyList(),
                    properties = emptyMap(),
                ),
            bodyModel = RichText(emptyList()),
            position = 0u,
            indentPx = 0,
            isEditing = false,
            isSelected = false,
            isCollapsed = false,
            hasChildren = false,
            headingLevel = 0u,
            isHeading = false,
            headingLine = false,
            hasOwnZoom = false,
            isTransclusion = false,
            isBrokenTransclusion = false,
            showBullet = true,
            effectiveZoom = 1.0,
            suggestionsVisible = false,
        )

    private fun ids(rows: List<Row>) = rows.map { it.item.id }

    // 行の操作は順に当たる。挿入、削除、移動、更新の位置は、その前の操作を当てたあとの行で数える。
    @Test
    fun the_changes_are_applied_in_order_and_each_place_counts_after_the_changes_before() {
        val rows = listOf(row("a"), row("b"), row("c"))

        val applied =
            rows.applied(
                listOf(
                    ListChange.Insert(1u, row("x")),
                    ListChange.Remove(0u),
                    ListChange.Move(2u, 0u),
                    ListChange.Update(1u, row("x", "changed")),
                ),
            )

        assertEquals(listOf("c", "x", "b"), ids(applied))
        assertEquals("changed", applied[1].item.body)
    }

    // 入れ替えは、前の行をすべて捨てて、届いた行にする。
    @Test
    fun a_reset_puts_the_rows_that_came_in_place_of_all_before() {
        val rows = listOf(row("a"), row("b"))

        val applied = rows.applied(listOf(ListChange.Reset(listOf(row("y"), row("z")))))

        assertEquals(listOf("y", "z"), ids(applied))
    }
}
