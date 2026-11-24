use crate::emacs_text::EmacsText;
use crate::protocol::{
    read_line, write_line, Bullet, Inbound, Outbound, RowView, SearchEntry, SelectionKey,
    SidebarView, TaskMark,
};
use client_core::app::SelectionKey as CoreSelectionKey;
use client_core::changes::ListChange;
use client_core::view::{SearchEntry as CoreSearchEntry, SearchKind as CoreSearchKind, TaskState};
use client_core::BodyModel;
use serde_json::{json, Value};

/// The line written for `message`, read back as JSON.
fn written(message: &Outbound) -> Value {
    serde_json::from_str(&write_line(message)).unwrap()
}

fn row(bullet: Bullet) -> RowView {
    RowView {
        id: "a".into(),
        level: 2,
        body: "本文".into(),
        text: EmacsText::of_markdown("本文", "tags/"),
        bullet,
        task: None,
        collapsed: false,
        header: None,
        selected: false,
    }
}

// Emacsからの1行は、通し番号と操作に分けて読まれる。
#[test]
fn a_line_from_emacs_has_a_sequence_number_and_a_message() {
    let line = r#"{"seq":7,"type":"body_changed","id":"a","body":"日本語"}"#;
    let envelope = read_line(line).unwrap();
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
    let envelope = read_line(line).unwrap();
    assert_eq!(envelope.message, Inbound::Answer { req: 3, yes: false });
}

// 名前の変更は、フォルダを含む新しいパスとして届く。
#[test]
fn a_rename_carries_the_new_path() {
    let line = r#"{"seq":1,"type":"rename_page","path":"a/b"}"#;
    let envelope = read_line(line).unwrap();
    assert_eq!(envelope.message, Inbound::RenamePage { path: "a/b".into() });
}

// 選択の始まりは項目として、選択中のキーはその名前として届く。
#[test]
fn a_selection_starts_on_an_item_and_takes_keys_by_name() {
    let line = r#"{"seq":1,"type":"select","id":"a"}"#;
    let envelope = read_line(line).unwrap();
    assert_eq!(envelope.message, Inbound::Select { id: "a".into() });
    let line = r#"{"seq":2,"type":"selection_key","key":"MoveDown"}"#;
    let envelope = read_line(line).unwrap();
    assert_eq!(
        envelope.message,
        Inbound::SelectionKey {
            key: SelectionKey::MoveDown
        }
    );
}

// トランスクルージョンの貼り付けは、項目と、Emacsのクリップボードの文字として届く。解除は、項目として届く。
#[test]
fn a_transclusion_carries_the_item_and_the_pasted_text() {
    let line = r#"{"seq":1,"type":"paste_as_transclusion","id":"b","text":"abcdefghij"}"#;
    let envelope = read_line(line).unwrap();
    assert_eq!(
        envelope.message,
        Inbound::PasteAsTransclusion {
            id: "b".into(),
            text: "abcdefghij".into()
        }
    );
    let line = r#"{"seq":2,"type":"remove_transclusion","id":"b"}"#;
    let envelope = read_line(line).unwrap();
    assert_eq!(
        envelope.message,
        Inbound::RemoveTransclusion { id: "b".into() }
    );
}

// フォルダ名の変更は、新しいフォルダのパスとして届く。降格は、中身のない行として届く。
#[test]
fn a_folder_rename_carries_the_path_and_a_demotion_nothing() {
    let line = r#"{"seq":1,"type":"rename_folder","path":"a/c"}"#;
    let envelope = read_line(line).unwrap();
    assert_eq!(
        envelope.message,
        Inbound::RenameFolder { path: "a/c".into() }
    );
    let line = r#"{"seq":2,"type":"demote_index"}"#;
    let envelope = read_line(line).unwrap();
    assert_eq!(envelope.message, Inbound::DemoteIndex);
}

