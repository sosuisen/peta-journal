//! The search box: finding pages and items, and creating a page from the typed name.

use crate::app::{Event, PageState, Screen, SearchKey};
use crate::effects::{ApiOp, ApiOutput, UiOp};
use crate::harness::Harness;
use crate::model::{Page, SearchResult};
use crate::view::{SearchEntry, SearchKind};

fn page_hit(title: &str) -> SearchResult {
    SearchResult {
        page_id: format!("id-{title}"),
        page_title: title.into(),
        ..Default::default()
    }
}

fn item_hit(title: &str, item_id: &str) -> SearchResult {
    SearchResult {
        item_id: item_id.into(),
        ..page_hit(title)
    }
}

fn typed(h: &mut Harness, query: &str) {
    h.send(Event::SearchQueryChanged(query.into()));
}

/// Types the query and lets the server answer with the hits.
fn searched(h: &mut Harness, query: &str, hits: Vec<SearchResult>) {
    h.api
        .answer_next("SearchCombined", Ok(ApiOutput::SearchResults(hits)));
    typed(h, query);
    h.fire_timers();
}

fn key(h: &mut Harness, key: SearchKey) {
    h.send(Event::SearchKey(key));
}

fn urls(h: &Harness) -> Vec<String> {
    h.navigations.iter().map(|n| n.url.clone()).collect()
}

/// A harness showing the doc page `notes/Idea`, as the create entries depend on it.
fn on_doc_page() -> Harness {
    let mut h = Harness::new();
    h.model.screen = Screen::Page(PageState {
        page_name: "notes/Idea".into(),
        ..Default::default()
    });
    h.model.current_page = Some(Page {
        id: "idea".into(),
        dir_path: "notes/".into(),
        name: "Idea".into(),
        page_type: "doc".into(),
        ..Default::default()
    });
    h
}

/// The titles of the create entries.
fn creates(h: &Harness) -> Vec<String> {
    let results = h.view().search.results;
    let creates = results.into_iter().filter(|r| r.kind == SearchKind::Create);
    creates.map(|r| r.title).collect()
}

fn entry(index: usize, kind: SearchKind, title: &str) -> SearchEntry {
    SearchEntry {
        index,
        kind,
        title: title.into(),
        snippet: String::new(),
    }
}

// 入力が一息置いた後に検索され、結果はページ、作成、項目の順に番号付きで並ぶ。選択はまだない。
#[test]
fn the_query_is_searched_after_a_pause_and_the_results_are_listed_with_their_numbers() {
    let mut h = Harness::new();
    typed(&mut h, "plan");
    assert!(!h.api.called("SearchCombined"), "not before the pause");

    h.api.answer_next(
        "SearchCombined",
        Ok(ApiOutput::SearchResults(vec![
            item_hit("Diary", "i1"),
            page_hit("Plans"),
        ])),
    );
    h.fire_timers();
    let search = h.view().search;
    assert_eq!(search.query, "plan");
    assert_eq!(
        search.results,
        [
            entry(0, SearchKind::Page, "Plans"),
            entry(1, SearchKind::Create, "plan"),
            entry(2, SearchKind::Item, "Diary"),
        ]
    );
    assert_eq!(search.selected_index, None);
}

// `#タグ` を含む検索語では、ページ作成の項目は出ない。
#[test]
fn a_query_with_a_tag_offers_no_page_to_create() {
    let mut h = Harness::new();
    searched(&mut h, "#todo plan", vec![]);
    assert!(h.view().search.results.is_empty());
}

// 検索語が空になると、結果は検索せずにすぐ消える。
#[test]
fn an_empty_query_clears_the_hits_at_once() {
    let mut h = Harness::new();
    searched(&mut h, "plan", vec![page_hit("Plans")]);
    typed(&mut h, "  ");
    assert!(h.view().search.results.is_empty());
    assert!(h.pending_timer_ids().is_empty());
    assert_eq!(h.api.calls_named("SearchCombined").len(), 1);
}

// 続けて入力すると前のタイマーは取り消され、最後の検索語だけが検索される。
#[test]
fn a_new_keystroke_replaces_the_pending_search() {
    let mut h = Harness::new();
    typed(&mut h, "p");
    typed(&mut h, "pl");
    assert_eq!(h.pending_timer_ids().len(), 1);
    h.fire_timers();
    assert_eq!(h.api.calls_named("SearchCombined").len(), 1);
    assert!(h
        .api
        .calls
        .iter()
        .any(|c| matches!(c, ApiOp::SearchCombined { query, .. } if query == "pl")));
}

