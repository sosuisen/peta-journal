//! Blank lines in a body. A blank line between two blocks would end the item in its file, so
//! the file has a line of one backslash in its place (ADR 070). The shell sees blank lines.

use std::collections::BTreeMap;

use crate::app::{Event, Route};
use crate::effects::{ApiOp, ApiOutput};
use crate::harness::{item, split_calls, typed, update_calls, Harness};
use crate::model::{
    BacklinkEntry, BatchOpInput, Batched, DocPage, Item, ItemPosition, JournalBefore,
    MovedToJournal, Page, PageWithItems, PasteCursor, Pasted, Split, Stashed, TranscludedTrees,
};

fn notes() -> Page {
    Page {
        id: "notes".into(),
        name: "notes".into(),
        page_type: "doc".into(),
        ..Default::default()
    }
}

/// Opens the page `notes`, whose file holds an item with this body.
fn open_with_body(h: &mut Harness, body: &str) {
    h.api.answer_next(
        "GetOrCreateDoc",
        Ok(ApiOutput::DocPage(DocPage {
            page: notes(),
            items: vec![],
            created: false,
        })),
    );
    h.api.answer_next(
        "GetPageById",
        Ok(ApiOutput::Page(PageWithItems {
            page: notes(),
            items: vec![item("a", 1, body)],
            created: false,
            transcluded_trees: None,
        })),
    );
    h.send(Event::Route {
        route: Route::Page {
            name: "notes".into(),
            fragment: None,
        },
        today: "2026-02-10".into(),
    });
}

// ファイルの本文の`\`だけの行は、画面では空行になる。
#[test]
fn a_backslash_line_of_a_loaded_body_is_shown_as_a_blank_line() {
    let mut h = Harness::new();

    open_with_body(&mut h, "a\n\\\nb");

    assert_eq!(h.bodies(), ["a=a\n\nb"]);
}

// 自動保存は、本文の途中の空行を`\`だけの行として送る。ファイルで項目が分かれないためである。
#[test]
fn the_auto_save_sends_a_blank_line_as_a_backslash_line() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    typed(&mut h, "a", "a\n\nb");

    h.fire_timers();

    assert_eq!(
        update_calls(&h.api),
        [("a".into(), Some("a\n\\\nb".into()))]
    );
}

// `\`だけの行は、`\`を1つ足して送る。読み戻すときに、空行と区別するためである。
#[test]
fn a_line_of_backslashes_is_sent_with_one_more() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    typed(&mut h, "a", "a\n\\");

    h.fire_timers();

    assert_eq!(update_calls(&h.api), [("a".into(), Some("a\n\\\\".into()))]);
}

// 分割は、ファイルの形の本文と、その本文の中で同じ場所を指すカーソルを送る。空行が`\`だけの行になると、後ろの文字の位置がずれるためである。
#[test]
fn a_split_sends_the_body_of_the_file_and_the_cursor_in_it() {
    let mut h = Harness::with_items(vec![item("a", 1, "a\n\nb\n\\c")]);
    h.model.state.add_editing_item("a");

    let now_ms = h.now_ms;
    h.send(Event::EnterPressed {
        item_id: "a".into(),
        cursor: "a\n\nb\n\\".len(),
        now_ms,
    });

    assert_eq!(
        split_calls(&h.api),
        [("a".to_string(), "a\n\\\nb\n\\c".to_string(), 7)]
    );
}

// 空行の上での分割は、その行の`\`の前で分ける。空行は、新しい項目の先頭に移る。
#[test]
fn a_split_on_a_blank_line_is_before_its_backslash() {
    let mut h = Harness::with_items(vec![item("a", 1, "a\n  \nb")]);
    h.model.state.add_editing_item("a");

    let now_ms = h.now_ms;
    h.send(Event::EnterPressed {
        item_id: "a".into(),
        cursor: "a\n ".len(),
        now_ms,
    });

    assert_eq!(
        split_calls(&h.api),
        [("a".to_string(), "a\n\\\nb".to_string(), 2)]
    );
}