// 日付が変わったかもしれないという知らせは、中身のない行として届く。
#[test]
fn a_day_that_may_have_changed_arrives_with_nothing_more() {
    let line = r#"{"seq":3,"type":"day_may_have_changed"}"#;
    let envelope = read_line(line).unwrap();
    assert_eq!(envelope.message, Inbound::DayMayHaveChanged);
}

// Stashへ送る項目は、そのidとして届く。
#[test]
fn a_stash_carries_the_item() {
    let line = r#"{"seq":1,"type":"send_to_stash","id":"a"}"#;
    let envelope = read_line(line).unwrap();
    assert_eq!(envelope.message, Inbound::SendToStash { id: "a".into() });
}

// 今のページの削除は、中身のない行として届く。
#[test]
fn a_page_deletion_carries_nothing() {
    let line = r#"{"seq":1,"type":"delete_page"}"#;
    let envelope = read_line(line).unwrap();
    assert_eq!(envelope.message, Inbound::DeletePage);
}

// 画像の挿入は、項目とカーソルと、Emacsが選んだファイルのパスとして届く。
#[test]
fn an_image_insertion_carries_the_item_the_cursor_and_the_path() {
    let line = r#"{"seq":1,"type":"insert_image","id":"a","path":"c:/写真/pic.png","cursor":2}"#;
    let envelope = read_line(line).unwrap();
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
    let envelope = read_line(set).unwrap();
    assert_eq!(
        envelope.message,
        Inbound::SetProperty {
            key: "status".into(),
            value: "draft".into()
        }
    );
    let delete = r#"{"seq":2,"type":"delete_property","key":"status"}"#;
    let envelope = read_line(delete).unwrap();
    assert_eq!(
        envelope.message,
        Inbound::DeleteProperty {
            key: "status".into()
        }
    );
}

// 行の差分は、Emacsが読むJSONとして、変化の種類の名前の下に書き出される。日本語はエスケープされない。
#[test]
fn the_changes_of_the_rows_are_written_as_json_for_emacs() {
    let message = Outbound::Rows {
        ack: 4,
        page_id: Some("p".into()),
        changes: vec![
            ListChange::Update {
                at: 0,
                item: RowView {
                    text: EmacsText::of_markdown("**本文**", "tags/"),
                    ..row(Bullet::Plain)
                },
            },
            ListChange::Move { from: 2, to: 1 },
        ],
    };
    assert_eq!(
        written(&message),
        json!({
            "t": "rows", "ack": 4, "page_id": "p",
            "changes": [
                {"Update": {"at": 0, "item": {
                    "id": "a", "level": 2, "body": "本文", "bullet": "plain",
                    "task": null, "collapsed": false, "header": null, "selected": false,
                    "text": {"text": "本文\n", "spans": [{
                        "from": 0, "to": 2, "looks": ["bold"], "link": null, "image": null,
                        "letters": true,
                    }]},
                }}},
                {"Move": {"from": 2, "to": 1}},
            ],
        })
    );
    assert!(write_line(&message).contains("本文"));
}

// バレットの種類は、壊れた参照、参照、通常の順で決まる。タスクかどうかは、バレットを変えない。
#[test]
fn the_bullet_shows_what_the_row_is_in_order_of_importance() {
    assert_eq!(Bullet::of(true, true, true), Bullet::Broken);
    assert_eq!(Bullet::of(false, true, true), Bullet::Transclusion);
    assert_eq!(Bullet::of(false, false, true), Bullet::Plain);
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
    let mut task = row(Bullet::Plain);
    task.task = Some(TaskMark::Running);
    let message = Outbound::Rows {
        ack: 1,
        page_id: None,
        changes: vec![ListChange::Reset { items: vec![task] }],
    };
    assert_eq!(
        written(&message)["changes"][0]["Reset"]["items"][0]["task"],
        json!("running")
    );
}

