use crate::protocol::{Bullet, Envelope, Inbound, Outbound, RowView, TaskMark};
use client_core::view::TaskState;
use serde_json::json;

fn row(bullet: Bullet) -> RowView {
    RowView {
        id: "a".into(),
        level: 2,
        body: "本文".into(),
        bullet,
        task: None,
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

// 質問への答えは、はいかいいえとして届く。
#[test]
fn an_answer_is_yes_or_no() {
    let line = r#"{"seq":1,"type":"answer","req":3,"yes":false}"#;
    let envelope: Envelope = serde_json::from_str(line).unwrap();
    assert_eq!(envelope.message, Inbound::Answer { req: 3, yes: false });
}

// 名前の変更は、フォルダを含む新しいパスとして届く。
#[test]
fn a_rename_carries_the_new_path() {
    let line = r#"{"seq":1,"type":"rename_page","path":"a/b"}"#;
    let envelope: Envelope = serde_json::from_str(line).unwrap();
    assert_eq!(envelope.message, Inbound::RenamePage { path: "a/b".into() });
}

// Stashへ送る項目は、そのidとして届く。
#[test]
fn a_stash_carries_the_item() {
    let line = r#"{"seq":1,"type":"send_to_stash","id":"a"}"#;
    let envelope: Envelope = serde_json::from_str(line).unwrap();
    assert_eq!(envelope.message, Inbound::SendToStash { id: "a".into() });
// 今のページの削除は、中身のない行として届く。
#[test]
fn a_page_deletion_carries_nothing() {
    let line = r#"{"seq":1,"type":"delete_page"}"#;
    let envelope: Envelope = serde_json::from_str(line).unwrap();
    assert_eq!(envelope.message, Inbound::DeletePage);
}

// 画像の挿入は、項目とカーソルと、Emacsが選んだファイルのパスとして届く。
#[test]
fn an_image_insertion_carries_the_item_the_cursor_and_the_path() {
    let line = r#"{"seq":1,"type":"insert_image","id":"a","path":"c:/写真/pic.png","cursor":2}"#;
    let envelope: Envelope = serde_json::from_str(line).unwrap();
    assert_eq!(
        envelope.message,
        Inbound::InsertImage {
            id: "a".into(),
            path: "c:/写真/pic.png".into(),
            cursor: 2,
        }
    );
}

// プロパティの設定はキーと値として、削除はキーとして届く。
#[test]
fn a_property_line_carries_its_key_and_value() {
    let set = r#"{"seq":1,"type":"set_property","key":"status","value":"draft"}"#;
    let envelope: Envelope = serde_json::from_str(set).unwrap();
    assert_eq!(
        envelope.message,
        Inbound::SetProperty {
            key: "status".into(),
            value: "draft".into()
        }
    );
    let delete = r#"{"seq":2,"type":"delete_property","key":"status"}"#;
    let envelope: Envelope = serde_json::from_str(delete).unwrap();
    assert_eq!(
        envelope.message,
        Inbound::DeleteProperty {
            key: "status".into()
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
                "task": null, "collapsed": false, "header": null,
            }],
        })
    );
    assert!(serde_json::to_string(&message).unwrap().contains("本文"));
}

// バレットの種類は、壊れた参照、参照、通常の順で決まる。タスクかどうかは、バレットを変えない。
#[test]
fn the_bullet_shows_what_the_row_is_in_order_of_importance() {
    assert_eq!(Bullet::of(true, true), Bullet::Broken);
    assert_eq!(Bullet::of(false, true), Bullet::Transclusion);
    assert_eq!(Bullet::of(false, false), Bullet::Plain);
}

// タスクの状態は、バレットとは別に伝わる。タスクでない行には、状態がない。
#[test]
fn the_state_of_a_task_goes_beside_the_bullet() {
    assert_eq!(
        TaskMark::of(Some(TaskState::NotStarted)),
        Some(TaskMark::Task)
    );
    assert_eq!(
        TaskMark::of(Some(TaskState::Running)),
        Some(TaskMark::Running)
    );
    assert_eq!(TaskMark::of(Some(TaskState::Done)), Some(TaskMark::Done));
    assert_eq!(TaskMark::of(None), None);
    assert_eq!(
        serde_json::to_value(TaskMark::Running).unwrap(),
        json!("running")
    );
}
