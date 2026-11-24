//! What the rows of the view model say about their items, and the zoom of an item.

use crate::app::Event;
use crate::effects::ApiOp;
use crate::harness::{item, Harness};
use crate::model::ItemExt;
use crate::model::ItemPosition;
use crate::testing::{item_with, shown};
use crate::view::{Row, TaskState};

/// The items whose rows say that they have children, in their order.
fn with_children(h: &Harness) -> Vec<String> {
    rows(h)
        .iter()
        .filter(|row| row.has_children)
        .map(|row| row.item.id.clone())
        .collect()
}

// 子のある項目だけが、子を持つとされる。項目を足したときと消したときに、その答えも変わる。
#[test]
fn only_an_item_with_children_has_children_and_the_answer_follows_added_and_deleted_items() {
    let mut h = Harness::with_items(vec![item("a", 1, "a"), item("b", 1, "b")]);
    assert!(with_children(&h).is_empty());

    h.model
        .state
        .add_item(item("child", 2, "child"), &ItemPosition::after("a"));
    assert_eq!(with_children(&h), ["a"]);

    h.model.state.delete_item("child");
    assert!(with_children(&h).is_empty());
}

/// A page with one transclusion `t` of the item `src`, whose tree is `tree`: the source item
/// first, then its descendants.
fn with_transclusion(tree: Vec<crate::model::Item>) -> Harness {
    let transclusion = item_with("t", 1, |i| i.transclude_item_id = Some("src".into()));
    let mut h = Harness::with_items(vec![item("a", 1, "a"), transclusion]);
    h.model
        .state
        .set_transcluded_trees([("src".to_string(), tree)].into());
    h
}

// 埋め込みの行は、埋め込んだ元の項目に子があるときだけ、子を持つとされる。
#[test]
fn a_transclusion_has_children_only_when_its_source_item_has_them() {
    let alone = with_transclusion(vec![item("src", 1, "src")]);
    assert!(with_children(&alone).is_empty());

    let parent = with_transclusion(vec![item("src", 1, "src"), item("c", 2, "c")]);
    assert_eq!(with_children(&parent), ["t"]);
}

// 埋め込まれた子の行は、すぐ下に、より深い行があるときだけ、子を持つとされる。
#[test]
fn a_transcluded_row_has_children_only_when_a_deeper_row_follows_it() {
    let h = with_transclusion(vec![
        item("src", 1, "src"),
        item("c1", 2, "c1"),
        item("c2", 3, "c2"),
        item("c3", 3, "c3"),
    ]);

    assert_eq!(with_children(&h), ["t", "c1"]);
}

// 子が別の項目の下に動くと、子を持つとされる項目も変わる。
#[test]
fn a_child_moved_under_another_item_makes_that_item_the_one_with_children() {
    let mut h = Harness::with_items(vec![
        item("a", 1, "a"),
        item("child", 2, "child"),
        item("b", 1, "b"),
    ]);
    assert_eq!(with_children(&h), ["a"]);

    h.model.state.replace_items(
        &["child".to_string()],
        vec![item("child", 2, "child")],
        &ItemPosition::after("b"),
    );

    assert_eq!(with_children(&h), ["b"]);
}

fn rows(h: &Harness) -> Vec<Row<crate::RichText>> {
    h.view().outline.rows
}

fn with_props(
    mut it: crate::model::Item,
    types: &[&str],
    props: &[(&str, &str)],
) -> crate::model::Item {
    it.types = types.iter().map(|t| t.to_string()).collect();
    for (k, v) in props {
        it.properties.insert(k.to_string(), (*v).into());
    }
    it
}

// タスクは begin と end から自分の状態を報告する。
#[test]
fn a_task_reports_its_state_from_begin_and_end() {
    let h = Harness::with_items(vec![
        item("plain", 1, "text"),
        with_props(item("todo", 1, "text"), &["task"], &[]),
        with_props(
            item("running", 1, "text"),
            &["task"],
            &[("begin", "2026-01-01T00:00:00Z")],
        ),
        with_props(
            item("done", 1, "text"),
            &["task"],
            &[
                ("begin", "2026-01-01T00:00:00Z"),
                ("end", "2026-01-01T01:00:00Z"),
            ],
        ),
    ]);
    let states: Vec<Option<TaskState>> = rows(&h).iter().map(|r| r.task).collect();
    assert_eq!(
        states,
        [
            None,
            Some(TaskState::NotStarted),
            Some(TaskState::Running),
            Some(TaskState::Done)
        ]
    );
}

// 一行だけの見出しは見出し行であるが、続きの行がある見出しはそうではない。
#[test]
fn a_one_line_heading_is_a_heading_line_but_a_heading_with_more_lines_is_not() {
    let h = Harness::with_items(vec![
        item("one", 1, "## Title"),
        item("many", 1, "## Title\nmore"),
        item("text", 1, "plain"),
    ]);
    let r = rows(&h);
    assert_eq!((r[0].heading_level, r[0].heading_line), (2, true));
    assert_eq!((r[1].heading_level, r[1].heading_line), (2, false));
    assert_eq!((r[2].heading_level, r[2].heading_line), (0, false));
}

