//! The event stream from the server: the connection state and reconnecting.

use crate::app::{ConnectionState, Event};
use crate::effects::{ApiOp, SseEvent, TimerOp, UiOp};
use crate::harness::Harness;

fn started() -> Harness {
    let mut h = Harness::new();
    h.send(Event::Start);
    h
}

fn sse(h: &mut Harness, event: SseEvent) {
    h.send(Event::Sse(event));
}

fn pending_delays_ms(h: &Harness) -> Vec<u32> {
    h.timers
        .iter()
        .filter_map(|t| match t.operation {
            TimerOp::Start { ms, .. } => Some(ms),
            _ => None,
        })
        .collect()
}

// 起動でサーバーのストリームにつなぎ、接続中になる。設定、プロパティ定義、最近のページも読み込まれる。
#[test]
fn starting_connects_to_the_stream_and_loads_the_settings() {
    let h = started();
    assert_eq!(h.view().connection, ConnectionState::Connecting);
    assert_eq!(h.sse.len(), 1);
    for op in ["GetConfig", "GetPropertyDefinitions", "GetRecentPages"] {
        assert!(h.api.called(op), "{op}");
    }
}

// ストリームが開くと接続済みになり、最近のページが読み直される。
#[test]
fn an_open_stream_is_connected() {
    let mut h = started();
    let recent_loads = h.api.calls_named("GetRecentPages").len();
    sse(&mut h, SseEvent::Open);
    assert_eq!(h.view().connection, ConnectionState::Connected);
    assert_eq!(h.api.calls_named("GetRecentPages").len(), recent_loads + 1);
}

// 切れると状態はエラーになり、間隔を倍にしながらつなぎ直す。間隔は30秒を超えず、10回で諦める。
#[test]
fn a_broken_stream_reconnects_with_a_growing_delay_and_gives_up_after_ten_tries() {
    let mut h = started();
    let mut delays = Vec::new();
    for attempt in 1..=10 {
        sse(&mut h, SseEvent::Error);
        assert_eq!(h.view().connection, ConnectionState::Error);
        delays.extend(pending_delays_ms(&h));
        h.fire_timers();
        assert_eq!(h.view().connection, ConnectionState::Connecting);
        assert_eq!(h.sse.len(), 1 + attempt);
    }
    assert_eq!(
        delays,
        [1000, 2000, 4000, 8000, 16000, 30000, 30000, 30000, 30000, 30000]
    );

    sse(&mut h, SseEvent::Error);
    assert!(pending_delays_ms(&h).is_empty(), "no eleventh try");
    assert_eq!(h.view().connection, ConnectionState::Error);
}

// 一度つながると、次に切れたときの間隔は最初からになる。
#[test]
fn a_connection_resets_the_delay() {
    let mut h = started();
    sse(&mut h, SseEvent::Error);
    h.fire_timers();
    sse(&mut h, SseEvent::Open);
    sse(&mut h, SseEvent::Error);
    assert_eq!(pending_delays_ms(&h), [1000]);
}

// pagesUpdated でないメッセージや JSON でないメッセージでは何も起きない。pagesUpdated ではサイドバーが読み直される。
#[test]
fn only_a_pages_updated_message_refreshes_the_sidebar() {
    let mut h = Harness::with_items(vec![]);
    sse(&mut h, SseEvent::Message("not json".into()));
    sse(&mut h, SseEvent::Message(r#"{"type":"hello"}"#.into()));
    assert!(!h.api.called("GetRecentPages"));

    sse(
        &mut h,
        SseEvent::Message(r#"{"type":"pagesUpdated","pageId":"x","source":"app"}"#.into()),
    );
    assert!(h.api.called("GetRecentPages"));
}

// サーバーの拡張が確認を求めると、ユーザーに尋ね、その答えを requestId 付きでサーバーに返す。
#[test]
fn a_confirm_from_the_server_asks_the_user_and_sends_the_answer_back() {
    for answer in [true, false] {
        let mut h = Harness::with_items(vec![]);
        h.confirm = answer;

        sse(
            &mut h,
            SseEvent::Message(
                r#"{"type":"confirm","requestId":"confirm-7","message":"Insert?"}"#.into(),
            ),
        );

        assert!(h
            .ui
            .iter()
            .any(|op| matches!(op, UiOp::Confirm(m) if m == "Insert?")));
        assert_eq!(
            h.api.calls_named("AnswerConfirm"),
            vec![&ApiOp::AnswerConfirm {
                request_id: "confirm-7".into(),
                ok: answer,
            }]
        );
    }
}
