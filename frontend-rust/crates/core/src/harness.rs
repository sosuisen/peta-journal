//! A test runtime for the core, like `outline-test-support.ts`: drives the commands an
//! event produces, answers the API like the server would, keeps timers to be fired by the
//! test, and records what the shell was asked to do.

use crate::app::{Event, Model, PetaApp};
use crate::effects::*;
use crate::model::*;
use crate::testing::plain_item;
use crux_core::{App, Command, Request};
use std::collections::{BTreeMap, HashMap, VecDeque};

/// The hashtags of a body as the server stores them in the tag property.
fn tags_in(body: &str) -> BTreeMap<String, serde_json::Value> {
    let tags: Vec<serde_json::Value> = body
        .split(|c: char| c.is_whitespace())
        .filter_map(|w| w.strip_prefix('#'))
        .filter(|t| {
            !t.is_empty()
                && t.chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        })
        .map(|t| serde_json::Value::String(t.to_string()))
        .collect();
    let mut props = BTreeMap::new();
    if !tags.is_empty() {
        props.insert("tag".to_string(), serde_json::Value::Array(tags));
    }
    props
}

/// Records API calls and answers like the server would: an update echoes the fields it got,
/// a create returns a new item (with the client id when one was passed), a move echoes the ids.
#[derive(Default)]
pub struct FakeApi {
    pub calls: Vec<ApiOp>,
    created: u32,
    /// Answers queued per operation name; used before the default answer.
    pub answers: HashMap<&'static str, VecDeque<ApiResult<ApiOutput>>>,
}

pub fn op_name(op: &ApiOp) -> &'static str {
    match op {
        ApiOp::GetPageById { .. } => "GetPageById",
        ApiOp::GetMostRecentJournalBefore { .. } => "GetMostRecentJournalBefore",
        ApiOp::GetMostRecentWeeklyJournalBefore { .. } => "GetMostRecentWeeklyJournalBefore",
        ApiOp::GetOrCreateJournal { .. } => "GetOrCreateJournal",
        ApiOp::GetOrCreateDoc { .. } => "GetOrCreateDoc",
        ApiOp::CreateItem { .. } => "CreateItem",
        ApiOp::UpdateItem { .. } => "UpdateItem",
        ApiOp::ShiftItemLevels { .. } => "ShiftItemLevels",
        ApiOp::MoveItems { .. } => "MoveItems",
        ApiOp::DeletePage { .. } => "DeletePage",
        ApiOp::DeleteItem { .. } => "DeleteItem",
        ApiOp::CycleTask { .. } => "CycleTask",
        ApiOp::UpsertItemProperty { .. } => "UpsertItemProperty",
        ApiOp::SearchCombined { .. } => "SearchCombined",
        ApiOp::SearchPageNames { .. } => "SearchPageNames",
        ApiOp::GetRecentPages { .. } => "GetRecentPages",
        ApiOp::UpdatePage { .. } => "UpdatePage",
        ApiOp::UpsertPageProperty { .. } => "UpsertPageProperty",
        ApiOp::DeletePageProperty { .. } => "DeletePageProperty",
        ApiOp::GetConfig => "GetConfig",
        ApiOp::GetPropertyDefinitions => "GetPropertyDefinitions",
        ApiOp::AddPropertyDefinition { .. } => "AddPropertyDefinition",
        ApiOp::UpdateConfig { .. } => "UpdateConfig",
        ApiOp::BrowseFolder => "BrowseFolder",
        ApiOp::ExportTreeMarkdown { .. } => "ExportTreeMarkdown",
        ApiOp::ImportTreeMarkdown { .. } => "ImportTreeMarkdown",
        ApiOp::ParseFlatMarkdown { .. } => "ParseFlatMarkdown",
        ApiOp::SetTransclusion { .. } => "SetTransclusion",
        ApiOp::RemoveTransclusion { .. } => "RemoveTransclusion",
        ApiOp::GetChildPages { .. } => "GetChildPages",
        ApiOp::UploadAsset { .. } => "UploadAsset",
        ApiOp::GetFunctions => "GetFunctions",
        ApiOp::LaunchUrl { .. } => "LaunchUrl",
    }
}

impl FakeApi {
    /// Queues an answer for the next call of an operation.
    pub fn answer_next(&mut self, name: &'static str, answer: ApiResult<ApiOutput>) {
        self.answers.entry(name).or_default().push_back(answer);
    }

    pub fn fail_next(&mut self, name: &'static str, message: &str) {
        self.answer_next(name, Err(ApiError(message.to_string())));
    }

    pub fn calls_named(&self, name: &str) -> Vec<&ApiOp> {
        self.calls.iter().filter(|op| op_name(op) == name).collect()
    }

    pub fn called(&self, name: &str) -> bool {
        !self.calls_named(name).is_empty()
    }

