//! What crosses to a native shell as bincode (ADR 024): the events it sends and the view model
//! it draws.

use crate::app::{Event, Route, Screen};
use crate::harness::{item, Harness};
use crate::model::ItemPosition;
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
            opens_new_day: true,
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

// 項目の行き先は、境界の形式では、番号の付いた選択肢として書かれる。JSONでは、サーバーが読む文字列のままである。
// ネイティブのシェルの型は、Rustの型の形から生成されるので、文字列で書くと読み手とずれる。
#[test]
fn the_place_of_an_item_is_written_as_a_choice_with_a_number() {
    let place = ItemPosition::after("x");

    let mut bytes = Vec::new();
    BincodeFfiFormat::serialize(&mut bytes, &place).unwrap();
    // The third choice, then the length of the ID and the ID
    assert_eq!(bytes, [2, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, b'x']);
    let read: ItemPosition = BincodeFfiFormat::deserialize(&bytes).unwrap();
    assert_eq!(read, place);

    let json = serde_json::to_string(&place).unwrap();
    assert_eq!(json, r#""after:x""#);
    assert_eq!(serde_json::from_str::<ItemPosition>(&json).unwrap(), place);
    assert!(serde_json::from_str::<ItemPosition>(r#""nowhere""#).is_err());
}
