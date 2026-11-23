//! The frontend core calls a journal that is inside the app. Each test has a journal of its
//! own in a folder of its own, and calls it the way the core does: with an `ApiOp`.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use client_api_embedded::{EmbeddedJournal, EventQueue};
use client_core::effects::{ApiOp, ApiOutput, ApiResult, AtlasChange, SseEvent};
use client_core::model::{
    BacklinkEntry, BatchOpInput, Batched, DocPage, Item, ItemPosition, JournalBefore, Page,
    PasteMode, Pasted, PropertyDefinition, SearchResult, ServerConfig, Split, Stashed,
};
use client_core::rest::ClientId;
use journal_core::assembly::{CoreHost, CoreSettings, JournalCore};
use journal_core::clock::SystemClock;
use journal_core::infrastructure::export::reconcile_service::MissingFilesPrompt;
use journal_core::journal::{ChangedBy, FolderWatch};
use serde_json::{json, Value};

/// An app that has no folder to watch and asks nothing about missing files.
struct App;

impl MissingFilesPrompt for App {
    fn notify_new_missing(&self) {}
}

impl FolderWatch for App {
    fn restart(&self) {}
}

struct Fixture {
    journal: EmbeddedJournal,
    core: JournalCore,
    events: Arc<EventQueue>,
    /// What the frontend core got from the event stream so far
    heard: Arc<Mutex<Vec<SseEvent>>>,
    reader: Option<std::thread::JoinHandle<()>>,
    client_id: ClientId,
    _dir: tempfile::TempDir,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.events.close();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

/// Waits until the value is there. The limit only stops a test that would wait for ever.
fn eventually<T>(what: &str, mut value: impl FnMut() -> Option<T>) -> T {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(value) = value() {
            return value;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for {what}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Takes the value out of the answer of the given kind.
macro_rules! expect {
    ($answer:expr, $variant:ident) => {
        match $answer {
            Ok(ApiOutput::$variant(value)) => value,
            other => panic!("expected {}, got {other:?}", stringify!($variant)),
        }
    };
}

impl Fixture {
    fn new() -> Fixture {
        let dir = tempfile::Builder::new()
            .prefix("peta-embedded-")
            .tempdir()
            .unwrap();
        let config_dir = dir.path().join("config");
        std::fs::create_dir_all(&config_dir).unwrap();
        let events = Arc::new(EventQueue::default());
        let core = JournalCore::build(
            &CoreSettings {
                db_path: dir.path().join("journal.db"),
                index_dir: dir.path().join("index"),
                config_dir,
                confined_to: None,
                import_enabled: false,
                import_copy_wait: Duration::from_secs(60),
                export_overwrite_wait: Duration::from_secs(60),
            },
            CoreHost {
                events: events.clone(),
                missing_files: Arc::new(App),
                folder_watch: Arc::new(App),
                clock: Arc::new(SystemClock),
            },
        )
        .unwrap();
        core.start_up();
        let client_id = ClientId::new();
        let journal = EmbeddedJournal::new(
            Arc::clone(&core.journal),
            Arc::clone(&core.confirm),
            &client_id,
        );
        // The frontend core reads the event stream from its start
        let heard = Arc::new(Mutex::new(Vec::new()));
        let reader = {
            let (events, heard) = (events.clone(), heard.clone());
            std::thread::spawn(move || events.read(&mut |event| heard.lock().unwrap().push(event)))
        };
        eventually("the event stream to open", || {
            heard.lock().unwrap().first().cloned()
        });
        Fixture {
            journal,
            core,
            events,
            heard,
            reader: Some(reader),
            client_id,
            _dir: dir,
        }
    }

    /// The messages of the event stream so far, as JSON
    fn messages(&self) -> Vec<Value> {
        self.heard
            .lock()
            .unwrap()
            .iter()
            .filter_map(|event| match event {
                SseEvent::Message(data) => Some(serde_json::from_str(data).unwrap()),
                _ => None,
            })
            .collect()
    }

    fn call(&self, op: ApiOp) -> ApiResult<ApiOutput> {
        self.journal.call(op)
    }

    fn doc(&self, title: &str, dir_path: &str) -> DocPage {
        expect!(
            self.call(ApiOp::GetOrCreateDoc {
                title: title.into(),
                dir_path: dir_path.into(),
            }),
            DocPage
        )
    }

    fn item(&self, page_id: &str, body: &str) -> Item {
        expect!(
            self.call(ApiOp::CreateItem {
                page_id: page_id.into(),
                position: ItemPosition::Last,
                level: 1,
                body: body.into(),
                id: None,
            }),
            Item
        )
    }

    /// The items of the page, each as `level:body`
    fn outline(&self, page_id: &str) -> Vec<String> {
        outline(
            &expect!(
                self.call(ApiOp::GetPageById {
                    page_id: page_id.into(),
                }),
                Page
            )
            .items,
        )
    }
}

fn outline(items: &[Item]) -> Vec<String> {
    items
        .iter()
        .map(|i| format!("{}:{}", i.level, i.body))
        .collect()
}

// ---- pages ----

// ドキュメントのページは、作って、読んで、名前を変えて、最近のページに並び、消せる。
#[test]
fn a_doc_is_made_read_renamed_listed_and_deleted() {
    let f = Fixture::new();

    let made = f.doc("Tea", "");
    assert!(made.created);
    assert_eq!(made.page.name, "Tea");
    let id = made.page.id;

    let read = expect!(
        f.call(ApiOp::GetPageById {
            page_id: id.clone()
        }),
        Page
    );
    assert_eq!(read.page.id, id);

    let renamed = expect!(
        f.call(ApiOp::UpdatePage {
            page_id: id.clone(),
            title: Some("Green tea".into()),
            dir_path: None,
            overwrite: false,
        }),
        PageInfo
    );
    assert_eq!(renamed.name, "Green tea");

    let recent = expect!(
        f.call(ApiOp::GetRecentPages {
            max_results: 10,
            page_type: Some("doc".into()),
        }),
        Pages
    );
    assert!(recent.iter().any(|p| p.id == id), "{recent:?}");

    assert_eq!(
        f.call(ApiOp::DeletePage {
            page_id: id.clone()
        }),
        Ok(ApiOutput::Done)
    );
    assert!(f.call(ApiOp::GetPageById { page_id: id }).is_err());
}

// ほかのページが持つ名前には変えられず、その名前のページがあると答える。
#[test]
fn a_name_another_page_has_is_answered_as_taken() {
    let f = Fixture::new();
    f.doc("Taken", "");
    let other = f.doc("Other", "").page.id;

    let answer = f.call(ApiOp::UpdatePage {
        page_id: other,
        title: Some("Taken".into()),
        dir_path: None,
        overwrite: false,
    });

    assert!(
        matches!(answer, Ok(ApiOutput::PageNameTaken(_))),
        "{answer:?}"
    );
}

// フォルダーのindexページは、フォルダーの名前のページに降格できる。
#[test]
fn the_index_page_of_a_directory_is_demoted_to_the_page_of_its_name() {
    let f = Fixture::new();
    let index = f.doc("index", "fruit/").page.id;

    let demoted = expect!(
        f.call(ApiOp::DemotePage {
            page_id: index,
            overwrite: false,
        }),
        PageInfo
    );

    assert_eq!(
        (demoted.dir_path.as_str(), demoted.name.as_str()),
        ("", "fruit")
    );
}

// フォルダーのindexページの子のページと、ページへのバックリンクを読める。
#[test]
fn the_child_pages_and_the_backlinks_of_a_page_are_read() {
    let f = Fixture::new();
    let index = f.doc("index", "garden/").page.id;
    let child = f.doc("Roses", "garden/").page.id;
    let source = f.doc("Notes", "").page.id;
    f.item(&source, "see [[garden/Roses]]");

    let children = expect!(f.call(ApiOp::GetChildPages { parent_id: index }), Pages);
    assert!(children.iter().any(|p| p.id == child), "{children:?}");

    let backlinks = expect!(f.call(ApiOp::GetBacklinks { page_id: child }), Backlinks);
    assert_eq!(backlinks.len(), 1, "{backlinks:?}");
    assert_eq!(backlinks[0].source_page.id, source);
}

// ---- properties ----

// プロパティは、定義して、ページと項目に付けて、外して、定義を消せる。
#[test]
fn a_property_is_defined_set_on_a_page_and_an_item_removed_and_undefined() {
    let f = Fixture::new();
    let page = f.doc("Mood", "").page.id;
    let item = f.item(&page, "today").id;

    let defined = expect!(
        f.call(ApiOp::AddPropertyDefinition {
            key: "mood".into(),
            value_type: "text".into(),
        }),
        PropertyDefinitions
    );
    assert!(defined.iter().any(|d| d.key == "mood"), "{defined:?}");
    let listed = expect!(f.call(ApiOp::GetPropertyDefinitions), PropertyDefinitions);
    assert_eq!(listed, defined);

    let with = expect!(
        f.call(ApiOp::UpsertPageProperty {
            page_id: page.clone(),
            key: "mood".into(),
            value_type: "text".into(),
            value: "calm".into(),
        }),
        PageInfo
    );
    assert_eq!(with.properties["mood"].as_text(), Some("calm"));
    let without = expect!(
        f.call(ApiOp::DeletePageProperty {
            page_id: page,
            key: "mood".into(),
        }),
        PageInfo
    );
    assert!(!without.properties.contains_key("mood"));

    let with = expect!(
        f.call(ApiOp::UpsertItemProperty {
            item_id: item.clone(),
            key: "mood".into(),
            value_type: "text".into(),
            value: "bright".into(),
        }),
        Item
    );
    assert_eq!(with.properties["mood"].as_text(), Some("bright"));
    let without = expect!(
        f.call(ApiOp::DeleteItemProperty {
            item_id: item,
            key: "mood".into(),
        }),
        Item
    );
    assert!(!without.properties.contains_key("mood"));

    let left = expect!(
        f.call(ApiOp::DeletePropertyDefinition { key: "mood".into() }),
        PropertyDefinitions
    );
    assert!(left.iter().all(|d| d.key != "mood"), "{left:?}");
}

// ---- items ----

// 項目は、書き換え、字下げ、分割、結合ができる。
#[test]
fn an_item_is_updated_shifted_split_and_merged() {
    let f = Fixture::new();
    let page = f.doc("Outline", "").page.id;
    let first = f.item(&page, "first").id;
    let second = f.item(&page, "second").id;

    let updated = expect!(
        f.call(ApiOp::UpdateItem {
            item_id: first.clone(),
            body: Some("one two".into()),
            level: None,
            collapsed: None,
        }),
        Item
    );
    assert_eq!(updated.body, "one two");

    let shifted = expect!(
        f.call(ApiOp::ShiftItemLevels {
            item_ids: vec![second.clone()],
            delta: 1,
        }),
        Items
    );
    assert_eq!(outline(&shifted), ["2:second"]);

    let split = expect!(
        f.call(ApiOp::SplitItem {
            item_id: first.clone(),
            root_id: None,
            body: "one two".into(),
            cursor: 3,
        }),
        Split
    );
    assert_eq!(split.item.body.trim(), "one");
    assert_eq!(split.created.body.trim(), "two");

    let merged = expect!(
        f.call(ApiOp::MergeItems {
            item_id: split.created.id,
            into_id: first,
            body: None,
            into_body: None,
        }),
        Item
    );
    assert!(merged.body.contains("one") && merged.body.contains("two"));
}

// 項目は、タスクにでき、貼り付けとスタッシュができ、まとめて書き換えられる。
#[test]
fn an_item_takes_a_task_state_a_paste_a_stash_and_a_batch() {
    let f = Fixture::new();
    let page = f.doc("Work", "").page.id;
    let item = f.item(&page, "write").id;
    let aside = f.item(&page, "later").id;

    let task = expect!(
        f.call(ApiOp::CycleTask {
            item_id: item.clone()
        }),
        Item
    );
    assert!(task.types.contains(&"task".to_string()), "{task:?}");

    let pasted = expect!(
        f.call(ApiOp::PasteText {
            item_id: item.clone(),
            body: "write".into(),
            cursor: 5,
            text: " a\n\nb".into(),
            mode: PasteMode::Flat,
        }),
        Pasted
    );
    assert_eq!(pasted.items.len(), 2, "{pasted:?}");

    let stashed = expect!(
        f.call(ApiOp::StashItem {
            item_id: aside.clone(),
            body: None,
        }),
        Stashed
    );
    assert!(stashed.created);
    assert_eq!(stashed.heading.body, "# Stash");

    let batched = expect!(
        f.call(ApiOp::ApplyBatch {
            ops: vec![BatchOpInput::Update {
                id: aside.clone(),
                body: Some("much later".into()),
                level: None,
                collapsed: None,
                task: None,
            }],
        }),
        Batched
    );
    assert_eq!(batched.items[0].body, "much later");
}

// 項目は、ほかのページへ動かし、Markdownに書き出し、消せる。
#[test]
fn items_are_moved_exported_and_deleted() {
    let f = Fixture::new();
    let from = f.doc("From", "").page.id;
    let to = f.doc("To", "").page.id;
    let item = f.item(&from, "travels").id;

    let moved = expect!(
        f.call(ApiOp::MoveItems {
            item_ids: vec![item.clone()],
            target_page_id: to.clone(),
            position: ItemPosition::Last,
            level_delta: None,
            body: None,
        }),
        Items
    );
    assert_eq!(moved[0].page_id, to);
    // A new page has one empty item
    assert_eq!(f.outline(&to), ["1:", "1:travels"]);

    let export = expect!(
        f.call(ApiOp::ExportTreeMarkdown {
            item_ids: vec![item.clone()],
        }),
        TreeExport
    );
    assert_eq!(export.tree_markdown.trim(), "- travels");

    expect!(
        f.call(ApiOp::DeleteItems {
            item_ids: vec![item],
        }),
        Items
    );
    assert_eq!(f.outline(&to), ["1:"]);
}

// 項目は、ほかの項目を映すことができ、映されている項目がどれかを尋ねられる。
#[test]
fn an_item_shows_another_item_and_the_shown_items_are_found() {
    let f = Fixture::new();
    let page = f.doc("Mirror", "").page.id;
    let source = f.item(&page, "the source").id;
    let mirror = f.item(&page, "").id;

    let shows = expect!(
        f.call(ApiOp::SetTransclusion {
            item_id: mirror.clone(),
            source_item_id: source.clone(),
        }),
        Item
    );
    assert_eq!(shows.transclude_item_id.as_deref(), Some(source.as_str()));
    let shown = expect!(
        f.call(ApiOp::TranscludedItems {
            item_ids: vec![source.clone()],
        }),
        ItemIds
    );
    assert_eq!(shown, [source.as_str()]);

    let plain = expect!(f.call(ApiOp::RemoveTransclusion { item_id: mirror }), Item);
    assert_eq!(plain.transclude_item_id, None);
    let shown = expect!(
        f.call(ApiOp::TranscludedItems {
            item_ids: vec![source],
        }),
        ItemIds
    );
    assert!(shown.is_empty());
}

// ---- journals ----

// ジャーナルは、日と週の名前で作られ、それより前のいちばん新しいものとして見つかる。
#[test]
fn a_journal_is_made_by_its_name_and_found_as_the_most_recent_before_a_later_one() {
    let f = Fixture::new();

    let day = expect!(
        f.call(ApiOp::GetOrCreateJournal {
            file_name: "2031-05-06".into(),
        }),
        Page
    );
    assert!(day.created);
    let before = expect!(
        f.call(ApiOp::GetMostRecentJournalBefore {
            before: "2031-05-09".into(),
        }),
        JournalBefore
    );
    assert_eq!(before.page.map(|p| p.id), Some(day.page.id));

    let week = expect!(
        f.call(ApiOp::GetOrCreateJournal {
            file_name: "2031-W19".into(),
        }),
        Page
    );
    let before = expect!(
        f.call(ApiOp::GetMostRecentWeeklyJournalBefore {
            before: "2031-W22".into(),
        }),
        JournalBefore
    );
    assert_eq!(before.page.map(|p| p.id), Some(week.page.id));
}

// 項目は、ある日のジャーナルへ動かせる。ジャーナルは、なければ作られる。
#[test]
fn items_are_moved_to_the_journal_of_a_day_which_is_made_for_them() {
    let f = Fixture::new();
    let page = f.doc("Inbox", "").page.id;
    let item = f.item(&page, "for that day").id;

    let journal = expect!(
        f.call(ApiOp::MoveItemsToJournal {
            file_name: "2031-07-01".into(),
            item_ids: vec![item],
            position: ItemPosition::Last,
            body: None,
        }),
        MovedToJournal
    );

    assert!(journal.created);
    assert_eq!(journal.page.name, "2031-07-01");
    assert_eq!(f.outline(&journal.page.id), ["1:for that day"]);
}

// ---- search ----

// 項目の言葉でも、ページの名前でも、ページを検索できる。
#[test]
fn a_page_is_found_by_a_word_of_an_item_and_by_its_name() {
    let f = Fixture::new();
    let page = f.doc("Kettle", "").page.id;
    f.item(&page, "boil the rhubarb");
    // The app commits the index at times it chooses; a test does it now
    f.core.search.flush();

    let by_word = expect!(
        f.call(ApiOp::SearchCombined {
            query: "rhubarb".into(),
            max_results: 10,
        }),
        SearchResults
    );
    assert!(by_word.iter().any(|r| r.page_id == page), "{by_word:?}");

    let by_name = expect!(
        f.call(ApiOp::SearchPageNames {
            query: "Kettle".into(),
            max_results: 10,
        }),
        SearchResults
    );
    assert!(by_name.iter().any(|r| r.page_id == page), "{by_name:?}");
}

// ---- atlas ----

/// A directory with the pages `a` and `b`, and its atlas page.
fn atlas(f: &Fixture) -> (String, String) {
    let a = f.doc("a", "map/").page.id;
    f.doc("b", "map/");
    (f.doc("index_atlas", "map/").page.id, a)
}

fn change(f: &Fixture, atlas_id: &str, change: AtlasChange) -> Vec<Item> {
    expect!(
        f.call(ApiOp::ChangeAtlas {
            atlas_id: atlas_id.into(),
            change,
        }),
        Items
    )
}

// アトラスのリンクカードは、足して、宛先を変えて、グループに入れて出して、消せる。
#[test]
fn a_link_card_of_an_atlas_is_added_relinked_grouped_ungrouped_and_removed() {
    let f = Fixture::new();
    let (atlas_id, a) = atlas(&f);

    let items = change(
        &f,
        &atlas_id,
        AtlasChange::AddLink {
            target: "[[elsewhere/l]]".into(),
            bounds: "1,2,0,40".into(),
        },
    );
    let link = items
        .iter()
        .find(|i| i.body == "[[elsewhere/l]]")
        .unwrap()
        .id
        .clone();

    let items = change(
        &f,
        &atlas_id,
        AtlasChange::SetLink {
            item_id: link.clone(),
            target: "far/zz".into(),
        },
    );
    assert!(items.iter().any(|i| i.body == "[[far/zz]]"), "{items:?}");

    let items = change(
        &f,
        &atlas_id,
        AtlasChange::Group {
            cards: vec![link.clone()],
            target: a,
        },
    );
    assert_eq!(outline(&items[..3]), ["1:", "2:[[map/a]]", "2:[[far/zz]]"]);
    let group = items[0].id.clone();

    let items = change(
        &f,
        &atlas_id,
        AtlasChange::SetGroupTitle {
            item_id: group,
            side: "left".into(),
            title: "Tools".into(),
        },
    );
    assert_eq!(items[0].body, "Tools");

    let items = change(
        &f,
        &atlas_id,
        AtlasChange::Ungroup {
            cards: vec![link.clone()],
        },
    );
    assert!(items.iter().all(|i| i.level == 1), "{items:?}");

    let items = change(&f, &atlas_id, AtlasChange::Remove { item_id: link });
    assert!(items.iter().all(|i| i.body != "[[far/zz]]"), "{items:?}");
}

// アトラスの吹き出しは、足して、書き換えられる。
#[test]
fn a_callout_of_an_atlas_is_added_and_rewritten() {
    let f = Fixture::new();
    let (atlas_id, _) = atlas(&f);

    let items = change(
        &f,
        &atlas_id,
        AtlasChange::AddCallout {
            bounds: "10,20,0,60".into(),
            side: "left".into(),
            text: "Hello".into(),
        },
    );
    let callout = items.last().unwrap().clone();
    assert_eq!(callout.body, "Hello");

    let items = change(
        &f,
        &atlas_id,
        AtlasChange::SetCallout {
            item_id: callout.id,
            side: "bottom".into(),
            text: "Bye".into(),
        },
    );
    assert_eq!(items.last().unwrap().body, "Bye");
}

// ---- settings ----

// 設定は、読んで、変えられる。
#[test]
fn the_config_is_read_and_updated() {
    let f = Fixture::new();

    let config = expect!(f.call(ApiOp::GetConfig), Config);
    assert_eq!(config.home_link, "");

    let updated = expect!(
        f.call(ApiOp::UpdateConfig {
            markdown_dir: None,
            week_numbering: None,
            logical_outdent: Some(true),
            tags_dir: None,
            home_link: Some("today".into()),
        }),
        Config
    );
    assert_eq!(updated.home_link, "today");
    assert!(updated.logical_outdent);
    assert_eq!(expect!(f.call(ApiOp::GetConfig), Config), updated);
}

// ---- the app around the journal ----

// 変更は、このフロントエンドのIDと一緒に、アプリに知らされる。
#[test]
fn the_app_is_told_of_a_change_with_the_id_of_this_frontend() {
    let f = Fixture::new();
    let page = f.doc("Told", "").page.id;

    f.item(&page, "a change");

    let told = eventually("the message of the change", || f.messages().pop());
    assert_eq!(
        told,
        json!({
            "type": "pagesUpdated",
            "pageId": page,
            "source": "app",
            "clientId": f.client_id.as_str(),
        })
    );
}

// イベントの流れは、開いてから始まり、閉じるとエラーで終わる。
#[test]
fn the_event_stream_opens_and_ends_with_an_error_when_it_is_closed() {
    let mut f = Fixture::new();

    f.events.close();
    f.reader.take().unwrap().join().unwrap();

    assert_eq!(*f.heard.lock().unwrap(), [SseEvent::Open, SseEvent::Error]);
}

// 誰も読んでいないときのメッセージは、捨てられる。
#[test]
fn a_message_is_dropped_while_no_one_reads() {
    use journal_core::events::EventSink;
    let events = EventQueue::default();

    assert!(!events.send("lost".into()));

    events.close();
    let mut heard = Vec::new();
    events.read(&mut |event| heard.push(event));
    assert_eq!(heard, [SseEvent::Open, SseEvent::Error]);
}

// ジャーナルが断った操作は、エラーとして答えられる。
#[test]
fn an_operation_the_journal_refuses_is_answered_as_an_error() {
    let f = Fixture::new();

    assert!(f
        .call(ApiOp::CreateItem {
            page_id: "no such page".into(),
            position: ItemPosition::Last,
            level: 1,
            body: "lost".into(),
            id: None,
        })
        .is_err());
    // No extension asked anything
    assert!(f
        .call(ApiOp::AnswerConfirm {
            request_id: "1".into(),
            ok: true,
        })
        .is_err());
}

// 拡張機能の質問には、フロントエンドの答えが届く。
#[test]
fn the_question_of_an_extension_gets_the_answer_of_the_frontend() {
    let f = Fixture::new();
    let confirm = Arc::clone(&f.core.confirm);
    let asking = std::thread::spawn(move || confirm.confirm("Go on?"));
    let question = eventually("the question", || f.messages().pop());
    assert_eq!(question["type"], json!("confirm"));
    assert_eq!(question["message"], json!("Go on?"));
    let request_id = question["requestId"].as_str().unwrap().to_string();

    let answered = f.call(ApiOp::AnswerConfirm {
        request_id,
        ok: true,
    });

    assert_eq!(answered, Ok(ApiOutput::Done));
    assert!(asking.join().unwrap());
}

// サーバーが自分のマシンで行う操作は、アプリの中のジャーナルにはない。
#[test]
fn what_the_server_does_with_its_machine_is_not_done_inside_the_app() {
    let f = Fixture::new();
    let page = f.doc("Here", "").page.id;

    for op in [
        ApiOp::BrowseFolder,
        ApiOp::LaunchUrl {
            url: "obsidian://open".into(),
        },
        ApiOp::OpenExportFolder { page_id: page },
        ApiOp::UploadAsset { file: 1 },
    ] {
        assert!(f.call(op.clone()).is_err(), "{op:?}");
    }
}

// ---- assets ----

// シェルが渡したパスのファイルは、新しい名前で本文フォルダの`assets/`に写される。答えには、保存した名前と元の名前が入る（ADR 071）。
#[test]
fn a_file_the_shell_names_by_its_path_is_copied_into_the_assets() {
    let f = Fixture::new();
    let markdown = f._dir.path().join("markdown");
    f.core.config.set_markdown_dir(&markdown.to_string_lossy());
    let pasted = f._dir.path().join("pasted.png");
    std::fs::write(&pasted, b"drawing").unwrap();

    let saved = expect!(f.journal.upload(&pasted), Uploaded);

    assert_eq!(saved.original_name, "pasted.png");
    assert!(
        saved.file_name.ends_with(".png") && saved.file_name != "pasted.png",
        "{saved:?}"
    );
    let copy = markdown.join("assets").join(&saved.file_name);
    assert_eq!(std::fs::read(copy).unwrap(), b"drawing");
}

// ---- the shapes of the answers ----
//
// The journal writes its answers with its own types and the frontend core reads them with
// others. The core's types take a missing field as empty, so a field that one side renames
// would go unseen. These tests read each answer into the core's type, write it back, and
// name every field that is not the same on both sides.

/// The differences between what the journal wrote and what the core holds of it, by field.
fn differences(journal: &Value, core: &Value, path: &str, found: &mut Vec<String>) {
    match (journal, core) {
        (Value::Object(journal), Value::Object(core)) => {
            for (key, value) in journal {
                let at = format!("{path}.{key}");
                match core.get(key) {
                    Some(held) => differences(value, held, &at, found),
                    None => found.push(format!("{at}: the core does not read it")),
                }
            }
            // A field that may be absent: the journal leaves it out and the core has none
            let made_up = core
                .iter()
                .filter(|(key, held)| !journal.contains_key(*key) && !held.is_null());
            for (key, _) in made_up {
                found.push(format!("{path}.{key}: the journal does not write it"));
            }
        }
        (Value::Array(journal), Value::Array(core)) => {
            if journal.len() != core.len() {
                found.push(format!("{path}: the core holds another number of them"));
            }
            for (value, held) in journal.iter().zip(core) {
                differences(value, held, &format!("{path}[]"), found);
            }
        }
        _ if journal != core => found.push(format!("{path}: the core holds another value")),
        _ => {}
    }
}

/// What is lost or made up when the core reads this answer of the journal as a `T`.
fn drift<T: serde::de::DeserializeOwned + serde::Serialize>(answer: &Value) -> Vec<String> {
    let read: T = serde_json::from_value(answer.clone()).expect("the core reads the answer");
    let mut found = Vec::new();
    differences(answer, &serde_json::to_value(read).unwrap(), "", &mut found);
    found.sort();
    found.dedup();
    found
}

/// The JSON of an answer of the journal.
fn written(answer: impl serde::Serialize) -> Value {
    serde_json::to_value(answer).unwrap()
}

/// The fields of an item that the frontend core has no use for. It makes the `url` itself
/// with `page-urls` (ADR 061).
fn unread_of_items(path: &str) -> Vec<String> {
    vec![
        format!("{path}.ast: the core does not read it"),
        format!("{path}.parentItemId: the core does not read it"),
        format!("{path}.url: the core does not read it"),
    ]
}

/// The field of a page that the frontend core has no use for: it makes the `url` itself with
/// `page-urls` (ADR 061).
fn unread_of_pages(path: &str) -> Vec<String> {
    vec![format!("{path}.url: the core does not read it")]
}

impl Fixture {
    /// The one who changes the journal in these tests: this frontend.
    fn by(&self) -> ChangedBy {
        ChangedBy::Client(self.client_id.as_str().parse().unwrap())
    }

    /// A page with a heading, a tagged task under it and a property of its own.
    fn full_page(&self) -> (String, Vec<Item>) {
        let page = self.doc("Shapes", "notes/").page.id;
        let heading = self.item(&page, "# Head");
        let task = self.item(&page, "TODO task #topic");
        expect!(
            self.call(ApiOp::UpsertPageProperty {
                page_id: page.clone(),
                key: "tags".into(),
                value_type: "tags".into(),
                value: "a, b".into(),
            }),
            PageInfo
        );
        (page, vec![heading, task])
    }
}

// ページと項目は、項目の`ast`と`parentItemId`を除いて、ジャーナルが書いた欄のまま読まれる。
#[test]
fn a_page_and_its_items_are_read_with_every_field_but_the_ast_and_the_parent() {
    let f = Fixture::new();
    f.full_page();

    let answer = written(
        f.core
            .journal
            .get_or_create_doc(Some("Shapes"), Some("notes/"))
            .unwrap(),
    );

    assert_eq!(
        drift::<DocPage>(&answer),
        [unread_of_items(".items[]"), unread_of_pages(".page")].concat()
    );
}

// ページの一覧は、ジャーナルが書いた欄のまま読まれる。
#[test]
fn a_list_of_pages_is_read_with_every_field() {
    let f = Fixture::new();
    f.full_page();

    let answer = written(f.core.journal.recent_pages(None, None).unwrap());

    assert!(!answer["pages"].as_array().unwrap().is_empty());
    assert_eq!(drift::<Vec<Page>>(&answer["pages"]), unread_of_pages("[]"));
}

// バックリンクは、項目の`ast`と`parentItemId`を除いて、ジャーナルが書いた欄のまま読まれる。
#[test]
fn backlinks_are_read_with_every_field_but_the_ast_and_the_parent() {
    let f = Fixture::new();
    let target = f.doc("Target", "").page.id;
    let source = f.doc("Source", "").page.id;
    f.item(&source, "see [[Target]]");

    let answer = written(f.core.journal.backlinks(&target).unwrap());

    assert_eq!(
        drift::<Vec<BacklinkEntry>>(&answer["entries"]),
        [
            unread_of_items("[].sourceItem"),
            unread_of_pages("[].sourcePage")
        ]
        .concat()
    );
}

// 検索の結果は、ジャーナルが書いた欄のまま読まれる。
#[test]
fn search_results_are_read_with_every_field() {
    let f = Fixture::new();
    f.full_page();

    let answer = eventually("the search to find the task", || {
        let answer = written(f.core.journal.search_combined("task", None).unwrap());
        (!answer["results"].as_array().unwrap().is_empty()).then_some(answer)
    });

    assert_eq!(
        drift::<Vec<SearchResult>>(&answer["results"]),
        Vec::<String>::new()
    );
}

// プロパティの定義と設定は、ジャーナルが書いた欄のまま読まれる。
#[test]
fn the_property_definitions_and_the_config_are_read_with_every_field() {
    let f = Fixture::new();

    let definitions = written(f.core.journal.property_definitions().unwrap());
    let config = written(f.core.journal.config());

    assert!(!definitions["definitions"].as_array().unwrap().is_empty());
    assert_eq!(
        drift::<Vec<PropertyDefinition>>(&definitions["definitions"]),
        Vec::<String>::new()
    );
    // The values to render the links of a body are for the clients that cannot call Rust. The
    // core has them from `page-urls` (ADR 061)
    assert_eq!(
        drift::<ServerConfig>(&config),
        [
            ".hashtagHref: the core does not read it",
            ".pagesBase: the core does not read it",
            ".wikilinkFragmentPattern: the core does not read it",
            ".wikilinkHref: the core does not read it",
        ]
    );
}

// 前のジャーナルは、項目の`ast`と`parentItemId`を除いて、ジャーナルが書いた欄のまま読まれる。
#[test]
fn the_journal_before_a_day_is_read_with_every_field_but_the_ast_and_the_parent() {
    let f = Fixture::new();
    let journal = expect!(
        f.call(ApiOp::GetOrCreateJournal {
            file_name: "2026-01-05".into(),
        }),
        Page
    );
    f.item(&journal.page.id, "a day");

    let answer = written(
        f.core
            .journal
            .most_recent_journal_before("2026-01-06")
            .unwrap(),
    );

    assert_eq!(answer["found"], json!(true));
    assert_eq!(
        drift::<JournalBefore>(&answer),
        [unread_of_items(".items[]"), unread_of_pages(".page")].concat()
    );
}

// 貼り付け、分割、一括、スタッシュの答えは、項目の`ast`と`parentItemId`を除いて、ジャーナルが書いた欄のまま読まれる。
#[test]
fn what_a_change_of_items_came_to_is_read_with_every_field_but_the_ast_and_the_parent() {
    let f = Fixture::new();
    let (_, items) = f.full_page();
    let (journal, by) = (&f.core.journal, f.by());
    let id = &items[1].id;

    let pasted = written(
        journal
            .paste(
                id,
                serde_json::from_value(
                    json!({ "body": "one", "cursor": 3, "text": "a\nb", "mode": "flat" }),
                )
                .unwrap(),
                &by,
            )
            .unwrap(),
    );
    let split = written(
        journal
            .split(
                id,
                serde_json::from_value(json!({ "body": "onea", "cursor": 2 })).unwrap(),
                &by,
            )
            .unwrap(),
    );
    let batched = written(
        journal
            .apply_batch(
                serde_json::from_value(
                    json!({ "ops": [{ "op": "update", "id": id, "body": "new" }] }),
                )
                .unwrap(),
                &by,
            )
            .unwrap(),
    );
    let stashed = written(
        journal
            .stash(id, serde_json::from_value(json!({})).unwrap(), &by)
            .unwrap(),
    );

    assert_eq!(drift::<Pasted>(&pasted), unread_of_items(".items[]"));
    assert_eq!(
        drift::<Split>(&split),
        [unread_of_items(".created"), unread_of_items(".item")].concat()
    );
    assert_eq!(drift::<Batched>(&batched), unread_of_items(".items[]"));
    assert_eq!(
        drift::<Stashed>(&stashed),
        [unread_of_items(".heading"), unread_of_items(".items[]")].concat()
    );
}
