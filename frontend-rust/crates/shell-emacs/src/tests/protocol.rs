use crate::protocol::{Bullet, Envelope, Inbound, Outbound, RowView};
use peta_core::view::TaskState;
use serde_json::json;

fn row(bullet: Bullet) -> RowView {
    RowView {
        id: "a".into(),
        level: 2,
        body: "本文".into(),
        bullet,
        collapsed: false,
        header: None,
    }
}

// Emacsからの1行は、通し番号と操作に分けて読まれる。
#[test]
fn a_line_from_emacs_has_a_sequence_number_and_a_message() {
    let line = r#"{"seq":7,"type":"body_changed","id":"a","body":"日本語"}"#;
    let envelope: Envelope = serde_json::from_str(line).unwrap();
    assert_eq!(envelope.seq, 7);
    assert_eq!(
        envelope.message,
        Inbound::BodyChanged {
            id: "a".into(),
            body: "日本語".into()
        }
    );
}

// 答えのない質問は、値がnullの答えとして届く。
#[test]
fn an_answer_may_have_no_value() {
    let line = r#"{"seq":1,"type":"answer","req":3,"value":null}"#;
    let envelope: Envelope = serde_json::from_str(line).unwrap();
    assert_eq!(
        envelope.message,
        Inbound::Answer {
            req: 3,
            value: serde_json::Value::Null
        }
    );
}

// 行は、Emacsが読むJSONとして書き出され、日本語はエスケープされない。
#[test]
fn rows_are_written_as_json_for_emacs() {
    let message = Outbound::Rows {
        ack: 4,
        page_id: Some("p".into()),
        rows: vec![row(Bullet::Plain)],
    };
    assert_eq!(
        serde_json::to_value(&message).unwrap(),
        json!({
            "t": "rows", "ack": 4, "page_id": "p",
            "rows": [{
                "id": "a", "level": 2, "body": "本文", "bullet": "plain",
                "collapsed": false, "header": null,
            }],
        })
    );
    assert!(serde_json::to_string(&message).unwrap().contains("本文"));
}

// バレットの種類は、壊れた参照、参照、タスクの状態、通常の順で決まる。
#[test]
fn the_bullet_shows_what_the_row_is_in_order_of_importance() {
    let task = Some(TaskState::Running);
    assert_eq!(Bullet::of(true, true, task), Bullet::Broken);
    assert_eq!(Bullet::of(false, true, task), Bullet::Transclusion);
    assert_eq!(Bullet::of(false, false, task), Bullet::Running);
    assert_eq!(
        Bullet::of(false, false, Some(TaskState::NotStarted)),
        Bullet::Task
    );
    assert_eq!(Bullet::of(false, false, Some(TaskState::Done)), Bullet::Done);
    assert_eq!(Bullet::of(false, false, None), Bullet::Plain);
}
