//! The changes of the screen since a shell was last told (ADR 076), for a shell of any body
//! model (ADR 078).

use crate::changes::{
    Changes, HeadingChange, HeadingsPanel, IndexPagesPanel, ListChange, OutlinePanel, PageChange,
    Panel, RecentPagesPanel, RowChange, SearchChange, SearchPanel, Shown,
};
use crate::harness::Harness;
use crate::model::Page;
use crate::outline::Heading;
use crate::testing::plain_item;
use crate::view::{SearchEntry, SearchKind, SidebarPage};
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
            headings: vec![],
            index_pages: vec![],
            recent_pages: vec![],
            search_results: vec![],
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

// 最初の差分は、行と見出しとページの一覧の入れ替えと、全部のパネルである。
#[test]
fn the_first_changes_are_a_reset_of_every_list_and_every_panel() {
    let view = first_view();
    let mut shown = Shown::default();

    let changes = shown.changes(view.clone());

    let outline = &view.outline;
    let index = &view.index_pages;
    assert_eq!(
        changes,
        Changes {
            rows: vec![RowChange::Reset {
                items: outline.rows.clone()
            }],
            headings: vec![HeadingChange::Reset {
                items: view.headings.headings.clone()
            }],
            index_pages: vec![PageChange::Reset {
                items: index.pages.clone()
            }],
            recent_pages: vec![PageChange::Reset {
                items: view.recent_pages.pages.clone()
            }],
            search_results: vec![SearchChange::Reset {
                items: view.search.results.clone()
            }],
            panels: vec![
                Panel::Screen(view.screen.clone()),
                Panel::AtlasCards(view.atlas_cards.clone()),
                Panel::AtlasCallouts(view.atlas_callouts.clone()),
                Panel::AtlasLinks(view.atlas_links.clone()),
                Panel::AtlasUnplaced(view.atlas_unplaced.clone()),
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
                Panel::IndexPages(IndexPagesPanel {
                    header: index.header.clone(),
                    path: index.path.clone(),
                    sort: index.sort,
                    parent_path: index.parent_path.clone(),
                    current_page_id: index.current_page_id.clone(),
                    state: index.state,
                }),
                Panel::Headings(HeadingsPanel {
                    state: view.headings.state
                }),
                Panel::RecentPages(RecentPagesPanel {
                    state: view.recent_pages.state
                }),
                Panel::Search(SearchPanel {
                    query: view.search.query.clone(),
                    show_dropdown: view.search.show_dropdown,
                    selected_index: view.search.selected_index,
                    searching: view.search.searching,
                }),
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

// 文字の変わった見出しは、その場所の見出しの更新として、差分に入る。見出しのパネルは送られない。
#[test]
fn a_heading_whose_text_changed_is_an_update_of_the_headings_at_its_place() {
    let heading = |id: &str, text: &str| Heading {
        id: id.into(),
        text: text.into(),
        level: 1,
    };
    let mut before = first_view();
    before.headings.headings = vec![heading("a", "One"), heading("b", "Two")];
    let mut after = before.clone();
    after.headings.headings[1] = heading("b", "Three");
    let mut shown = Shown::default();
    shown.changes(before);

    let changes = shown.changes(after);

    assert_eq!(
        changes.headings,
        vec![HeadingChange::Update {
            at: 1,
            item: heading("b", "Three")
        }]
    );
    assert_eq!(changes.panels, vec![]);
}

/// A page of the sidebar, with the id `id`.
fn sidebar_page(id: &str) -> SidebarPage {
    SidebarPage {
        page: Page {
            id: id.into(),
            ..Page::default()
        },
        label: id.into(),
        published: false,
        is_dir: false,
        dir_label: None,
        path: id.into(),
    }
}

// 並べ替えを変えると、フォルダのページは、移動として差分に入る。フォルダのページのパネルは送られない。
#[test]
fn a_change_of_the_sort_moves_the_pages_of_the_folder() {
    let mut before = first_view();
    before.index_pages.pages = vec![sidebar_page("a"), sidebar_page("b")];
    let mut after = before.clone();
    after.index_pages.pages.reverse();
    let mut shown = Shown::default();
    shown.changes(before);

    let changes = shown.changes(after);

    assert_eq!(
        changes.index_pages,
        vec![PageChange::Move { from: 1, to: 0 }]
    );
    assert_eq!(changes.panels, vec![]);
}

// 新しく変わったページは、最近のページの先頭への挿入として、差分に入る。最近のページのパネルは送られない。
#[test]
fn a_page_changed_last_is_inserted_at_the_top_of_the_recent_pages() {
    let mut before = first_view();
    before.recent_pages.pages = vec![sidebar_page("a")];
    let mut after = before.clone();
    after.recent_pages.pages.insert(0, sidebar_page("b"));
    let mut shown = Shown::default();
    shown.changes(before);

    let changes = shown.changes(after);

    assert_eq!(
        changes.recent_pages,
        vec![PageChange::Insert {
            at: 0,
            item: sidebar_page("b")
        }]
    );
    assert_eq!(changes.panels, vec![]);
}

// 抜き出しの変わった検索の結果は、その場所の結果の更新として、差分に入る。検索のパネルは送られない。
#[test]
fn a_search_result_whose_snippet_changed_is_an_update_of_the_results_at_its_place() {
    let result = |key: &str, snippet: &str| SearchEntry {
        key: key.into(),
        kind: SearchKind::Item,
        title: "t".into(),
        label: "t".into(),
        snippet: snippet.into(),
    };
    let mut before = first_view();
    before.search.results = vec![result("item:a", "one"), result("item:b", "two")];
    let mut after = before.clone();
    after.search.results[1] = result("item:b", "three");
    let mut shown = Shown::default();
    shown.changes(before);

    let changes = shown.changes(after);

    assert_eq!(
        changes.search_results,
        vec![SearchChange::Update {
            at: 1,
            item: result("item:b", "three")
        }]
    );
    assert_eq!(changes.panels, vec![]);
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
        vec![RowChange::Update {
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
    assert!(matches!(first.rows[..], [RowChange::Reset { .. }]));

    let mut view = app.view(&h.model);
    view.title = "notes".into();
    let changes = shown.changes(view);

    assert!(changes.rows.is_empty());
    assert_eq!(changes.panels, vec![Panel::Title("notes".into())]);
}

// 境界に出す行の差分（RowChange）は、コアの差分（ListChange）から移しても中身が変わらず、同じJSONになる。
#[test]
fn a_row_change_made_from_a_list_change_writes_the_same_json() {
    let view = Harness::with_items(vec![plain_item("a", 1, "one")]).view();
    let row = view.outline.rows[0].clone();
    let changes = [
        ListChange::Insert {
            at: 1,
            item: row.clone(),
        },
        ListChange::Remove { at: 2 },
        ListChange::Move { from: 3, to: 4 },
        ListChange::Update {
            at: 5,
            item: row.clone(),
        },
        ListChange::Reset { items: vec![row] },
    ];

    for change in changes {
        let written = serde_json::to_value(&change).unwrap();

        let moved: RowChange<RichText> = change.into();

        assert_eq!(serde_json::to_value(&moved).unwrap(), written);
    }
}
