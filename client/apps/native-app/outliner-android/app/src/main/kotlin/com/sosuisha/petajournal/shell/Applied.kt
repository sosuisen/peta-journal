package com.sosuisha.petajournal.shell

import com.sosuisha.petajournal.types.Changes
import com.sosuisha.petajournal.types.Heading
import com.sosuisha.petajournal.types.HeadingChange
import com.sosuisha.petajournal.types.HeadingsView
import com.sosuisha.petajournal.types.IndexPagesPanel
import com.sosuisha.petajournal.types.IndexPagesView
import com.sosuisha.petajournal.types.OutlinePanel
import com.sosuisha.petajournal.types.OutlineView
import com.sosuisha.petajournal.types.PageChange
import com.sosuisha.petajournal.types.Panel
import com.sosuisha.petajournal.types.RecentPagesView
import com.sosuisha.petajournal.types.Row
import com.sosuisha.petajournal.types.RowChange
import com.sosuisha.petajournal.types.SearchChange
import com.sosuisha.petajournal.types.SearchEntry
import com.sosuisha.petajournal.types.SearchPanel
import com.sosuisha.petajournal.types.SearchView
import com.sosuisha.petajournal.types.SidebarPage
import com.sosuisha.petajournal.types.ViewModel

// The view model that the changes of the core make (ADR 076). The core sends only what changed;
// the shell keeps the whole of it for the screen to read.

/**
 * The view model of the first changes of a core. They hold every panel and a reset of the rows, the
 * headings, the pages of the sidebar and the results of the search.
 */
internal fun firstView(changes: Changes): ViewModel {
    val panels = changes.panels
    return ViewModel(
        screen = panels.the<Panel.Screen>().value,
        atlasCards = panels.the<Panel.AtlasCards>().value,
        atlasCallouts = panels.the<Panel.AtlasCallouts>().value,
        atlasLinks = panels.the<Panel.AtlasLinks>().value,
        atlasUnplaced = panels.the<Panel.AtlasUnplaced>().value,
        outline =
            outlineOf(panels.the<Panel.Outline>().value, emptyList<Row>().applied(changes.rows)),
        indexPages =
            indexPagesOf(
                panels.the<Panel.IndexPages>().value,
                emptyList<SidebarPage>().after(changes.indexPages.map { it.change }),
            ),
        headings =
            HeadingsView(
                headings = emptyList<Heading>().after(changes.headings.map { it.change }),
                state = panels.the<Panel.Headings>().value.state,
            ),
        recentPages =
            RecentPagesView(
                pages = emptyList<SidebarPage>().after(changes.recentPages.map { it.change }),
                state = panels.the<Panel.RecentPages>().value.state,
            ),
        search =
            searchOf(
                panels.the<Panel.Search>().value,
                emptyList<SearchEntry>().after(changes.searchResults.map { it.change }),
            ),
        config = panels.the<Panel.Config>().value,
        connection = panels.the<Panel.Connection>().value,
        today = panels.the<Panel.Today>().value,
        calendar = panels.the<Panel.Calendar>().value,
        title = panels.the<Panel.Title>().value,
        history = panels.the<Panel.History>().value,
    )
}

/** The view model after the changes. */
internal fun ViewModel.with(changes: Changes): ViewModel {
    var view = this
    for (panel in changes.panels) view = view.with(panel)
    if (changes.rows.isNotEmpty()) {
        view =
            view.copy(outline = view.outline.copy(rows = view.outline.rows.applied(changes.rows)))
    }
    if (changes.headings.isNotEmpty()) {
        val headings = view.headings.headings.after(changes.headings.map { it.change })
        view = view.copy(headings = view.headings.copy(headings = headings))
    }
    if (changes.indexPages.isNotEmpty()) {
        val pages = view.indexPages.pages.after(changes.indexPages.map { it.change })
        view = view.copy(indexPages = view.indexPages.copy(pages = pages))
    }
    if (changes.recentPages.isNotEmpty()) {
        val pages = view.recentPages.pages.after(changes.recentPages.map { it.change })
        view = view.copy(recentPages = view.recentPages.copy(pages = pages))
    }
    if (changes.searchResults.isNotEmpty()) {
        val results = view.search.results.after(changes.searchResults.map { it.change })
        view = view.copy(search = view.search.copy(results = results))
    }
    return view
}

/** The rows after the changes, applied one after another. */
internal fun List<Row>.applied(changes: List<RowChange>): List<Row> =
    after(changes.map { it.change })

/**
 * A change of a list of any kind (ADR 088). The border has a type of its own for the changes of
 * each list, all of the same shape; each is made into this, and [after] applies them all.
 */
internal sealed interface Change<out T> {
    data class Insert<T>(val at: Int, val item: T) : Change<T>

    data class Remove(val at: Int) : Change<Nothing>

    data class Move(val from: Int, val to: Int) : Change<Nothing>