// 分割の答えの本文の`\`だけの行は、画面では空行になる。
#[test]
fn the_bodies_of_a_split_answer_are_shown_with_blank_lines() {
    let mut h = Harness::with_items(vec![item("a", 1, "a\n\nb\nc")]);
    h.model.state.add_editing_item("a");
    let split = Split {
        item: item("a", 1, "a\n\\\nb"),
        created: item("n", 1, "\\\nc"),
    };
    h.api
        .answer_next("SplitItem", Ok(ApiOutput::Split(Box::new(split))));

    let now_ms = h.now_ms;
    h.send(Event::EnterPressed {
        item_id: "a".into(),
        cursor: "a\n\nb\n".len(),
        now_ms,
    });

    assert_eq!(h.bodies(), ["a=a\n\nb", "n=\nc"]);
}

// 貼り付けは、ファイルの形の本文とカーソルを送る。答えの本文とカーソルは、画面の形に直る。
#[test]
fn a_paste_sends_the_body_of_the_file_and_puts_the_cursor_back_on_the_screen() {
    let mut h = Harness::with_items(vec![item("a", 1, "a\n\nb")]);
    let pasted = Pasted {
        items: vec![item("a", 1, "a\n\\\nbX")],
        cursor: PasteCursor {
            item_id: "a".into(),
            position: "a\n\\\nbX".len(),
        },
    };
    h.api
        .answer_next("PasteText", Ok(ApiOutput::Pasted(pasted)));

    h.send(Event::PasteText {
        item_id: "a".into(),
        text: "X".into(),
        html: String::new(),
        cursor: "a\n\nb".len(),
        raw: false,
    });

    assert!(matches!(
        &h.api.calls[0],
        ApiOp::PasteText { body, cursor: 5, .. } if body == "a\n\\\nb"
    ));
    assert_eq!(h.bodies(), ["a=a\n\nbX"]);
    assert_eq!(h.focus_cursor(), Some("a\n\nbX".len()));
}

/// An item whose body in the file has a backslash line.
fn from_file(id: &str) -> Item {
    item(id, 1, "a\n\\\nb")
}

/// The bodies of every item an answer has, in no order.
fn bodies_of(output: &ApiOutput) -> Vec<String> {
    let trees = |trees: &Option<TranscludedTrees>| -> Vec<Item> {
        trees
            .iter()
            .flat_map(|t| t.values().flatten().cloned())
            .collect()
    };
    let items: Vec<Item> = match output {
        ApiOutput::Page(p) => [p.items.clone(), trees(&p.transcluded_trees)].concat(),
        ApiOutput::JournalBefore(j) => {
            let trees = j.transcluded_trees.values().flatten().cloned();
            j.items.iter().cloned().chain(trees).collect()
        }
        ApiOutput::Item(i) => vec![i.clone()],
        ApiOutput::Items(items) => items.clone(),
        ApiOutput::Backlinks(entries) => entries
            .iter()
            .filter_map(|e| e.source_item.clone())
            .collect(),
        ApiOutput::Stashed(s) => [vec![s.heading.clone()], s.items.clone()].concat(),
        ApiOutput::Batched(b) => [b.items.clone(), b.created.clone()].concat(),
        ApiOutput::DocPage(d) => d.items.clone(),
        ApiOutput::MovedToJournal(m) => m.items.clone(),
        other => panic!("not an answer with items: {other:?}"),
    };
    items.into_iter().map(|i| i.body).collect()
}

