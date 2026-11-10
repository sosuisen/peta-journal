//! What crosses to a native shell as bincode (ADR 024): the events it sends and the view model
//! it draws.

use crate::app::Event;
use crux_core::bridge::{BincodeFfiFormat, FfiFormat};

/// The event as it is after a trip through the format of the border.
fn sent(event: &Event) -> Event {
    let mut bytes = Vec::new();
    BincodeFfiFormat::serialize(&mut bytes, event).unwrap();
    BincodeFfiFormat::deserialize(&bytes).unwrap()
}

// シェルが送るイベントは、境界の形式に書いて、同じイベントに読み戻せる。
#[test]
fn an_event_of_the_shell_arrives_as_it_was_sent() {
    let event = Event::BodyChanged {
        item_id: "a".into(),
        body: "typed".into(),
        now_ms: 1_000,
    };
    assert_eq!(format!("{:?}", sent(&event)), format!("{event:?}"));
}
