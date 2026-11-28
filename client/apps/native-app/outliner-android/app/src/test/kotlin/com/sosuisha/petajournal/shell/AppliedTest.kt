package com.sosuisha.petajournal.shell

import com.sosuisha.petajournal.types.CalendarView
import com.sosuisha.petajournal.types.Changes
import com.sosuisha.petajournal.types.ConfigView
import com.sosuisha.petajournal.types.ConnectionState
import com.sosuisha.petajournal.types.Heading
import com.sosuisha.petajournal.types.HeadingChange
import com.sosuisha.petajournal.types.HeadingsPanel
import com.sosuisha.petajournal.types.HistoryView
import com.sosuisha.petajournal.types.IndexPagesPanel
import com.sosuisha.petajournal.types.IndexSort
import com.sosuisha.petajournal.types.IndexSortKey
import com.sosuisha.petajournal.types.Item
import com.sosuisha.petajournal.types.LoadState
import com.sosuisha.petajournal.types.OutlinePanel
import com.sosuisha.petajournal.types.Page
import com.sosuisha.petajournal.types.PageChange
import com.sosuisha.petajournal.types.Panel
import com.sosuisha.petajournal.types.RecentPagesPanel
import com.sosuisha.petajournal.types.RichText
import com.sosuisha.petajournal.types.Row
import com.sosuisha.petajournal.types.RowChange
import com.sosuisha.petajournal.types.ScreenView
import com.sosuisha.petajournal.types.SearchChange
import com.sosuisha.petajournal.types.SearchEntry
import com.sosuisha.petajournal.types.SearchKind
import com.sosuisha.petajournal.types.SearchPanel
import com.sosuisha.petajournal.types.SidebarPage
import com.sosuisha.petajournal.types.SortOrder
import com.sosuisha.petajournal.types.WeekNumbering
import com.sosuisha.petajournal.types.WikiView
import org.junit.Assert.assertEquals
import org.junit.Test

/** The changes of the rows and the panels, with no change of the headings. */
private fun Changes(rows: List<RowChange>, panels: List<Panel>) =
    Changes(rows, emptyList(), emptyList(), emptyList(), emptyList(), panels)

