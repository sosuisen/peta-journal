//! The frontend core calls a journal that is inside the app. Each test has a journal of its
//! own in a folder of its own, and calls it the way the core does: with an `ApiOp`.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use hmtid::Hmtid;
use journal_core::application::atlas_service::AtlasEvents;
use journal_core::application::confirm_service::ConfirmEvents;
use journal_core::application::extension_service::ExtensionEvents;
use journal_core::assembly::{CoreHost, CoreSettings, JournalCore};
use journal_core::infrastructure::export::import_service::ImportEvents;
use journal_core::infrastructure::export::reconcile_service::MissingFilesPrompt;
use journal_core::journal::{ChangedBy, FolderWatch, JournalEvents};
use peta_core::effects::{ApiOp, ApiOutput, ApiResult, AtlasChange};
use peta_core::model::{BatchOp, Item, ItemPosition, PageWithItems, PasteMode};
use peta_core::rest::ClientId;
use peta_embedded_journal::EmbeddedJournal;

/// The app around the journal. It keeps the changes it was told of, each as the page and the
/// client that made the change.
#[derive(Default)]
struct App {
    told: Mutex<Vec<(String, String)>>,
    /// The questions of the extensions, each by the ID to answer it with
    asked: Mutex<Vec<String>>,
}

impl JournalEvents for App {
    fn page_changed(&self, page_id: &Hmtid, by: &ChangedBy) {
        let client = match by {
            ChangedBy::Client(client) => client.as_str().to_string(),
            ChangedBy::Mcp => "mcp".to_string(),
        };
        self.told
            .lock()
            .unwrap()
            .push((page_id.to_string(), client));
    }
}

impl AtlasEvents for App {
    fn atlas_changed(&self, _atlas_id: &Hmtid) {}
}

impl ImportEvents for App {
    fn page_changed(&self, _page_id: &Hmtid) {}
}

impl ExtensionEvents for App {
    fn page_changed(&self, _page_id: &Hmtid) {}
}

impl ConfirmEvents for App {
    /// The app shows the question. The answer comes later, as an `ApiOp`
    fn ask_user(&self, request_id: &str, _message: &str) -> bool {
        self.asked.lock().unwrap().push(request_id.to_string());
        true
    }
}

impl MissingFilesPrompt for App {
    fn notify_new_missing(&self) {}
}

impl FolderWatch for App {
    fn restart(&self) {}
}

struct Fixture {
    journal: EmbeddedJournal,
    core: JournalCore,
    app: Arc<App>,
    client_id: ClientId,
    _dir: tempfile::TempDir,
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
        let app = Arc::new(App::default());
        let core = JournalCore::build(
            &CoreSettings {
                db_path: dir.path().join("journal.db"),
                index_dir: dir.path().join("index"),
                config_dir,
                import_enabled: false,
                import_copy_wait: Duration::from_secs(60),
                export_overwrite_wait: Duration::from_secs(60),
            },
            CoreHost {
                journal_events: app.clone(),
                atlas_events: app.clone(),
                import_events: app.clone(),
                extension_events: app.clone(),
                confirm_events: app.clone(),
                missing_files: app.clone(),
                folder_watch: app.clone(),
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
        Fixture {
            journal,
            core,
            app,
            client_id,
            _dir: dir,
        }
    }

    fn call(&self, op: ApiOp) -> ApiResult<ApiOutput> {
        self.journal.call(op)
    }

    fn doc(&self, title: &str, dir_path: &str) -> PageWithItems {
        expect!(
            self.call(ApiOp::GetOrCreateDoc {
                title: title.into(),
                dir_path: dir_path.into(),
            }),
            Page
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
            ops: vec![BatchOp::Update {
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
        Page
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

    assert_eq!(
        *f.app.told.lock().unwrap(),
        [(page, f.client_id.as_str().to_string())]
    );
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
    let request_id = loop {
        if let Some(id) = f.app.asked.lock().unwrap().first() {
            break id.clone();
        }
        std::thread::sleep(Duration::from_millis(10));
    };

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