    fn answer(&mut self, op: &ApiOp) -> ApiResult<ApiOutput> {
        if let Some(queued) = self
            .answers
            .get_mut(op_name(op))
            .and_then(|q| q.pop_front())
        {
            return queued;
        }
        Ok(match op {
            ApiOp::UpdateItem {
                item_id,
                body,
                level,
                collapsed,
            } => {
                let mut item =
                    plain_item(item_id, level.unwrap_or(1), body.as_deref().unwrap_or(""));
                item.ast = format!("ast:{}", item.body);
                item.properties = tags_in(&item.body);
                item.collapsed = collapsed.unwrap_or(false);
                ApiOutput::Item(item)
            }
            ApiOp::CreateItem {
                page_id,
                level,
                body,
                id,
                ..
            } => {
                self.created += 1;
                let new_id = id
                    .clone()
                    .filter(|i| !i.is_empty())
                    .unwrap_or_else(|| format!("new{}", self.created));
                let mut item = plain_item(&new_id, *level, body);
                item.page_id = page_id.clone();
                item.idx = format!("created{}", self.created);
                item.properties = tags_in(body);
                ApiOutput::Item(item)
            }
            ApiOp::ShiftItemLevels { .. } => ApiOutput::Items(Vec::new()),
            ApiOp::MoveItems { item_ids, .. } => ApiOutput::Items(
                item_ids
                    .iter()
                    .map(|id| plain_item(id, 1, &format!("body of {id}")))
                    .collect(),
            ),
            ApiOp::DeleteItem { .. } | ApiOp::DeletePage { .. } => ApiOutput::Done,
            ApiOp::CycleTask { item_id } => {
                let mut item = plain_item(item_id, 1, "");
                item.types = vec!["task".into()];
                ApiOutput::Item(item)
            }
            ApiOp::UpsertItemProperty {
                item_id,
                key,
                value,
                ..
            } => {
                let mut item = plain_item(item_id, 1, "");
                item.types = vec!["task".into()];
                item.properties
                    .insert(key.clone(), serde_json::Value::String(value.clone()));
                ApiOutput::Item(item)
            }
            ApiOp::GetPageById { page_id } => ApiOutput::Page(PageWithItems {
                page: Page {
                    id: page_id.clone(),
                    name: "reloaded".into(),
                    page_type: "doc".into(),
                    ..Default::default()
                },
                items: vec![plain_item("reloaded", 1, "reloaded")],
                ..Default::default()
            }),
            ApiOp::GetOrCreateJournal { file_name } => ApiOutput::Page(PageWithItems {
                page: Page {
                    id: format!("page-{file_name}"),
                    name: file_name.clone(),
                    page_type: "journal".into(),
                    ..Default::default()
                },
                items: vec![plain_item(&format!("item-{file_name}"), 1, "")],
                ..Default::default()
            }),
            ApiOp::GetOrCreateDoc { title, dir_path } => ApiOutput::Page(PageWithItems {
                page: Page {
                    id: format!("page-{title}"),
                    name: title.clone(),
                    dir_path: dir_path.clone(),
                    page_type: "doc".into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
            ApiOp::ParseFlatMarkdown { markdown } => ApiOutput::ParsedItems(
                markdown
                    .split('\n')
                    .filter(|l| !l.is_empty())
                    .map(|l| {
                        let body = l.trim_start();
                        ParsedFlatItem {
                            level: 1 + ((l.len() - body.len()) / 2) as i32,
                            body: body.to_string(),
                            ..Default::default()
                        }
                    })
                    .collect(),
            ),
            ApiOp::ImportTreeMarkdown { .. } => ApiOutput::FlatMarkdown("from tree".into()),
            ApiOp::UploadAsset { .. } => ApiOutput::FileName("pic.png".into()),
            ApiOp::ExportTreeMarkdown { .. } => ApiOutput::TreeExport(TreeExport {
                tree_markdown: "- tree".into(),
                ast: String::new(),
                flat_markdown: "flat".into(),
            }),
            ApiOp::SetTransclusion {
                item_id,
                source_item_id,
            } => {
                let mut item = plain_item(item_id, 1, "");
                item.transclude_item_id = Some(source_item_id.clone());
                ApiOutput::Item(item)
            }
            ApiOp::RemoveTransclusion { item_id } => ApiOutput::Item(plain_item(item_id, 1, "")),
            ApiOp::SearchCombined { .. } | ApiOp::SearchPageNames { .. } => {
                ApiOutput::SearchResults(Vec::new())
            }
            ApiOp::GetRecentPages { .. } | ApiOp::GetChildPages { .. } => {
                ApiOutput::Pages(Vec::new())
            }
            ApiOp::GetConfig => ApiOutput::Config(ServerConfig::default()),
            ApiOp::UpdateConfig { tags_dir, .. } => ApiOutput::Config(ServerConfig {
                tags_dir: tags_dir.clone().unwrap_or_default(),
                ..ServerConfig::default()
            }),
            ApiOp::GetPropertyDefinitions => ApiOutput::PropertyDefinitions(Vec::new()),
            ApiOp::AddPropertyDefinition { key, value_type } => {
                ApiOutput::PropertyDefinitions(vec![PropertyDefinition {
                    key: key.clone(),
                    value_type: value_type.clone(),
                    system: false,
                    page: true,
                }])
            }
            ApiOp::GetFunctions => ApiOutput::Functions(Vec::new()),
            ApiOp::UpdatePage {
                page_id,
                title,
                dir_path,
            } => ApiOutput::PageInfo(Page {
                id: page_id.clone(),
                name: title.clone().unwrap_or_default(),
                dir_path: dir_path.clone().unwrap_or_default(),
                page_type: "doc".into(),
                ..Default::default()
            }),
            ApiOp::UpsertPageProperty {
                page_id,
                key,
                value,
                ..
            } => {
                let mut page = Page {
                    id: page_id.clone(),
                    modified_date: 42,
                    ..Default::default()
                };
                // The page's own dates are not stored properties: the server moves the date
                match key.as_str() {
                    "created" => page.created_date = 42,
                    "modified" => {}
                    _ => {
                        page.properties
                            .insert(key.clone(), serde_json::Value::String(value.clone()));
                    }
                }
                ApiOutput::PageInfo(page)
            }
            ApiOp::DeletePageProperty { page_id, .. } => ApiOutput::PageInfo(Page {
                id: page_id.clone(),
                ..Default::default()
            }),
            ApiOp::BrowseFolder => ApiOutput::Folder(FolderChoice::default()),
            ApiOp::LaunchUrl { .. } => ApiOutput::Launched(true),
            ApiOp::GetMostRecentJournalBefore { .. }
            | ApiOp::GetMostRecentWeeklyJournalBefore { .. } => {
                ApiOutput::JournalBefore(JournalBefore::default())
            }
        })
    }
}

pub struct Harness {
    app: PetaApp,
    pub model: Model,
    root: Command<Effect, Event>,
    pub api: FakeApi,
    /// Timers started and not yet fired or cancelled.
    pub timers: Vec<Request<TimerOp>>,
    pub sse: Vec<Request<SseOp>>,
    pub ui: Vec<UiOp>,
    pub navigations: Vec<NavigateOp>,
    /// What `ReadClipboardText` answers.
    pub clipboard_text: String,
    /// What `Confirm` answers.
    pub confirm: bool,
    /// What `Prompt` answers.
    pub prompt: Option<String>,
    pub now_ms: u64,
}

impl Default for Harness {
    fn default() -> Self {
        Harness::new()
    }
}

impl Harness {
    pub fn new() -> Harness {
        Harness {
            app: PetaApp,
            model: Model::default(),
            root: Command::done(),
            api: FakeApi::default(),
            timers: Vec::new(),
            sse: Vec::new(),
            ui: Vec::new(),
            navigations: Vec::new(),
            clipboard_text: String::new(),
            confirm: true,
            prompt: None,
            now_ms: 1_000_000,
        }
    }

    /// A harness with items on a doc page, like most Angular specs start.
    pub fn with_items(items: Vec<Item>) -> Harness {
        let mut h = Harness::new();
        h.model.state.set_items(items);
        h.model.screen = crate::app::Screen::Page(crate::app::PageState {
            page_name: "test-page".into(),
            page: Some(Page {
                id: "page1".into(),
                name: "test-page".into(),
                page_type: "doc".into(),
                ..Default::default()
            }),
            ..Default::default()
        });
        h
    }

    /// The view model as the shell would get it now.
    pub fn view(&self) -> crate::ViewModel {
        self.app.view(&self.model)
    }

    /// Sends an event and runs everything it causes, answering the server at once.
    pub fn send(&mut self, event: Event) {
        self.model.clock_ms = self.now_ms;
        let cmd = self.app.update(event, &mut self.model);
        self.root.spawn(|ctx| cmd.into_future(ctx));
        self.pump();
    }

    /// Runs the pending commands until nothing moves without the shell: all events are
    /// processed and every effect except timers and streams is answered.
    pub fn pump(&mut self) {
        loop {
            let events: Vec<Event> = self.root.events().collect();
            if !events.is_empty() {
                for event in events {
                    self.model.clock_ms = self.now_ms;
                    let cmd = self.app.update(event, &mut self.model);
                    self.root.spawn(|ctx| cmd.into_future(ctx));
                }
                continue;
            }
            let effects: Vec<Effect> = self.root.effects().collect();
            if effects.is_empty() {
                break;
            }
            for effect in effects {
                self.handle(effect);
            }
        }
    }

    fn handle(&mut self, effect: Effect) {
        match effect {
            Effect::Render(_) => {}
            Effect::Api(mut request) => {
                let op = request.operation.clone();
                let answer = self.api.answer(&op);
                self.api.calls.push(op);
                let _ = request.resolve(answer);
            }
            Effect::Timer(mut request) => match request.operation {
                TimerOp::Start { .. } => self.timers.push(request),
                TimerOp::Cancel { id } => {
                    if let Some(pos) = self.timers.iter().position(
                        |t| matches!(t.operation, TimerOp::Start { id: i, .. } if i == id),
                    ) {
                        let mut t = self.timers.remove(pos);
                        let _ = t.resolve(TimerOutput::Cancelled);
                    }
                    let _ = request.resolve(TimerOutput::Cancelled);
                }
            },
            Effect::Sse(request) => self.sse.push(request),
            Effect::Ui(mut request) => {
                let op = request.operation.clone();
                let answer = match &op {
                    UiOp::ReadClipboardText => UiOutput::Text(Some(self.clipboard_text.clone())),
                    UiOp::Confirm(_) => UiOutput::Bool(self.confirm),
                    UiOp::Prompt(_) => UiOutput::Text(self.prompt.clone()),
                    UiOp::LocalNow => UiOutput::Now {
                        local: LocalDateTimeDto {
                            year: 2026,
                            month: 2,
                            day: 10,
                            hour: 9,
                            minute: 5,
                        },
                        now_ms: self.now_ms,
                    },
                    _ => UiOutput::Done,
                };
                self.ui.push(op);
                let _ = request.resolve(answer);
            }
            Effect::Navigate(mut request) => {
                self.navigations.push(request.operation.clone());
                let _ = request.resolve(());
            }
        }
    }

    /// Fires every pending timer, oldest first, and runs what follows.
    pub fn fire_timers(&mut self) {
        let timers = std::mem::take(&mut self.timers);
        for mut t in timers {
            let _ = t.resolve(TimerOutput::Fired);
        }
        self.pump();
    }

    pub fn pending_timer_ids(&self) -> Vec<u64> {
        self.timers
            .iter()
            .filter_map(|t| match t.operation {
                TimerOp::Start { id, .. } => Some(id),
                _ => None,
            })
            .collect()
    }

    pub fn items_ids(&self) -> Vec<String> {
        self.model
            .state
            .items()
            .iter()
            .map(|i| i.id.clone())
            .collect()
    }

    /// `id=body` of every own item.
    pub fn bodies(&self) -> Vec<String> {
        self.model
            .state
            .items()
            .iter()
            .map(|i| format!("{}={}", i.id, i.body))
            .collect()
    }

    /// `id:level=body` of every own item.
    pub fn outline(&self) -> Vec<String> {
        self.model
            .state
            .items()
            .iter()
            .map(|i| format!("{}:{}={}", i.id, i.level, i.body))
            .collect()
    }

    pub fn focused_editor_requests(&self) -> Vec<Option<String>> {
        self.ui
            .iter()
            .filter_map(|u| match u {
                UiOp::FocusEditor { item_id, .. } => Some(item_id.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn focus_cursor(&self) -> Option<usize> {
        self.model.state.focus_request().and_then(|r| r.cursor)
    }
}

/// A body-only item fixture, like `item(id, level, { body })` in the specs.
pub fn item(id: &str, level: i32, body: &str) -> Item {
    plain_item(id, level, body)
}

/// The create calls as `(page, position, level, body, id)`.
pub fn create_calls(api: &FakeApi) -> Vec<(String, String, i32, String, Option<String>)> {
    api.calls
        .iter()
        .filter_map(|op| match op {
            ApiOp::CreateItem {
                page_id,
                position,
                level,
                body,
                id,
            } => Some((
                page_id.clone(),
                position.to_api(),
                *level,
                body.clone(),
                id.clone(),
            )),
            _ => None,
        })
        .collect()
}

/// The update calls as `(item, body)`.
pub fn update_calls(api: &FakeApi) -> Vec<(String, Option<String>)> {
    api.calls
        .iter()
        .filter_map(|op| match op {
            ApiOp::UpdateItem { item_id, body, .. } => Some((item_id.clone(), body.clone())),
            _ => None,
        })
        .collect()
}

/// The move calls as `(ids, page, position)`.
pub fn move_calls(api: &FakeApi) -> Vec<(Vec<String>, String, String)> {
    api.calls
        .iter()
        .filter_map(|op| match op {
            ApiOp::MoveItems {
                item_ids,
                target_page_id,
                position,
                ..
            } => Some((item_ids.clone(), target_page_id.clone(), position.to_api())),
            _ => None,
        })
        .collect()
}
