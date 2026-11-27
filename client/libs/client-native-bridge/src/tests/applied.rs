//! The view model as a shell keeps it from the changes of the core (ADR 076), for the tests to
//! hold against the view model of the core.

use crate::{Changes, HeadingChange, PageChange, Panel, RowChange};
use client_core::{RichText, ViewModel};

/// Applies `change`, of the change type `kind` of the border, to the list `list`. The change
/// types of the lists all have the same shape (ADR 088).
macro_rules! applied {
    ($kind:ident, $change:expr, $list:expr) => {{
        let list = $list;
        match $change {
            $kind::Insert { at, item } => list.insert(at as usize, item),
            $kind::Remove { at } => {
                list.remove(at as usize);
            }
            $kind::Move { from, to } => {
                let item = list.remove(from as usize);
                list.insert(to as usize, item);
            }
            $kind::Update { at, item } => list[at as usize] = item,
            $kind::Reset { items } => *list = items,
        }
    }};
}

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
            Panel::IndexPages(value) => {
                let index = &mut view.index_pages;
                index.header = value.header;
                index.path = value.path;
                index.sort = value.sort;
                index.parent_path = value.parent_path;
                index.current_page_id = value.current_page_id;
                index.state = value.state;
            }
            Panel::Headings(value) => view.headings.state = value.state,
            Panel::RecentPages(value) => view.recent_pages.state = value.state,
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
        applied!(RowChange, change, &mut view.outline.rows);
    }
    for change in changes.headings {
        applied!(HeadingChange, change, &mut view.headings.headings);
    }
    for change in changes.index_pages {
        applied!(PageChange, change, &mut view.index_pages.pages);
    }
    for change in changes.recent_pages {
        applied!(PageChange, change, &mut view.recent_pages.pages);
    }
    view
}