fn sidebar_page(id: &str, label: &str, path: &str, is_dir: bool) -> client_core::view::SidebarPage {
    client_core::view::SidebarPage {
        page: client_core::model::Page {
            id: id.into(),
            ..Default::default()
        },
        label: label.into(),
        published: false,
        is_dir,
        dir_label: None,
        path: path.into(),
    }
}

fn core_sidebar(header: &str, parent: Option<&str>) -> client_core::view::SidebarView {
    client_core::view::SidebarView {
        index_header: header.into(),
        index_path: client_core::screens::sidebar::index_path(header),
        index_parent_path: parent.map(Into::into),
        index_pages: vec![
            sidebar_page("s", "sub/", "notes/sub/index", true),
            sidebar_page("i", "Idea", "notes/Idea", false),
        ],
        current_page_id: Some("i".into()),
        ..Default::default()
    }
}

// フォルダの一覧には、見出しのindexページへのパスと、親のフォルダへのパスが付く。今のページには印が付く。
#[test]
fn the_index_list_carries_the_paths_of_its_header_and_its_parent() {
    let sidebar = SidebarView::of(&core_sidebar("notes/", Some("index")));
    let index = sidebar.index.unwrap();
    assert_eq!(index.header, "notes/");
    assert_eq!(index.path, "notes/index");
    assert_eq!(index.parent.as_deref(), Some("index"));
    let lines: Vec<(&str, &str, bool, bool)> = index
        .pages
        .iter()
        .map(|line| {
            (
                line.label.as_str(),
                line.path.as_str(),
                line.folder,
                line.current,
            )
        })
        .collect();
    assert_eq!(
        lines,
        [
            ("sub/", "notes/sub/index", true, false),
            ("Idea", "notes/Idea", false, true)
        ]
    );
}

// ページも親もない一覧は、出さない。
#[test]
fn an_empty_index_list_is_left_out() {
    let mut core = core_sidebar("", None);
    core.index_pages.clear();
    assert!(SidebarView::of(&core).index.is_none());
}

// サイドバーは、種類の名前の付いた1行のJSONになる。
#[test]
fn the_sidebar_is_a_line_with_its_type() {
    let message = Outbound::Sidebar(SidebarView::of(&core_sidebar("(root)/", None)));
    let line = written(&message);
    assert_eq!(line["t"], json!("sidebar"));
    assert_eq!(line["index"]["path"], json!("index"));
}

// Emacsの選択のキーは、同じ名前のコアのキーになる。
#[test]
fn a_selection_key_of_emacs_is_the_key_of_the_core_of_the_same_name() {
    let keys = [
        SelectionKey::Escape,
        SelectionKey::Delete,
        SelectionKey::MoveUp,
        SelectionKey::MoveDown,
        SelectionKey::Indent,
        SelectionKey::Outdent,
        SelectionKey::ExtendUp,
        SelectionKey::ExtendDown,
        SelectionKey::Copy,
        SelectionKey::Cut,
        SelectionKey::Undo,
        SelectionKey::Redo,
    ];
    for key in keys {
        let core: CoreSelectionKey = key.into();
        assert_eq!(format!("{core:?}"), format!("{key:?}"));
    }
}

// 検索の結果の種類は、コアの種類の名前で書き出される。
#[test]
fn the_kind_of_a_search_result_is_written_by_the_name_of_the_core() {
    let kinds = [
        (CoreSearchKind::Today, "today"),
        (CoreSearchKind::Page, "page"),
        (CoreSearchKind::Create, "create"),
        (CoreSearchKind::Item, "item"),
    ];
    for (kind, name) in kinds {
        let entry = CoreSearchEntry {
            position: 0,
            kind,
            title: "t".into(),
            label: "l".into(),
            snippet: "s".into(),
        };
        let message = Outbound::Search {
            results: vec![SearchEntry::of(&entry)],
            searching: false,
        };
        assert_eq!(written(&message)["results"][0]["kind"], json!(name));
    }
}