// 検索に失敗しても、それまでの結果は残る。
#[test]
fn a_failed_search_keeps_the_hits_shown() {
    let mut h = Harness::new();
    searched(&mut h, "plan", vec![page_hit("Plans")]);
    h.api.fail_next("SearchCombined", "offline");
    typed(&mut h, "plans");
    h.fire_timers();
    assert_eq!(h.view().search.results[0].title, "Plans");
}

// 検索語を変えると選択は消える。答えが届く前にEnterを押しても、前の検索語の結果は開かない。
#[test]
fn enter_right_after_changing_the_query_opens_no_result_of_the_old_query() {
    let mut h = Harness::new();
    searched(&mut h, "plan", vec![page_hit("Plans")]);
    key(&mut h, SearchKey::Down);
    assert_eq!(h.view().search.selected_index, Some(0));

    typed(&mut h, "plans");
    key(&mut h, SearchKey::Enter);
    assert_eq!(urls(&h), Vec::<String>::new());
    assert_eq!(h.view().search.selected_index, None);
}

// 検索語を受けてから答えが届くまで、検索中である。
#[test]
fn a_search_is_running_until_its_answer_arrives() {
    let mut h = Harness::new();
    assert!(!h.view().search.searching);
    typed(&mut h, "plan");
    assert!(h.view().search.searching);
    h.api
        .answer_next("SearchCombined", Ok(ApiOutput::SearchResults(vec![])));
    h.fire_timers();
    assert!(!h.view().search.searching);
}

// 検索が失敗しても、検索中は終わる。
#[test]
fn a_failed_search_is_no_longer_running() {
    let mut h = Harness::new();
    h.api.fail_next("SearchCombined", "offline");
    typed(&mut h, "plan");
    h.fire_timers();
    assert!(!h.view().search.searching);
}

// 空の検索語は検索しないので、検索中ではない。
#[test]
fn an_empty_query_is_not_a_running_search() {
    let mut h = Harness::new();
    typed(&mut h, "plan");
    typed(&mut h, "  ");
    assert!(!h.view().search.searching);
}

// 結果を開くと検索は片付き、検索中でなくなる。
#[test]
fn opening_a_result_ends_the_running_search() {
    let mut h = Harness::new();
    searched(&mut h, "plan", vec![page_hit("Plans")]);
    typed(&mut h, "plans");
    h.send(Event::SearchResultClicked { index: 0 });
    assert!(!h.view().search.searching);
}

// docページを開いているときは、そのフォルダに作る項目も出る。同じ名前のページが検索結果かサイドバーにあれば、その項目は出ない。大文字小文字は区別しない。
#[test]
fn the_create_entries_offer_the_bare_title_and_the_title_in_the_folder_of_the_open_page() {
    let mut h = on_doc_page();
    searched(&mut h, "Plan", vec![]);
    assert_eq!(creates(&h), ["Plan", "notes/Plan"]);

    searched(&mut h, "Plan", vec![page_hit("plan")]);
    assert_eq!(creates(&h), ["notes/Plan"]);

    h.model.sidebar.index_pages = vec![Page {
        dir_path: "notes/".into(),
        name: "PLAN".into(),
        ..Default::default()
    }];
    searched(&mut h, "Plan", vec![]);
    assert_eq!(creates(&h), ["Plan"]);
}

// ジャーナルやルートのページを開いているときは、フォルダがないので素の名前だけを提案する。
#[test]
fn a_journal_or_a_root_page_offers_only_the_bare_title() {
    let mut h = on_doc_page();
    h.model.current_page.as_mut().unwrap().page_type = "journal".into();
    searched(&mut h, "Plan", vec![]);
    assert_eq!(creates(&h), ["Plan"]);

    h.model.current_page = Some(Page {
        name: "Root".into(),
        page_type: "doc".into(),
        ..Default::default()
    });
    searched(&mut h, "Plan", vec![]);
    assert_eq!(creates(&h), ["Plan"]);
}