// 行の1行目の倍率は、見出しの倍率である。編集中は、本文全体が1行の見出しのときだけ見出しの倍率になる（#795）。
#[test]
fn the_first_line_of_a_row_has_the_scale_of_its_heading_and_while_edited_only_on_a_heading_line() {
    let mut h = Harness::with_items(vec![
        item("one", 1, "## Title"),
        item("many", 1, "## Title\nmore"),
        item("edited", 1, "## Title\nmore"),
        item("text", 1, "plain"),
    ]);
    h.model.state.add_editing_item("edited");
    let scales: Vec<_> = rows(&h).iter().map(|r| r.first_line_scale).collect();
    assert_eq!(scales, [1.4, 1.4, 1.0, 1.0]);
}

// ズームの変更で倍率が表示に反映されて保存され、リセットで 1.0 に戻る。
#[test]
fn set_zoom_shows_and_saves_the_factor() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.send(Event::ZoomStep {
        item_id: "a".into(),
        up: true,
    });
    assert_eq!(h.model.state.get_item_by_id("a").unwrap().zoom(), 1.1);
    assert!(h.api.calls.iter().any(|c| matches!(c, ApiOp::UpsertItemProperty { key, value, .. } if key == "zoom" && value == "1.1")));
    h.send(Event::ResetZoom {
        item_id: "a".into(),
    });
    assert_eq!(h.model.state.get_item_by_id("a").unwrap().zoom(), 1.0);
}

// 自前の zoom を持つ項目だけが zoom を持つとされる。子は zoom を継承するが自前のものではない。
#[test]
fn only_an_item_with_a_zoom_of_its_own_has_one() {
    let h = Harness::with_items(vec![
        with_props(item("zoomed", 1, "a"), &[], &[("zoom", "1.5")]),
        item("child", 2, "b"),
    ]);
    let r = rows(&h);
    assert!(r[0].has_own_zoom);
    assert!(
        !r[1].has_own_zoom,
        "the child inherits the zoom but has none of its own"
    );
    assert_eq!(r[1].effective_zoom, 1.5);
}

// 項目へのスクロールの求めは、なめらかなスクロールとしてシェルに渡る。
#[test]
fn a_scroll_request_is_passed_on_as_a_smooth_scroll() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.send(Event::ScrollToItemRequested {
        item_id: "a".into(),
    });
    assert!(h.ui.iter().any(
        |u| matches!(u, crate::effects::UiOp::ScrollToItem { item_id, smooth: true } if item_id == "a")
    ));
}

// coreが本文から作るASTでは、相対パスのリンクはwikilinkに、#tagはハッシュタグになる。
#[test]
fn the_ast_that_the_core_makes_of_a_body_has_wikilinks_and_hashtags() {
    use flatmarkdown_ast::Node;
    let ast = crate::asts::parse("**b** [p](../p.md) #t");
    let Node::Document { children } = &ast else {
        panic!("not a document: {ast:?}");
    };
    let kinds: Vec<&str> = children[0].children().iter().map(Node::type_name).collect();
    assert_eq!(kinds, ["strong", "text", "wikilink", "text", "hashtag"]);
}

// 行は、本文から作った描画モデルを持つ。
#[test]
fn a_row_has_the_body_model_of_its_body() {
    let h = Harness::with_items(vec![item("a", 1, "**b**"), item("c", 1, "d")]);
    let models: Vec<_> = rows(&h).into_iter().map(|row| row.body_model).collect();
    assert_eq!(models, [shown("**b**"), shown("d")]);
}

// 本文が変わると、行の描画モデルも新しい本文のものになる。前と同じ本文に戻せば、前と同じ描画モデルになる。
#[test]
fn the_body_model_of_a_row_follows_its_body() {
    let mut h = Harness::with_items(vec![item("a", 1, "one")]);
    let first = rows(&h)[0].body_model.clone();
    crate::harness::typed(&mut h, "a", "# two");
    assert_eq!(rows(&h)[0].body_model, shown("# two"));
    crate::harness::typed(&mut h, "a", "one");
    assert_eq!(rows(&h)[0].body_model, first);
}

// 同じ本文の行が2つあれば、どちらもその本文の描画モデルを持つ。
#[test]
fn rows_with_the_same_body_have_the_same_body_model() {
    let h = Harness::with_items(vec![item("a", 1, "*x*"), item("b", 1, "*x*")]);
    let models: Vec<_> = rows(&h).into_iter().map(|row| row.body_model).collect();
    assert_eq!(models, [shown("*x*"), shown("*x*")]);
}

// タグのフォルダーが変わると、タグのリンクは新しいフォルダーの下を指す。
#[test]
fn the_links_of_tags_follow_the_folder_of_the_tags() {
    use crate::rich_text::{RichBlock, RichLink};
    let mut h = Harness::with_items(vec![item("a", 1, "#topic")]);
    let link = |h: &Harness| match &rows(h)[0].body_model.blocks[..] {
        [RichBlock::Text { paragraphs }] => paragraphs[0].runs[0].link.clone(),
        blocks => panic!("not one text: {blocks:?}"),
    };
    h.model.config.tags_dir = "tags/".into();
    let page = |url: &str| Some(RichLink::Page { url: url.into() });
    assert_eq!(link(&h), page("/pages/tags/topic"));
    h.model.config.tags_dir = "labels/".into();
    assert_eq!(link(&h), page("/pages/labels/topic"));
}