    data class Update<T>(val at: Int, val item: T) : Change<T>

    data class Reset<T>(val items: List<T>) : Change<T>
}

/** The list after the changes, applied one after another. */
internal fun <T> List<T>.after(changes: List<Change<T>>): List<T> {
    val items = ArrayList(this)
    for (change in changes) {
        when (change) {
            is Change.Insert -> items.add(change.at, change.item)
            is Change.Remove -> items.removeAt(change.at)
            is Change.Move -> items.add(change.to, items.removeAt(change.from))
            is Change.Update -> items[change.at] = change.item
            is Change.Reset -> {
                items.clear()
                items.addAll(change.items)
            }
        }
    }
    return items
}

private val RowChange.change: Change<Row>
    get() =
        when (this) {
            is RowChange.Insert -> Change.Insert(at.toInt(), item)
            is RowChange.Remove -> Change.Remove(at.toInt())
            is RowChange.Move -> Change.Move(from.toInt(), to.toInt())
            is RowChange.Update -> Change.Update(at.toInt(), item)
            is RowChange.Reset -> Change.Reset(items)
        }

private val HeadingChange.change: Change<Heading>
    get() =
        when (this) {
            is HeadingChange.Insert -> Change.Insert(at.toInt(), item)
            is HeadingChange.Remove -> Change.Remove(at.toInt())
            is HeadingChange.Move -> Change.Move(from.toInt(), to.toInt())
            is HeadingChange.Update -> Change.Update(at.toInt(), item)
            is HeadingChange.Reset -> Change.Reset(items)
        }

private val PageChange.change: Change<SidebarPage>
    get() =
        when (this) {
            is PageChange.Insert -> Change.Insert(at.toInt(), item)
            is PageChange.Remove -> Change.Remove(at.toInt())
            is PageChange.Move -> Change.Move(from.toInt(), to.toInt())
            is PageChange.Update -> Change.Update(at.toInt(), item)
            is PageChange.Reset -> Change.Reset(items)
        }

private val SearchChange.change: Change<SearchEntry>
    get() =
        when (this) {
            is SearchChange.Insert -> Change.Insert(at.toInt(), item)
            is SearchChange.Remove -> Change.Remove(at.toInt())
            is SearchChange.Move -> Change.Move(from.toInt(), to.toInt())
            is SearchChange.Update -> Change.Update(at.toInt(), item)
            is SearchChange.Reset -> Change.Reset(items)
        }

private fun searchOf(panel: SearchPanel, results: List<SearchEntry>) =
    SearchView(
        query = panel.query,
        results = results,
        showDropdown = panel.showDropdown,
        selectedIndex = panel.selectedIndex,
        searching = panel.searching,
    )

private fun indexPagesOf(panel: IndexPagesPanel, pages: List<SidebarPage>) =
    IndexPagesView(
        header = panel.header,
        path = panel.path,
        sort = panel.sort,
        parentPath = panel.parentPath,
        pages = pages,
        currentPageId = panel.currentPageId,
        state = panel.state,
    )

private inline fun <reified P : Panel> List<Panel>.the(): P = filterIsInstance<P>().single()

private fun outlineOf(panel: OutlinePanel, rows: List<Row>) =
    OutlineView(
        rows = rows,
        selectionMode = panel.selectionMode,
        focusRequest = panel.focusRequest,
        focusOrder = panel.focusOrder,
        focusedItemId = panel.focusedItemId,
        keyRelayHolding = panel.keyRelayHolding,
        wiki = panel.wiki,
        scrolledFromTop = panel.scrolledFromTop,
    )

private fun ViewModel.with(panel: Panel): ViewModel =
    when (panel) {
        is Panel.Screen -> copy(screen = panel.value)
        is Panel.AtlasCards -> copy(atlasCards = panel.value)
        is Panel.AtlasCallouts -> copy(atlasCallouts = panel.value)
        is Panel.AtlasLinks -> copy(atlasLinks = panel.value)
        is Panel.AtlasUnplaced -> copy(atlasUnplaced = panel.value)
        is Panel.Outline -> copy(outline = outlineOf(panel.value, outline.rows))
        is Panel.IndexPages -> copy(indexPages = indexPagesOf(panel.value, indexPages.pages))
        is Panel.Headings -> copy(headings = headings.copy(state = panel.value.state))
        is Panel.RecentPages -> copy(recentPages = recentPages.copy(state = panel.value.state))
        is Panel.Search -> copy(search = searchOf(panel.value, search.results))
        is Panel.Config -> copy(config = panel.value)
        is Panel.Connection -> copy(connection = panel.value)
        is Panel.Today -> copy(today = panel.value)
        is Panel.Calendar -> copy(calendar = panel.value)
        is Panel.Title -> copy(title = panel.value)
        is Panel.History -> copy(history = panel.value)
    }
