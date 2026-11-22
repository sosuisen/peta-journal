use crate::changes::{Changes, OutlinePanel, Panel, Shown};
use crate::diff::ListChange;
use crate::BorderApp;
use client_core::{RichText, ViewModel};
use crux_core::Core;

/// The view model of a core that has just started.
fn first_view() -> ViewModel<RichText> {
    Core::<BorderApp>::new().view()
}

// 最初の差分は、行の入れ替えと、全部のパネルである。
#[test]
fn the_first_changes_are_a_reset_of_the_rows_and_every_panel() {
    let view = first_view();
    let mut shown = Shown::default();

    let changes = shown.changes(view.clone());

    let outline = &view.outline;
    assert_eq!(
        changes,
        Changes {
            rows: vec![ListChange::Reset {
                items: outline.rows.clone()
            }],
            panels: vec![
                Panel::Screen(view.screen.clone()),
                Panel::AtlasCards(view.atlas_cards.clone()),
                Panel::AtlasCallouts(view.atlas_callouts.clone()),
                Panel::AtlasLinks(view.atlas_links.clone()),
                Panel::AtlasLinkSuggestions(view.atlas_link_suggestions.clone()),
                Panel::Outline(OutlinePanel {
                    selection_mode: outline.selection_mode,
                    focus_request: outline.focus_request.clone(),
                    focus_order: outline.focus_order.clone(),
                    focused_item_id: outline.focused_item_id.clone(),
                    key_relay_holding: outline.key_relay_holding,
                    wiki: outline.wiki.clone(),
                    scrolled_from_top: outline.scrolled_from_top,
                }),
                Panel::Sidebar(view.sidebar.clone()),
                Panel::Search(view.search.clone()),
                Panel::Config(view.config.clone()),
                Panel::Connection(view.connection),
                Panel::Today(view.today.clone()),
                Panel::Calendar(view.calendar.clone()),
                Panel::Title(view.title.clone()),
                Panel::History(view.history.clone()),
            ],
        }
    );
}
