//! The changes of the screen since a shell was last told (ADR 076), for a shell of any body
//! model (ADR 078).

use crate::changes::{Changes, ListChange, OutlinePanel, Panel, Shown};
use crate::harness::Harness;
use crate::testing::plain_item;
use crate::{PetaApp, RichText, ViewModel};
use crux_core::App;

/// The view model of a core that has just started.
fn first_view() -> ViewModel<RichText> {
    Harness::new().view()
}

// 何も変わらなければ、差分は空である。
#[test]
fn nothing_changed_gives_empty_changes() {
    let mut shown = Shown::default();
    shown.changes(first_view());

    let changes = shown.changes(first_view());

    assert_eq!(
        changes,
        Changes {
            rows: vec![],
            panels: vec![]
        }
    );
}

// 変わったパネルだけが、差分に入る。
#[test]
fn only_the_panels_that_changed_are_in_the_changes() {
    let mut shown = Shown::default();
    shown.changes(first_view());
    let mut view = first_view();
    view.title = "notes".into();
    view.history.can_go_back = true;

    let changes = shown.changes(view.clone());

    assert_eq!(
        changes.panels,
        vec![Panel::Title(view.title), Panel::History(view.history)]
    );
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
                Panel::IndexPages(view.index_pages.clone()),
                Panel::Headings(view.headings.clone()),
                Panel::RecentPages(view.recent_pages.clone()),
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

// 本文が変わった行は、その場所の行の更新として、差分に入る。
#[test]
fn a_row_whose_body_changed_is_an_update_at_its_place() {
    let before = Harness::with_items(vec![plain_item("a", 1, "one"), plain_item("b", 1, "two")]);
    let after = Harness::with_items(vec![plain_item("a", 1, "one"), plain_item("b", 1, "three")]);
    let mut shown = Shown::default();
    shown.changes(before.view());
    let view = after.view();

    let changes = shown.changes(view.clone());

    assert_eq!(
        changes.rows,
        vec![ListChange::Update {
            at: 1,
            item: view.outline.rows[1].clone()
        }]
    );
}

// 差分は、どの本文の形の画面からも作れる。本文の形を持たない画面でも、行の入れ替えと、変わったパネルが作られる。
#[test]
fn the_changes_are_made_from_a_screen_of_any_body_model() {
    let h = Harness::new();
    let app = PetaApp::<()>::default();
    let mut shown = Shown::default();
    let first = shown.changes(app.view(&h.model));
    assert!(matches!(first.rows[..], [ListChange::Reset { .. }]));

    let mut view = app.view(&h.model);
    view.title = "notes".into();
    let changes = shown.changes(view);

    assert!(changes.rows.is_empty());
    assert_eq!(changes.panels, vec![Panel::Title("notes".into())]);
}