// 下キーと上キーは、ページ・作成・項目を通した一覧の上で選択を動かし、端で折り返す。結果がなければ何も選ばれない。
#[test]
fn arrow_keys_move_the_selection_over_pages_create_entries_and_items_and_wrap() {
    let mut h = Harness::new();
    key(&mut h, SearchKey::Down);
    assert_eq!(h.view().search.selected_index, None);

    searched(
        &mut h,
        "plan",
        vec![page_hit("Plans"), item_hit("Diary", "i1")],
    );
    // Plans, create "plan", Diary: three entries
    key(&mut h, SearchKey::Up);
    assert_eq!(h.view().search.selected_index, Some(2));
    key(&mut h, SearchKey::Down);
    assert_eq!(h.view().search.selected_index, Some(0));
    key(&mut h, SearchKey::Up);
    assert_eq!(h.view().search.selected_index, Some(2));
    key(&mut h, SearchKey::Down);
    assert_eq!(h.view().search.selected_index, Some(0));
    key(&mut h, SearchKey::Up);
    key(&mut h, SearchKey::Up);
    assert_eq!(h.view().search.selected_index, Some(1));
}

// Enter は選ばれた結果を開く。何も選ばれていなければ何もしない。Escape は一覧を閉じて選択を外す。
#[test]
fn enter_opens_the_selected_entry_and_escape_closes_the_list() {
    let mut h = Harness::new();
    h.send(Event::SearchDropdown(true));
    searched(&mut h, "plan", vec![page_hit("Plans")]);
    key(&mut h, SearchKey::Enter);
    assert!(urls(&h).is_empty());

    key(&mut h, SearchKey::Down);
    key(&mut h, SearchKey::Escape);
    assert!(!h.view().search.show_dropdown);
    assert_eq!(h.view().search.selected_index, None);

    key(&mut h, SearchKey::Down);
    key(&mut h, SearchKey::Enter);
    assert_eq!(urls(&h), ["/pages/Plans"]);
}

// Enterで結果を開いても検索欄にはフォーカスが残る。そのまま入力すると、一覧がまた開く。
#[test]
fn typing_after_enter_opened_a_result_shows_the_list_again() {
    let mut h = Harness::new();
    h.send(Event::SearchDropdown(true));
    searched(&mut h, "plan", vec![page_hit("Plans")]);
    key(&mut h, SearchKey::Down);
    key(&mut h, SearchKey::Enter);
    assert!(!h.view().search.show_dropdown);

    searched(&mut h, "diary", vec![page_hit("Diary")]);
    assert!(h.view().search.show_dropdown);
}

// ページの結果を選ぶとそのページが開き、項目の結果を選ぶとその項目まで飛ぶ。どちらも検索語と結果は消える。
#[test]
fn a_page_hit_opens_the_page_and_an_item_hit_jumps_to_the_item() {
    let mut h = Harness::new();
    searched(
        &mut h,
        "plan",
        vec![page_hit("Plans"), item_hit("Diary", "i1")],
    );
    h.send(Event::SearchResultClicked { index: 2 });
    assert_eq!(urls(&h), ["/pages/Diary#i1"]);
    assert_eq!(h.view().search.query, "");
    assert!(h.view().search.results.is_empty());

    searched(&mut h, "plan", vec![page_hit("Plans")]);
    h.send(Event::SearchResultClicked { index: 0 });
    assert_eq!(urls(&h), ["/pages/Diary#i1", "/pages/Plans"]);

    h.send(Event::SearchResultClicked { index: 9 });
    assert_eq!(urls(&h).len(), 2);
}

// 作成の項目を選ぶと、その名前のページが開く。フォルダ付きの名前はフォルダの中に開く。
#[test]
fn the_create_entry_opens_the_new_page() {
    let mut h = on_doc_page();
    searched(&mut h, "Plan", vec![]);
    h.send(Event::SearchResultClicked { index: 1 });
    assert_eq!(urls(&h), ["/pages/notes/Plan"]);
    assert_eq!(h.view().search.query, "");
}

// ファイル名に使えない名前は断られ、理由が出る。ジャーナルの日付の名前は検査されずに開く。
#[test]
fn a_bad_file_name_is_refused_and_a_journal_name_is_not_checked() {
    let mut h = Harness::new();
    searched(&mut h, "a:b", vec![]);
    h.send(Event::SearchResultClicked { index: 0 });
    assert!(urls(&h).is_empty());
    assert!(h.ui.iter().any(|op| matches!(op, UiOp::Alert(_))));

    searched(&mut h, "2026-02-10", vec![]);
    h.send(Event::SearchResultClicked { index: 0 });
    assert_eq!(urls(&h), ["/pages/2026-02-10"]);
}
