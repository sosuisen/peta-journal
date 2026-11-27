//! The view model as a shell keeps it from the changes of the core (ADR 076), for the tests to
//! hold against the view model of the core.

use crate::{Changes, Panel, RowChange};
use client_core::{RichText, ViewModel};

/// `view` after the changes, applied as a shell applies them. Each panel that came takes the
/// place of its part, and the changes to the rows are applied in order.
pub(super) fn applied(mut view: ViewModel<RichText>, changes: Changes) -> ViewModel<RichText> {
    for panel in changes.panels {
        match panel {
            Panel::Screen(value) => view.screen = value,
            Panel::AtlasCards(value) => view.atlas_cards = value,
            Panel::AtlasCallouts(value) => view.atlas_callouts = value,
            Panel::AtlasLinks(value) => view.atlas_links = value,
            Panel::AtlasLinkSuggestions(value) => view.atlas_link_suggestions = value,
            Panel::Outline(value) => {
                let outline = &mut view.outline;
                outline.selection_mode = value.selection_mode;
                outline.focus_request = value.focus_request;
                outline.focus_order = value.focus_order;
                outline.focused_item_id = value.focused_item_id;
                outline.key_relay_holding = value.key_relay_holding;
                outline.wiki = value.wiki;
                outline.scrolled_from_top = value.scrolled_from_top;
            }
            Panel::IndexPages(value) => view.index_pages = value,
            Panel::Headings(value) => view.headings = value,
            Panel::RecentPages(value) => view.recent_pages = value,
            Panel::Search(value) => view.search = value,
            Panel::Config(value) => view.config = value,
            Panel::Connection(value) => view.connection = value,
            Panel::Today(value) => view.today = value,
            Panel::Calendar(value) => view.calendar = value,
            Panel::Title(value) => view.title = value,
            Panel::History(value) => view.history = value,
        }
    }
    for change in changes.rows {
        let rows = &mut view.outline.rows;
        match change {
            RowChange::Insert { at, item } => rows.insert(at as usize, item),
            RowChange::Remove { at } => {
                rows.remove(at as usize);
            }
            RowChange::Move { from, to } => {
                let row = rows.remove(from as usize);
                rows.insert(to as usize, row);
            }
            RowChange::Update { at, item } => rows[at as usize] = item,
            RowChange::Reset { items } => *rows = items,
        }
    }
    view
}
