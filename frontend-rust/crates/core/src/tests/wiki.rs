//! The popup of page names typed inside `[[ ]]`.

use crate::app::{Event, WikiLinkContext};
use crate::effects::ApiOutput;
use crate::harness::{item, update_calls, Harness};
use crate::model::SearchResult;

fn found(title: &str) -> SearchResult {
    SearchResult {
        page_title: title.into(),
        ..Default::default()
    }
}

fn editing(body: &str) -> Harness {
    let mut h = Harness::with_items(vec![item("a", 1, body)]);
    h.model.state.add_editing_item("a");
    h
}

fn typed_in_link(h: &mut Harness, query: &str) {
    h.send(Event::WikiQuery {
        item_id: "a".into(),
        context: Some(WikiLinkContext {
            query: query.into(),
            x: 10.0,
            y: 20.0,
        }),
    });
}

fn offers(h: &mut Harness, titles: &[&str]) {
    h.api.answer_next(
        "SearchPageNames",
        Ok(ApiOutput::SearchResults(
            titles.iter().map(|t| found(t)).collect(),
        )),
    );
}

fn key(h: &mut Harness, key: &str) {
    h.send(Event::WikiKey { key: key.into() });
}

// `[[` の中で入力すると、一息置いた後にページ名の候補が出る。ポップアップはカーソルの少し下に置かれ、最初の候補が選ばれている。
#[test]
fn typing_inside_a_link_offers_page_names_after_a_pause() {
    let mut h = editing("see [[To]]");
    offers(&mut h, &["Todo", "Tools"]);
    typed_in_link(&mut h, "To");
    assert!(
        h.view().outline.wiki.suggestions.is_empty(),
        "not before the pause"
    );

    h.fire_timers();
    let wiki = h.view().outline.wiki;
    assert_eq!(wiki.suggestions, ["Todo", "Tools"]);
    assert_eq!(wiki.item_id.as_deref(), Some("a"));
    assert_eq!((wiki.selected_index, wiki.x, wiki.y), (0, 10.0, 24.0));
    assert!(h.view().outline.rows[0].suggestions_visible);
}

// 続けて入力すると前のタイマーは取り消され、最後の検索語だけが検索される。
#[test]
fn a_new_keystroke_replaces_the_pending_search() {
    let mut h = editing("[[T]]");
    typed_in_link(&mut h, "T");
    typed_in_link(&mut h, "To");
    assert_eq!(h.pending_timer_ids().len(), 1);

    h.fire_timers();
    assert_eq!(h.api.calls_named("SearchPageNames").len(), 1);
    assert!(h.api.calls.iter().any(
        |c| matches!(c, crate::effects::ApiOp::SearchPageNames { query, .. } if query == "To")
    ));
}

// 検索語が空になると候補は検索せずに消え、`[[` の外に出るとポップアップは閉じる。
#[test]
fn an_empty_query_clears_the_suggestions_and_leaving_the_link_closes_the_popup() {
    let mut h = editing("[[To]]");
    offers(&mut h, &["Todo"]);
    typed_in_link(&mut h, "To");
    h.fire_timers();

    typed_in_link(&mut h, "");
    assert!(h.view().outline.wiki.suggestions.is_empty());
    assert!(h.pending_timer_ids().is_empty());
    assert_eq!(h.view().outline.wiki.item_id.as_deref(), Some("a"));

    h.send(Event::WikiQuery {
        item_id: "a".into(),
        context: None,
    });
    assert_eq!(h.view().outline.wiki.item_id, None);
}

// ページ名の検索に失敗すると、候補は空のままである。
#[test]
fn a_failed_search_leaves_no_suggestions() {
    let mut h = editing("[[To]]");
    h.api.fail_next("SearchPageNames", "offline");
    typed_in_link(&mut h, "To");
    h.fire_timers();

    assert!(h.view().outline.wiki.suggestions.is_empty());
    assert!(!h.view().outline.rows[0].suggestions_visible);
}

// ArrowDown と ArrowUp は候補の選択を端で折り返して動かす。候補がなければキーは何もしない。
#[test]
fn arrow_keys_move_the_pick_and_wrap_around() {
    let mut h = editing("[[To]]");
    key(&mut h, "ArrowDown");
    assert_eq!(h.view().outline.wiki.selected_index, 0);

    offers(&mut h, &["Todo", "Tools", "Top"]);
    typed_in_link(&mut h, "To");
    h.fire_timers();
    key(&mut h, "ArrowUp");
    assert_eq!(h.view().outline.wiki.selected_index, 2);
    key(&mut h, "ArrowDown");
    assert_eq!(h.view().outline.wiki.selected_index, 0);
    key(&mut h, "Tab");
    assert_eq!(h.view().outline.wiki.selected_index, 0);
}

