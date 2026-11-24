package com.sosuisha.petajournal.shell

import com.sosuisha.petajournal.types.Changes
import com.sosuisha.petajournal.types.ListChange
import com.sosuisha.petajournal.types.OutlinePanel
import com.sosuisha.petajournal.types.OutlineView
import com.sosuisha.petajournal.types.Panel
import com.sosuisha.petajournal.types.Row
import com.sosuisha.petajournal.types.ViewModel

// The view model that the changes of the core make (ADR 076). The core sends only what changed;
// the shell keeps the whole of it for the screen to read.

/** The view model of the first changes of a core. They hold every panel and a reset of the rows. */
internal fun firstView(changes: Changes): ViewModel {
    val panels = changes.panels
    return ViewModel(
        screen = panels.the<Panel.Screen>().value,
        atlasCards = panels.the<Panel.AtlasCards>().value,
        atlasCallouts = panels.the<Panel.AtlasCallouts>().value,
        atlasLinks = panels.the<Panel.AtlasLinks>().value,
        atlasLinkSuggestions = panels.the<Panel.AtlasLinkSuggestions>().value,
        outline =
            outlineOf(panels.the<Panel.Outline>().value, emptyList<Row>().applied(changes.rows)),
        sidebar = panels.the<Panel.Sidebar>().value,
        search = panels.the<Panel.Search>().value,
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
    if (changes.rows.isEmpty()) return view
    return view.copy(outline = view.outline.copy(rows = view.outline.rows.applied(changes.rows)))
}

/** The rows after the changes, applied one after another. */
internal fun List<Row>.applied(changes: List<ListChange>): List<Row> {
    val rows = ArrayList(this)
    for (change in changes) {
        when (change) {
            is ListChange.Insert -> rows.add(change.at.toInt(), change.item)
            is ListChange.Remove -> rows.removeAt(change.at.toInt())
            is ListChange.Move -> rows.add(change.to.toInt(), rows.removeAt(change.from.toInt()))
            is ListChange.Update -> rows[change.at.toInt()] = change.item
            is ListChange.Reset -> {
                rows.clear()
                rows.addAll(change.items)
            }
        }
    }
    return rows
}

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
        is Panel.AtlasLinkSuggestions -> copy(atlasLinkSuggestions = panel.value)
        is Panel.Outline -> copy(outline = outlineOf(panel.value, outline.rows))
        is Panel.Sidebar -> copy(sidebar = panel.value)
        is Panel.Search -> copy(search = panel.value)
        is Panel.Config -> copy(config = panel.value)
        is Panel.Connection -> copy(connection = panel.value)
        is Panel.Today -> copy(today = panel.value)
        is Panel.Calendar -> copy(calendar = panel.value)
        is Panel.Title -> copy(title = panel.value)
        is Panel.History -> copy(history = panel.value)
    }
