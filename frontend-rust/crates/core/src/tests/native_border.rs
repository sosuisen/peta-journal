//! What crosses to a native shell as bincode (ADR 024): the events it sends and the view model
//! it draws.

use crate::app::{Event, Route, Screen};
use crate::harness::{item, Harness};
use crate::rest::ClientId;
use crux_core::bridge::{BincodeFfiFormat, FfiFormat};

/// The event as it is after a trip through the format of the border.
fn sent(event: &Event) -> Event {
    let mut bytes = Vec::new();
    BincodeFfiFormat::serialize(&mut bytes, event).unwrap();
    BincodeFfiFormat::deserialize(&bytes).unwrap()
}

// シェルが送るイベントは、境界の形式に書いて、同じイベントに読み戻せる。
#[test]
fn the_events_of_the_shell_arrive_as_they_were_sent() {
    let events = [
        Event::Start {
            client_id: ClientId::new(),
        },
        Event::Route {
            route: Route::Page {
                name: "notes".into(),
                fragment: Some("a".into()),
            },
            today: "2026-02-10".into(),
        },
        Event::BodyChanged {
            item_id: "a".into(),
            body: "typed".into(),
            now_ms: 1_000,
        },
    ];
    for event in events {
        assert_eq!(format!("{:?}", sent(&event)), format!("{event:?}"));
    }
}

// ビューモデルは、境界の形式に書ける。項目の本文も、ページのプロパティも入る。
#[test]
fn the_view_model_is_written_for_the_shell() {
    let mut h = Harness::with_items(vec![item("a", 1, "the body of a")]);
    if let Screen::Page(p) = &mut h.model.screen {
        let page = p.page.as_mut().unwrap();
        page.properties.insert("tags".into(), ["x"].into());
    }
    let mut bytes = Vec::new();
    BincodeFfiFormat::serialize(&mut bytes, &h.view()).unwrap();
    let written = String::from_utf8_lossy(&bytes);
    assert!(written.contains("the body of a"));
    assert!(written.contains("tags"));
}