// Enter は選ばれた候補で `[[` と `]]` の間の文を置き換え、カーソルをその直後に置き、入力として保存を予約し、ポップアップを閉じる。
#[test]
fn enter_puts_the_picked_title_into_the_link_and_closes_the_popup() {
    let mut h = editing("see [[To]] end");
    offers(&mut h, &["Todo", "Tools"]);
    typed_in_link(&mut h, "To");
    h.fire_timers();
    key(&mut h, "ArrowDown");
    key(&mut h, "Enter");

    assert_eq!(h.bodies(), ["a=see [[Tools]] end"]);
    assert_eq!(h.focus_cursor(), Some("see [[Tools".chars().count()));
    assert_eq!(h.view().outline.wiki.item_id, None);
    h.fire_timers();
    assert_eq!(
        update_calls(&h.api),
        [("a".into(), Some("see [[Tools]] end".into()))]
    );
}

// クリックで選んだ候補も同じように入る。入れ子の `[[` があるときは、内側のリンクが置き換えられる。
#[test]
fn a_pick_fills_the_innermost_open_link() {
    let mut h = editing("[[a [[To]]");
    offers(&mut h, &["Todo"]);
    typed_in_link(&mut h, "To");
    h.fire_timers();
    h.send(Event::WikiPick {
        title: "Todo".into(),
    });

    assert_eq!(h.bodies(), ["a=[[a [[Todo]]"]);
}

// 閉じる `]]` がなければ何も入らず、ポップアップだけが閉じる。ポップアップが開いていなければ選択は無視される。
#[test]
fn a_pick_without_a_closing_bracket_or_without_a_popup_changes_nothing() {
    let mut h = editing("[[To");
    offers(&mut h, &["Todo"]);
    typed_in_link(&mut h, "To");
    h.fire_timers();
    h.send(Event::WikiPick {
        title: "Todo".into(),
    });
    assert_eq!(h.bodies(), ["a=[[To"]);
    assert_eq!(h.view().outline.wiki.item_id, None);

    h.send(Event::WikiPick {
        title: "Todo".into(),
    });
    assert_eq!(h.bodies(), ["a=[[To"]);

    let mut h = editing("To");
    offers(&mut h, &["Todo"]);
    typed_in_link(&mut h, "To");
    h.fire_timers();
    h.send(Event::WikiPick {
        title: "Todo".into(),
    });
    assert_eq!(h.bodies(), ["a=To"]);
}

// 候補を選ぶときに項目がもうなければ、ポップアップが閉じるだけである。
#[test]
fn a_pick_for_an_item_that_is_gone_only_closes_the_popup() {
    let mut h = editing("[[To]]");
    offers(&mut h, &["Todo"]);
    typed_in_link(&mut h, "To");
    h.fire_timers();
    h.model.state.set_items(vec![]);
    h.send(Event::WikiPick {
        title: "Todo".into(),
    });

    assert_eq!(h.view().outline.wiki.item_id, None);
    assert!(update_calls(&h.api).is_empty());
}

fn open_link_at(h: &mut Harness, cursor: usize) {
    h.send(Event::OpenLinkAt {
        item_id: "a".into(),
        cursor,
    });
}

// カーソルの下のウィキリンクは、そのページを開く。
#[test]
fn the_wiki_link_at_the_cursor_opens_its_page() {
    let mut h = Harness::with_items(vec![item("a", 1, "see [[notes/Idea]]")]);
    open_link_at(&mut h, 8);
    assert_eq!(h.navigations.last().unwrap().url, "/pages/notes/Idea");
}

// カーソルの下のハッシュタグは、設定のタグのフォルダのページを開く。
#[test]
fn the_tag_at_the_cursor_opens_its_page_in_the_tags_folder() {
    let mut h = Harness::with_items(vec![item("a", 1, "about #rust")]);
    h.model.config.tags_dir = "tags/".into();
    open_link_at(&mut h, 8);
    assert_eq!(h.navigations.last().unwrap().url, "/pages/tags/rust");
}

// カーソルの下にリンクがなければ、どこへも移動しない。
#[test]
fn no_page_opens_when_the_cursor_is_not_on_a_link() {
    let mut h = Harness::with_items(vec![item("a", 1, "see [[notes/Idea]]")]);
    open_link_at(&mut h, 1);
    assert!(h.navigations.is_empty());
}
