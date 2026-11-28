//! The event stream from the server: the connection state and reconnecting.

use crate::app::{ConnectionState, Event};
use crate::effects::{ApiOp, SseEvent, TimerOp, UiOp};
use crate::harness::Harness;
use crate::screens::day::CHECK_MS;

fn started() -> Harness {
    let mut h = Harness::new();
    h.send(Event::Start {
        client_id: crate::rest::ClientId::new(),
        opens_new_day: true,
    });
    h
}

fn sse(h: &mut Harness, event: SseEvent) {
    h.send(Event::Sse(event));
}

/// The delays of the timers that wait to reconnect. The timer that waits to look at the
/// clock again runs beside them all the time.
fn pending_delays_ms(h: &Harness) -> Vec<u32> {
    h.timers
        .iter()
        .filter_map(|t| match t.operation {
            TimerOp::Start { ms, .. } if ms != CHECK_MS => Some(ms),
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

// 切れると状態はエラーになり、間隔を倍にしながらつなぎ直す。間隔は30秒を超えず、つながるまで諦めない。サーバーがあとから上がることがあるためである（#756）。
#[test]
fn a_broken_stream_reconnects_with_a_growing_delay_until_it_connects() {
    let mut h = started();
    let mut delays = Vec::new();
    for attempt in 1..=40 {
        sse(&mut h, SseEvent::Error);
        assert_eq!(h.view().connection, ConnectionState::Error);
        delays.extend(pending_delays_ms(&h));
        h.fire_timers();
        assert_eq!(h.view().connection, ConnectionState::Connecting);
        assert_eq!(h.sse.len(), 1 + attempt);
    }
    assert_eq!(delays[..6], [1000, 2000, 4000, 8000, 16000, 30000]);
    // Past the 32nd try, a doubling of the delay would no longer fit in a u32
    assert!(delays[6..].iter().all(|&d| d == 30000));
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

// 知らない出どころの変更では、サイドバーは読み直されるが、ページは読み直されない。
#[test]
fn a_change_from_a_source_that_is_not_known_refreshes_the_sidebar_and_not_the_page() {
    let mut h = Harness::with_items(vec![crate::harness::item("a", 1, "text")]);
    let page_id = crate::harness::item("a", 1, "text").page_id;

    sse(
        &mut h,
        SseEvent::Message(format!(
            r#"{{"type":"pagesUpdated","pageId":"{page_id}","source":"later"}}"#
        )),
    );

    assert!(h.api.called("GetRecentPages"));
    assert!(!h.api.called("GetPageById"));
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

/// What the server answers to the next read of the definitions: one user key.
fn next_definitions(h: &mut Harness, key: &str) {
    h.api.answer_next(
        "GetPropertyDefinitions",
        Ok(crate::effects::ApiOutput::PropertyDefinitions(vec![
            crate::model::PropertyDefinition {
                key: key.into(),
                value_type: "number".into(),
                system: false,
                page: true,
            },
        ])),
    );
}

fn shown_keys(h: &Harness) -> Vec<String> {
    h.view()
        .config
        .property_definitions
        .iter()
        .map(|d| d.key.clone())
        .collect()
}

// サーバーが定義の変わったことを知らせると、定義を読み直し、画面の定義が新しくなる（ADR 046）。
#[test]
fn a_message_that_the_definitions_changed_reads_them_again() {
    let mut h = started();
    next_definitions(&mut h, "score");

    sse(
        &mut h,
        SseEvent::Message(r#"{"type":"propertyDefinitionsUpdated"}"#.into()),
    );

    assert_eq!(shown_keys(&h), ["score"]);
}

// ストリームがつながるたびに、定義も読み直される。切れていた間の知らせは届かないからである。
#[test]
fn an_open_stream_reads_the_definitions_again() {
    let mut h = started();
    sse(&mut h, SseEvent::Error);
    h.fire_timers();
    next_definitions(&mut h, "score");

    sse(&mut h, SseEvent::Open);

    assert_eq!(shown_keys(&h), ["score"]);
}

fn visit(h: &mut Harness, name: &str) {
    h.send(Event::Route {
        route: crate::app::Route::Page {
            name: name.into(),
            fragment: None,
        },
        today: "2026-02-10".into(),
    });
}

/// The server was down when the shell started: the stream broke, and comes back once the
/// server is up.
fn server_comes_up(h: &mut Harness) {
    sse(h, SseEvent::Error);
    h.fire_timers();
    sse(h, SseEvent::Open);
}

// サーバーが落ちていてページを読めなかったとき、つなぎ直してストリームが開くと、そのページを開き直す（#756）。
#[test]
fn a_page_that_could_not_be_loaded_opens_once_the_server_is_up() {
    let mut h = started();
    h.api.fail_next("GetOrCreateDoc", "connection refused");
    visit(&mut h, "Notes");
    assert_eq!(h.model.shown_page_id(), None);

    server_comes_up(&mut h);

    assert_eq!(h.model.shown_page_id().as_deref(), Some("page-Notes"));
}

// サーバーが落ちていてジャーナルを読めなかったとき、つなぎ直してストリームが開くと、そのジャーナルを開き直す（#756）。
#[test]
fn a_journal_that_could_not_be_loaded_opens_once_the_server_is_up() {
    let mut h = started();
    h.api.fail_next("GetOrCreateJournal", "connection refused");
    visit(&mut h, "2026-02-09");
    assert_eq!(h.model.shown_page_id(), None);

    server_comes_up(&mut h);

    assert_eq!(h.model.shown_page_id().as_deref(), Some("page-2026-02-09"));
}

// 表示中のページは、つなぎ直してストリームが開くと読み直される。切れていた間の変更は知らされないからである。開き直しはしないので、元に戻す履歴と編集は残る。
#[test]
fn the_shown_page_is_read_again_once_the_stream_is_back() {
    let mut h = Harness::with_items(vec![crate::harness::item("a", 1, "old")]);

    server_comes_up(&mut h);

    assert_eq!(h.bodies(), ["reloaded=reloaded"]);
    assert!(!h.api.called("GetOrCreateDoc"));
}

// 表示中のジャーナルも、つなぎ直してストリームが開くと読み直され、開き直されはしない。
#[test]
fn the_shown_journal_is_read_again_and_not_opened_again_once_the_stream_is_back() {
    let mut h = started();
    visit(&mut h, "2026-02-09");

    server_comes_up(&mut h);

    assert_eq!(h.api.calls_named("GetOrCreateJournal").len(), 1);
    assert_eq!(
        h.api.calls_named("GetPageById"),
        [&ApiOp::GetPageById {
            page_id: "page-2026-02-09".into()
        }]
    );
}

// つなぎ直してストリームが開くと、サーバーの設定も読み直される。起動のときに読めなかったかもしれないからである（#756）。
#[test]
fn the_server_config_is_read_again_once_the_stream_is_back() {
    let mut h = started();
    let loads = h.api.calls_named("GetConfig").len();

    server_comes_up(&mut h);

    assert_eq!(h.api.calls_named("GetConfig").len(), loads + 1);
}

// 起動して最初にストリームが開いたときは、ページを読み直さない。起動のときに読んでいるからである。
#[test]
fn the_first_open_of_the_stream_reads_no_page_again() {
    let mut h = Harness::with_items(vec![crate::harness::item("a", 1, "old")]);
    h.send(Event::Start {
        client_id: crate::rest::ClientId::new(),
        opens_new_day: true,
    });

    sse(&mut h, SseEvent::Open);

    assert!(!h.api.called("GetPageById"));
    assert_eq!(h.api.calls_named("GetConfig").len(), 1);
}