/**
 * The view model that the changes of the core make (ADR 076). That the core sends the changes that
 * make its own view model is tested in Rust, in `client-native-bridge`.
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
            firstLineScale = 1.0f,
            hasOwnZoom = false,
            isTransclusion = false,
            isBrokenTransclusion = false,
            showBullet = true,
            effectiveZoom = 1.0,
            suggestionsVisible = false,
        )

    private fun ids(rows: List<Row>) = rows.map { it.item.id }

    /** The outline panel, with `selectionMode` to tell one from another. */
    private fun outline(selecting: Boolean) =
        OutlinePanel(
            selectionMode = selecting,
            keyRelayHolding = false,
            wiki = WikiView(suggestions = emptyList(), selectedIndex = 0u, x = 0.0, y = 0.0),
            scrolledFromTop = false,
        )

    private fun search(query: String) =
        SearchPanel(query = query, showDropdown = false, searching = false)

    /**
     * Every panel once, as the first changes of a core hold them. Each panel has a value of its
     * own, so a panel put in the place of another is seen.
     */
    private fun everyPanel(): List<Panel> =
        listOf(
            Panel.Screen(ScreenView.Settings),
            Panel.AtlasCards(emptyList()),
            Panel.AtlasCallouts(emptyList()),
            Panel.AtlasLinks(emptyList()),
            Panel.AtlasUnplaced(emptyList()),
            Panel.Outline(outline(selecting = true)),
            Panel.IndexPages(
                IndexPagesPanel(
                    header = "index",
                    path = "index",
                    sort = IndexSort(IndexSortKey.NAME, SortOrder.ASC),
                    currentPageId = "current",
                    state = LoadState.LOADED,
                ),
            ),
            Panel.Headings(HeadingsPanel(state = LoadState.FAILED)),
            Panel.RecentPages(RecentPagesPanel(state = LoadState.LOADING)),
            Panel.Search(search("first")),
            Panel.Config(
                ConfigView(
                    markdownDir = "markdown",
                    weekNumbering = WeekNumbering.ISO,
                    logicalOutdent = true,
                    tagsDir = "tags",
                    homeLink = "home",
                    propertyDefinitions = emptyList(),
                    definitionsAdded = 0u,
                ),
            ),
            Panel.Connection(ConnectionState.CONNECTED),
            Panel.Today("2026-11-22"),
            Panel.Calendar(
                CalendarView(
                    year = 2026,
                    month = 11u,
                    monthName = "November",
                    monthPage = "",
                    yearPage = "",
                    dayLabels = emptyList(),
                    weeks = emptyList(),
                ),
            ),
            Panel.Title("the title"),
            Panel.History(HistoryView(canGoBack = true, canGoForward = false, url = "/pages/back")),
        )

    // 行の操作は順に当たる。挿入、削除、移動、更新の位置は、その前の操作を当てたあとの行で数える。
    @Test
    fun the_changes_are_applied_in_order_and_each_place_counts_after_the_changes_before() {
        val rows = listOf(row("a"), row("b"), row("c"))

        val applied =
            rows.applied(
                listOf(
                    RowChange.Insert(1u, row("x")),
                    RowChange.Remove(0u),
                    RowChange.Move(2u, 0u),
                    RowChange.Update(1u, row("x", "changed")),
                ),
            )

        assertEquals(listOf("c", "x", "b"), ids(applied))
        assertEquals("changed", applied[1].item.body)
    }

    // 入れ替えは、前の行をすべて捨てて、届いた行にする。
    @Test
    fun a_reset_puts_the_rows_that_came_in_place_of_all_before() {
        val rows = listOf(row("a"), row("b"))

        val applied = rows.applied(listOf(RowChange.Reset(listOf(row("y"), row("z")))))

        assertEquals(listOf("y", "z"), ids(applied))
    }

    // 最初の差分から、すべての欄が、それぞれのパネルの値で作られる。行は、入れ替えで届いた行である。
    @Test
    fun the_first_changes_make_every_part_from_its_panel() {
        val view = firstView(Changes(listOf(RowChange.Reset(listOf(row("a")))), everyPanel()))

        assertEquals(ScreenView.Settings, view.screen)
        assertEquals(true, view.outline.selectionMode)
        assertEquals("current", view.indexPages.currentPageId)
        assertEquals(LoadState.FAILED, view.headings.state)
        assertEquals(LoadState.LOADING, view.recentPages.state)
        assertEquals("first", view.search.query)
        assertEquals("tags", view.config.tagsDir)
        assertEquals(ConnectionState.CONNECTED, view.connection)
        assertEquals("2026-11-22", view.today)
        assertEquals("November", view.calendar.monthName)
        assertEquals("the title", view.title)
        assertEquals("/pages/back", view.history.url)
        assertEquals(listOf("a"), ids(view.outline.rows))
    }

    // 届いたパネルだけが入れ替わり、ほかの欄は前のままである。
    @Test
    fun only_the_panels_that_came_take_the_place_of_their_parts() {
        val view = firstView(Changes(emptyList(), everyPanel()))

        val changed =
            view.with(
                Changes(
                    emptyList(),
                    listOf(Panel.Search(search("changed")), Panel.Title("another")),
                ),
            )

        assertEquals("changed", changed.search.query)
        assertEquals("another", changed.title)
        assertEquals("tags", changed.config.tagsDir)
        assertEquals("/pages/back", changed.history.url)
    }

    // 後の差分の行の操作は、今の行に当たる。
    @Test
    fun the_rows_of_a_later_change_are_applied_to_the_rows_there() {
        val view = firstView(Changes(listOf(RowChange.Reset(listOf(row("a")))), everyPanel()))

        val changed = view.with(Changes(listOf(RowChange.Insert(1u, row("b"))), emptyList()))

        assertEquals(listOf("a", "b"), ids(changed.outline.rows))
    }

    // 後の差分の見出しの操作は、今の見出しに当たる。見出しの状態は前のままである。
    @Test
    fun the_headings_of_a_later_change_are_applied_to_the_headings_there() {
        val view =
            firstView(
                Changes(
                    rows = emptyList(),
                    headings =
                        listOf(
                            HeadingChange.Reset(
                                listOf(Heading("a", "One", 1u), Heading("b", "Two", 2u)),
                            ),
                        ),
                    indexPages = emptyList(),
                    recentPages = emptyList(),
                    searchResults = emptyList(),
                    panels = everyPanel(),
                ),
            )

        val changed =
            view.with(
                Changes(
                    rows = emptyList(),
                    headings = listOf(HeadingChange.Update(1u, Heading("b", "Three", 2u))),
                    indexPages = emptyList(),
                    recentPages = emptyList(),
                    searchResults = emptyList(),
                    panels = emptyList(),
                ),
            )

        assertEquals(listOf("One", "Three"), changed.headings.headings.map { it.text })
        assertEquals(LoadState.FAILED, changed.headings.state)
    }

    // 後の差分のページの操作は、今のフォルダのページと最近のページに当たる。それぞれのパネルの値は前のままである。
    @Test
    fun the_pages_of_a_later_change_are_applied_to_the_lists_there() {
        fun page(id: String) =
            SidebarPage(
                page =
                    Page(
                        id = id,
                        parentId = "",
                        dirPath = "",
                        name = id,
                        pageType = "doc",
                        createdDate = 0,
                        modifiedDate = 0,
                        properties = emptyMap(),
                    ),
                label = id,
                published = false,
                isDir = false,
                path = id,
            )
        val view =
            firstView(
                Changes(
                    rows = emptyList(),
                    headings = emptyList(),
                    indexPages = listOf(PageChange.Reset(listOf(page("a"), page("b")))),
                    recentPages = listOf(PageChange.Reset(listOf(page("c")))),
                    searchResults = emptyList(),
                    panels = everyPanel(),
                ),
            )

        val changed =
            view.with(
                Changes(
                    rows = emptyList(),
                    headings = emptyList(),
                    indexPages = listOf(PageChange.Remove(0u)),
                    recentPages = listOf(PageChange.Insert(0u, page("d"))),
                    searchResults = emptyList(),
                    panels = emptyList(),
                ),
            )

        assertEquals(listOf("b"), changed.indexPages.pages.map { it.label })
        assertEquals("current", changed.indexPages.currentPageId)
        assertEquals(listOf("d", "c"), changed.recentPages.pages.map { it.label })
        assertEquals(LoadState.LOADING, changed.recentPages.state)
    }

    // 後の差分の検索の結果の操作は、今の結果に当たる。検索のパネルの値は前のままである。
    @Test
    fun the_search_results_of_a_later_change_are_applied_to_the_results_there() {
        fun result(key: String, snippet: String) =
            SearchEntry(
                key = key,
                kind = SearchKind.ITEM,
                title = "t",
                label = "t",
                snippet = snippet,
            )
        val view =
            firstView(
                Changes(
                    rows = emptyList(),
                    headings = emptyList(),
                    indexPages = emptyList(),
                    recentPages = emptyList(),
                    searchResults =
                        listOf(
                            SearchChange.Reset(
                                listOf(result("item:a", "one"), result("item:b", "two")),
                            ),
                        ),
                    panels = everyPanel(),
                ),
            )

        val changed =
            view.with(
                Changes(
                    rows = emptyList(),
                    headings = emptyList(),
                    indexPages = emptyList(),
                    recentPages = emptyList(),
                    searchResults = listOf(SearchChange.Update(1u, result("item:b", "three"))),
                    panels = emptyList(),
                ),
            )

        assertEquals(listOf("one", "three"), changed.search.results.map { it.snippet })
        assertEquals("first", changed.search.query)
    }

    // アウトラインのパネルは行を持たないので、届いても行は前のままである。
    @Test
    fun the_outline_panel_keeps_the_rows() {
        val view =
            firstView(Changes(listOf(RowChange.Reset(listOf(row("a"), row("b")))), everyPanel()))

        val changed =
            view.with(Changes(emptyList(), listOf(Panel.Outline(outline(selecting = false)))))

        assertEquals(false, changed.outline.selectionMode)
        assertEquals(listOf("a", "b"), ids(changed.outline.rows))
    }
}
