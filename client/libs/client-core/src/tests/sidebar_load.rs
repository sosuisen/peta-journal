//! Whether the lists of the sidebar are loading, loaded or failed (ADR 085).

use crate::app::Event;
use crate::effects::ApiOutput;
use crate::harness::Harness;
use crate::model::Page;
use crate::rest::ClientId;
use crate::view::LoadState;

fn doc(dir_path: &str, name: &str) -> Page {
    Page {
        id: format!("{dir_path}{name}"),
        dir_path: dir_path.into(),
        name: name.into(),
        page_type: "doc".into(),
        ..Page::default()
    }
}

fn start(h: &mut Harness) {
    h.send(Event::Start {
        client_id: ClientId::new(),
    });
}

/// The labels of the recent pages, and their state.
fn recent(h: &Harness) -> (Vec<String>, LoadState) {
    let view = h.view().recent_pages;
    let labels = view.pages.iter().map(|p| p.label.clone()).collect();
    (labels, view.state)
}

// 最近のページは、最初の答えが届くまで、空の一覧で読み込み中である。
#[test]
fn the_recent_pages_are_loading_and_empty_until_the_first_answer() {
    let mut h = Harness::new();
    h.hold("GetRecentPages");

    start(&mut h);

    assert_eq!(recent(&h), (vec![], LoadState::Loading));
}

// 最近のページは、答えが届くと、その一覧で読み込み済みになる。
#[test]
fn the_recent_pages_are_loaded_with_the_answer() {
    let mut h = Harness::new();
    h.api.answer_next(
        "GetRecentPages",
        Ok(ApiOutput::Pages(vec![doc("", "Idea")])),
    );

    start(&mut h);

    assert_eq!(recent(&h), (vec!["Idea".to_string()], LoadState::Loaded));
}