// サーバーのどの答えの項目も、本文の`\`だけの行は空行になる。トランスクルージョンの元の項目、バックリンクの項目も同じである。
#[test]
fn every_item_of_an_answer_comes_with_blank_lines() {
    let trees = || {
        Some(TranscludedTrees::from([(
            "s".to_string(),
            vec![from_file("s")],
        )]))
    };
    let answers = [
        ApiOutput::Page(PageWithItems {
            page: notes(),
            items: vec![from_file("a")],
            created: false,
            transcluded_trees: trees(),
        }),
        ApiOutput::JournalBefore(JournalBefore {
            found: true,
            page: Some(notes()),
            items: vec![from_file("a")],
            transcluded_trees: BTreeMap::from([("s".to_string(), vec![from_file("s")])]),
        }),
        ApiOutput::Item(from_file("a")),
        ApiOutput::Items(vec![from_file("a"), from_file("b")]),
        ApiOutput::Backlinks(vec![BacklinkEntry {
            source_page: notes(),
            source_item: Some(from_file("a")),
        }]),
        ApiOutput::Stashed(Box::new(Stashed {
            heading: from_file("h"),
            created: false,
            items: vec![from_file("a")],
        })),
        ApiOutput::Batched(Box::new(Batched {
            items: vec![from_file("a")],
            created: vec![from_file("b")],
        })),
        ApiOutput::DocPage(DocPage {
            page: notes(),
            items: vec![from_file("a")],
            created: false,
        }),
        ApiOutput::MovedToJournal(MovedToJournal {
            page: notes(),
            created: false,
            items: vec![from_file("a")],
        }),
    ];

    for answer in answers {
        let shown = crate::file_body::output_to_shell(answer.clone());

        let bodies = bodies_of(&shown);
        assert!(
            !bodies.is_empty() && bodies.iter().all(|b| b == "a\n\nb"),
            "{answer:?} came as {bodies:?}"
        );
    }
}

/// The bodies every request carries, in no order.
fn bodies_sent(op: &ApiOp) -> Vec<String> {
    match op {
        ApiOp::CreateItem { body, .. } => vec![body.clone()],
        ApiOp::MoveItems { body, .. }
        | ApiOp::MoveItemsToJournal { body, .. }
        | ApiOp::StashItem { body, .. } => body.iter().cloned().collect(),
        ApiOp::MergeItems {
            body, into_body, ..
        } => body.iter().chain(into_body).cloned().collect(),
        ApiOp::ApplyBatch { ops } => ops
            .iter()
            .filter_map(|op| match op {
                BatchOpInput::Create { body, .. } => Some(body.clone()),
                BatchOpInput::Update { body, .. } => body.clone(),
                _ => None,
            })
            .collect(),
        other => panic!("not a request with bodies: {other:?}"),
    }
}

// サーバーへのどの要求の本文も、空行は`\`だけの行になる。作成、移動、結合、stash、undoのbatchである。
#[test]
fn every_body_of_a_request_goes_with_backslash_lines() {
    let body = || "a\n\nb".to_string();
    let requests = [
        ApiOp::CreateItem {
            page_id: "notes".into(),
            position: ItemPosition::Last,
            level: 1,
            body: body(),
            id: None,
        },
        ApiOp::MoveItems {
            item_ids: vec!["a".into()],
            target_page_id: "notes".into(),
            position: ItemPosition::Last,
            level_delta: None,
            body: Some(body()),
        },
        ApiOp::MoveItemsToJournal {
            file_name: "2026-02-10".into(),
            item_ids: vec!["a".into()],
            position: ItemPosition::Last,
            body: Some(body()),
        },
        ApiOp::MergeItems {
            item_id: "a".into(),
            into_id: "b".into(),
            body: Some(body()),
            into_body: Some(body()),
        },
        ApiOp::StashItem {
            item_id: "a".into(),
            body: Some(body()),
        },
        ApiOp::ApplyBatch {
            ops: vec![
                BatchOpInput::Create {
                    id: "a".into(),
                    page_id: "notes".into(),
                    position: ItemPosition::Last,
                    level: 1,
                    body: body(),
                    transclude_item_id: None,
                    properties: Default::default(),
                },
                BatchOpInput::Update {
                    id: "b".into(),
                    body: Some(body()),
                    level: None,
                    collapsed: None,
                    task: None,
                },
            ],
        },
    ];

    for request in requests {
        let sent = crate::file_body::op_to_file(request.clone());

        let bodies = bodies_sent(&sent);
        assert!(
            !bodies.is_empty() && bodies.iter().all(|b| b == "a\n\\\nb"),
            "{request:?} went as {bodies:?}"
        );
    }
}
