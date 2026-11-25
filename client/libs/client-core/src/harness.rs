//! A test runtime for the core, like `outline-test-support.ts`: drives the commands an
//! event produces, answers the API like the server would, keeps timers to be fired by the
//! test, and records what the shell was asked to do.

use crate::app::{Event, Model, PetaApp};
use crate::effects::*;
use crate::model::*;
use crate::testing::plain_item;
use crux_core::{App, Command, Request};
use std::collections::{BTreeMap, HashMap, VecDeque};

/// The hashtags of a body as the server stores them in the tags property.
fn tags_in(body: &str) -> BTreeMap<String, crate::model::PropertyValue> {
    let tags: Vec<String> = body
        .split(|c: char| c.is_whitespace())
        .filter_map(|w| w.strip_prefix('#'))
        .filter(|t| {
            !t.is_empty()
                && t.chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        })
        .map(str::to_string)
        .collect();
    let mut props = BTreeMap::new();
    if !tags.is_empty() {
        props.insert("tags".to_string(), crate::model::PropertyValue::List(tags));
    }
    props
}

/// Gives an item a task state, as the server writes one.
fn put_task_state(item: &mut Item, state: &TaskState) {
    if state.task {
        item.types = vec!["task".into()];
    }
    let times = [
        ("begin", &state.begin),
        ("end", &state.end),
        ("plan", &state.plan),
    ];
    for (key, value) in times {
        if let Some(value) = value {
            item.properties.insert(
                key.to_string(),
                crate::model::PropertyValue::Text(value.clone()),
            );
        }
    }
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

/// An index that sorts right above `index`, after every index that starts with a smaller
/// last character.
fn index_right_above(index: &str) -> String {
    let mut chars: Vec<char> = index.chars().collect();
    let last = chars.pop().expect("an index is not empty");
    chars.push(char::from_u32(last as u32 - 1).expect("a character below the last one"));
    chars.push(char::MAX);
    chars.into_iter().collect()
}

pub fn op_name(op: &ApiOp) -> &'static str {
    match op {
        ApiOp::GetPageById { .. } => "GetPageById",
        ApiOp::GetMostRecentJournalBefore { .. } => "GetMostRecentJournalBefore",
        ApiOp::GetMostRecentWeeklyJournalBefore { .. } => "GetMostRecentWeeklyJournalBefore",
        ApiOp::GetOrCreateJournal { .. } => "GetOrCreateJournal",
        ApiOp::MoveItemsToJournal { .. } => "MoveItemsToJournal",
        ApiOp::GetOrCreateDoc { .. } => "GetOrCreateDoc",
        ApiOp::FindPage { .. } => "FindPage",
        ApiOp::CreateItem { .. } => "CreateItem",
        ApiOp::UpdateItem { .. } => "UpdateItem",
        ApiOp::ShiftItemLevels { .. } => "ShiftItemLevels",
        ApiOp::MoveItems { .. } => "MoveItems",
        ApiOp::DeletePage { .. } => "DeletePage",
        ApiOp::DeleteItems { .. } => "DeleteItems",
        ApiOp::MergeItems { .. } => "MergeItems",
        ApiOp::PasteText { .. } => "PasteText",
        ApiOp::SplitItem { .. } => "SplitItem",
        ApiOp::StashItem { .. } => "StashItem",
        ApiOp::ApplyBatch { .. } => "ApplyBatch",
        ApiOp::CycleTask { .. } => "CycleTask",
        ApiOp::UpsertItemProperty { .. } => "UpsertItemProperty",
        ApiOp::DeleteItemProperty { .. } => "DeleteItemProperty",
        ApiOp::ChangeAtlas { .. } => "ChangeAtlas",
        ApiOp::SearchCombined { .. } => "SearchCombined",
        ApiOp::SearchPageNames { .. } => "SearchPageNames",
        ApiOp::GetRecentPages { .. } => "GetRecentPages",
        ApiOp::UpdatePage { .. } => "UpdatePage",
        ApiOp::DemotePage { .. } => "DemotePage",
        ApiOp::UpsertPageProperty { .. } => "UpsertPageProperty",
        ApiOp::DeletePageProperty { .. } => "DeletePageProperty",
        ApiOp::GetConfig => "GetConfig",
        ApiOp::GetPropertyDefinitions => "GetPropertyDefinitions",
        ApiOp::DeletePropertyDefinition { .. } => "DeletePropertyDefinition",
        ApiOp::AddPropertyDefinition { .. } => "AddPropertyDefinition",
        ApiOp::UpdateConfig { .. } => "UpdateConfig",
        ApiOp::BrowseFolder => "BrowseFolder",
        ApiOp::ExportTreeMarkdown { .. } => "ExportTreeMarkdown",
        ApiOp::TranscludedItems { .. } => "TranscludedItems",
        ApiOp::SetTransclusion { .. } => "SetTransclusion",
        ApiOp::RemoveTransclusion { .. } => "RemoveTransclusion",
        ApiOp::AnswerConfirm { .. } => "AnswerConfirm",
        ApiOp::GetChildPages { .. } => "GetChildPages",
        ApiOp::GetBacklinks { .. } => "GetBacklinks",
        ApiOp::UploadAsset { .. } => "UploadAsset",
        ApiOp::OpenExportFolder { .. } => "OpenExportFolder",
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

    /// `stored` is the item the server has for an id: the fake keeps no items of its own,
    /// so the harness lets it look at those of the model.
    fn answer(
        &mut self,
        op: &ApiOp,
        stored: &dyn Fn(&str) -> Option<Item>,
        shown: &dyn Fn() -> Vec<Item>,
    ) -> ApiResult<ApiOutput> {
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
            // The server moves the item with the items below it under the stash heading of
            // the page, which it makes when there is none
            ApiOp::StashItem { item_id, body } => {
                let on_page = shown();
                let item = stored(item_id).unwrap_or_else(|| plain_item(item_id, 1, ""));
                let (heading, created) = match on_page.iter().find(|it| it.body.trim() == "# Stash")
                {
                    Some(heading) => (heading.clone(), false),
                    None => {
                        self.created += 1;
                        let mut heading = plain_item(&format!("new{}", self.created), 1, "# Stash");
                        heading.page_id = item.page_id.clone();
                        (heading, true)
                    }
                };
                let delta = heading.level + 1 - item.level;
                let items: Vec<Item> = on_page
                    .iter()
                    .skip_while(|it| it.id != item.id)
                    .enumerate()
                    .take_while(|(n, it)| *n == 0 || it.level > item.level)
                    .map(|(n, it)| {
                        let mut moved = it.clone();
                        moved.level += delta;
                        if let (0, Some(body)) = (n, body) {
                            moved.body = body.clone();
                        }
                        moved
                    })
                    .collect();
                ApiOutput::Stashed(Box::new(Stashed {
                    heading,
                    created,
                    items,
                }))
            }
            // The server answers with the moved items at their new levels, the first with
            // the body the request carries
            ApiOp::MoveItems {
                item_ids,
                level_delta,
                body,
                ..
            } => ApiOutput::Items(
                item_ids
                    .iter()
                    .enumerate()
                    .map(|(n, id)| {
                        let mut item = stored(id).unwrap_or_else(|| plain_item(id, 1, ""));
                        item.level = (item.level + level_delta.unwrap_or(0)).max(1);
                        if let (0, Some(body)) = (n, body) {
                            item.body = body.clone();
                        }
                        item
                    })
                    .collect(),
            ),
            // The server makes an empty item on a page the deletion leaves without one
            ApiOp::DeleteItems { item_ids } => {
                let on_page = shown();
                let emptied = on_page.iter().all(|it| item_ids.contains(&it.id));
                let created = on_page.first().filter(|_| emptied).map(|first| {
                    self.created += 1;
                    let mut item = plain_item(&format!("new{}", self.created), 1, "");
                    item.page_id = first.page_id.clone();
                    item
                });
                ApiOutput::Items(created.into_iter().collect())
            }
            ApiOp::DeletePage { .. } | ApiOp::AnswerConfirm { .. } => ApiOutput::Done,
            // The server joins the bodies the request carries, and those it has for the rest
            ApiOp::MergeItems {
                item_id,
                into_id,
                body,
                into_body,
            } => {
                let body_of = |id: &str, carried: &Option<String>| {
                    carried
                        .clone()
                        .or_else(|| stored(id).map(|item| item.body))
                        .unwrap_or_default()
                };
                let joined = format!("{}{}", body_of(into_id, into_body), body_of(item_id, body));
                let item = plain_item(into_id, 1, &joined);
                ApiOutput::Item(item)
            }
            // The text goes into the item as one piece; a test that wants more items answers
            // the paste itself
            ApiOp::PasteText {
                item_id,
                body,
                cursor,
                text,
                ..
            } => {
                let (before, after) = crate::util::split_at_char(body, *cursor);
                let item = plain_item(item_id, 1, &format!("{before}{text}{after}"));
                ApiOutput::Pasted(Pasted {
                    items: vec![item],
                    cursor: PasteCursor {
                        item_id: item_id.clone(),
                        position: crate::util::char_len(before) + crate::util::char_len(text),
                    },
                })
            }
            // The fake does not know the page. It answers as for an item without children, at
            // level 1, with an index that sorts the new item right above or right below an
            // item whose index is its id. A test that wants another place answers the split itself
            ApiOp::SplitItem {
                item_id,
                body,
                cursor,
                ..
            } => {
                self.created += 1;
                let (before, after) = crate::util::split_at_char(body, *cursor);
                let above = *cursor == 0 && !body.is_empty();
                let (kept, new_body) = if above {
                    (body.as_str(), "")
                } else {
                    (before, after)
                };
                let mut item = plain_item(item_id, 1, kept);
                item.properties = tags_in(kept);
                let mut created = plain_item(&format!("new{}", self.created), 1, new_body);
                created.properties = tags_in(new_body);
                created.idx = if above {
                    index_right_above(item_id)
                } else {
                    format!("{item_id}\u{1}")
                };
                ApiOutput::Split(Box::new(Split { item, created }))
            }
            ApiOp::ApplyBatch { ops } => {
                let mut written: Vec<Item> = Vec::new();
                for op in ops {
                    let item = match op {
                        BatchOpInput::Update {
                            id,
                            body,
                            level,
                            collapsed,
                            task,
                        } => {
                            let body = body.as_deref().unwrap_or("");
                            let mut item = plain_item(id, level.unwrap_or(1), body);
                            item.properties = tags_in(body);
                            item.collapsed = collapsed.unwrap_or(false);
                            if let Some(state) = task {
                                put_task_state(&mut item, state);
                            }
                            item
                        }
                        BatchOpInput::Create {
                            id,
                            page_id,
                            level,
                            body,
                            ..
                        } => {
                            self.created += 1;
                            let mut item = plain_item(id, *level, body);
                            item.page_id = page_id.clone();
                            item.idx = format!("created{}", self.created);
                            item
                        }
                        BatchOpInput::Move { ids, .. } => {
                            // A moved item keeps what an update before it wrote, at a new index
                            for id in ids {
                                self.created += 1;
                                // The core sends the update of a moved item first
                                let place = written.iter().position(|earlier| &earlier.id == id);
                                let mut item = written.remove(place.expect("updated before"));
                                item.idx = format!("moved{}", self.created);
                                written.push(item);
                            }
                            continue;
                        }
                        BatchOpInput::Delete { .. } => continue,
                    };
                    // An item written twice is answered once, as it is at the end
                    written.retain(|earlier| earlier.id != item.id);
                    written.push(item);
                }
                ApiOutput::Batched(Box::new(Batched {
                    items: written,
                    created: Vec::new(),
                }))
            }
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
                item.properties.insert(
                    key.clone(),
                    crate::model::PropertyValue::Text(value.clone()),
                );
                ApiOutput::Item(item)
            }
            ApiOp::DeleteItemProperty { item_id, .. } => {
                ApiOutput::Item(plain_item(item_id, 1, ""))
            }
            ApiOp::ChangeAtlas { .. } => ApiOutput::Items(Vec::new()),
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
            ApiOp::GetOrCreateJournal { file_name } => ApiOutput::Journal(JournalPage {
                page: Page {
                    id: format!("page-{file_name}"),
                    name: file_name.clone(),
                    page_type: "journal".into(),
                    ..Default::default()
                },
                items: vec![plain_item(&format!("item-{file_name}"), 1, "")],
                ..Default::default()
            }),
            // The server answers with the journal of the day and the moved items, the first
            // with the body the request carries
            ApiOp::MoveItemsToJournal {
                file_name,
                item_ids,
                body,
                ..
            } => ApiOutput::MovedToJournal(MovedToJournal {
                page: Page {
                    id: format!("page-{file_name}"),
                    name: file_name.clone(),
                    page_type: "journal".into(),
                    ..Default::default()
                },
                items: item_ids
                    .iter()
                    .enumerate()
                    .map(|(n, id)| {
                        let mut item = stored(id).unwrap_or_else(|| plain_item(id, 1, ""));
                        if let (0, Some(body)) = (n, body) {
                            item.body = body.clone();
                        }
                        item
                    })
                    .collect(),
                created: false,
            }),
            ApiOp::GetOrCreateDoc { title, dir_path } => ApiOutput::DocPage(DocPage {
                page: Page {
                    id: format!("page-{title}"),
                    name: title.clone(),
                    dir_path: dir_path.clone(),
                    page_type: "doc".into(),
                    ..Default::default()
                },
                items: Vec::new(),
                created: false,
            }),
            // Every page is there, unless a test says it is gone
            ApiOp::FindPage {
                journal,
                title,
                dir_path,
            } => ApiOutput::FoundPage(Some(Page {
                id: format!("page-{}", journal.as_ref().unwrap_or(title)),
                name: journal.clone().unwrap_or_else(|| title.clone()),
                dir_path: dir_path.clone(),
                ..Default::default()
            })),
            ApiOp::UploadAsset { .. } => ApiOutput::Uploaded(Uploaded {
                file_name: "pic.png".into(),
                original_name: "pic.png".into(),
            }),
            ApiOp::TranscludedItems { .. } => ApiOutput::ItemIds(Vec::new()),
            ApiOp::ExportTreeMarkdown { .. } => ApiOutput::TreeExport(TreeExport {
                tree_markdown: "- tree".into(),
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
            ApiOp::GetBacklinks { .. } => ApiOutput::Backlinks(Vec::new()),
            ApiOp::GetConfig => ApiOutput::Config(ServerConfig::default()),
            ApiOp::UpdateConfig {
                tags_dir,
                home_link,
                ..
            } => ApiOutput::Config(ServerConfig {
                tags_dir: tags_dir.clone().unwrap_or_default(),
                home_link: home_link.clone().unwrap_or_default(),
                ..ServerConfig::default()
            }),
            ApiOp::GetPropertyDefinitions | ApiOp::DeletePropertyDefinition { .. } => {
                ApiOutput::PropertyDefinitions(Vec::new())
            }
            ApiOp::AddPropertyDefinition { key, value_type } => {
                ApiOutput::PropertyDefinitions(vec![PropertyDefinition {
                    key: key.clone(),
                    value_type: value_type.clone(),
                    system: false,
                    page: true,
                }])
            }
            ApiOp::UpdatePage {
                page_id,
                title,
                dir_path,
                ..
            } => ApiOutput::PageInfo(Page {
                id: page_id.clone(),
                name: title.clone().unwrap_or_default(),
                dir_path: dir_path.clone().unwrap_or_default(),
                page_type: "doc".into(),
                ..Default::default()
            }),
            ApiOp::DemotePage { page_id, .. } => ApiOutput::PageInfo(Page {
                id: page_id.clone(),
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
                        page.properties.insert(
                            key.clone(),
                            crate::model::PropertyValue::Text(value.clone()),
                        );
                    }
                }
                ApiOutput::PageInfo(page)
            }
            ApiOp::DeletePageProperty { page_id, .. } => ApiOutput::PageInfo(Page {
                id: page_id.clone(),
                ..Default::default()
            }),
            ApiOp::BrowseFolder => ApiOutput::Folder(FolderChoice::default()),
            ApiOp::OpenExportFolder { .. } => ApiOutput::Done,
            ApiOp::GetMostRecentJournalBefore { .. }
            | ApiOp::GetMostRecentWeeklyJournalBefore { .. } => {
                ApiOutput::JournalBefore(JournalBefore::default())
            }
        })
    }
}

pub struct Harness {
    app: PetaApp<crate::RichText>,
    pub model: Model,
    root: Command<Effect, Event>,
    pub api: FakeApi,
    /// Timers started and not yet fired or cancelled.
    pub timers: Vec<Request<TimerOp>>,
    pub sse: Vec<Request<SseOp>>,
    pub ui: Vec<UiOp>,
    /// The item whose editor loses its focus when the shell draws, as the editor of a row put
    /// in another place does. The blur reaches the core before the answer to the focus request.
    pub blurs_when_drawn: Option<String>,
    pub navigations: Vec<NavigateOp>,
    /// What `ReadClipboardText` answers.
    pub clipboard_text: String,
    /// What `Confirm` answers.
    pub confirm: bool,
    /// What `Prompt` answers.
    pub prompt: Option<String>,
    pub now_ms: u64,
    /// What `LocalNow` answers: the local date and time of the shell.
    pub local_now: LocalDateTimeDto,
    /// The operation whose requests wait for `release_held` instead of an answer at once,
    /// as a slow request does.
    holding: Option<&'static str>,
    held: Vec<Request<ApiOp>>,
    /// True when the orders to focus an editor wait for `take_focus` instead of an answer at
    /// once, as on iOS, where the field takes the keyboard a moment after it is drawn.
    pub focus_takes_time: bool,
    held_focus: Vec<Request<UiOp>>,
}

impl Default for Harness {
    fn default() -> Self {
        Harness::new()
    }
}

impl Harness {
    pub fn new() -> Harness {
        Harness {
            app: PetaApp::default(),
            model: Model::default(),
            root: Command::done(),
            api: FakeApi::default(),
            timers: Vec::new(),
            sse: Vec::new(),
            ui: Vec::new(),
            blurs_when_drawn: None,
            navigations: Vec::new(),
            clipboard_text: String::new(),
            confirm: true,
            prompt: None,
            now_ms: 1_000_000,
            local_now: LocalDateTimeDto {
                year: 2026,
                month: 2,
                day: 10,
                hour: 9,
                minute: 5,
            },
            holding: None,
            held: Vec::new(),
            focus_takes_time: false,
            held_focus: Vec::new(),
        }
    }

    /// Holds the requests of an operation until `release_held`, as a slow server does.
    pub fn hold(&mut self, name: &'static str) {
        self.holding = Some(name);
    }

    /// Answers the held requests, oldest first, and runs what follows.
    pub fn release_held(&mut self) {
        self.holding = None;
        for request in std::mem::take(&mut self.held) {
            self.handle(Effect::Api(request));
        }
        self.pump();
    }

    /// The fields of the orders to focus an editor take the keyboard now, oldest first: the
    /// orders are answered, and what follows runs.
    pub fn take_focus(&mut self) {
        for mut request in std::mem::take(&mut self.held_focus) {
            let _ = request.resolve(UiOutput::Done);
        }
        self.pump();
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
    pub fn view(&self) -> crate::ViewModel<crate::RichText> {
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
            Effect::Api(request) if self.holding == Some(op_name(&request.operation)) => {
                self.held.push(request);
            }
            Effect::Api(mut request) => {
                let op = request.operation.clone();
                let state = &self.model.state;
                let answer = self.api.answer(
                    &op,
                    &|id| state.get_item_by_id(id).map(|item| (*item).clone()),
                    &|| state.items().iter().map(|item| (**item).clone()).collect(),
                );
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
                    UiOp::Confirm(_) | UiOp::ConfirmDestructive(_) => UiOutput::Bool(self.confirm),
                    UiOp::Prompt(_) => UiOutput::Text(self.prompt.clone()),
                    UiOp::LocalNow => UiOutput::Now {
                        local: self.local_now,
                        now_ms: self.now_ms,
                    },
                    _ => UiOutput::Done,
                };
                if let UiOp::FocusEditor { item_id, .. } = &op {
                    // A shell that draws from the view model alone has to get every order
                    assert_eq!(
                        self.model.state.focus_order().map(|order| &order.item_id),
                        Some(item_id),
                        "an editor is focused without an order in the view model"
                    );
                    if let Some(item_id) = self.blurs_when_drawn.take() {
                        let cmd = self
                            .app
                            .update(Event::EditFinished { item_id }, &mut self.model);
                        self.root.spawn(|ctx| cmd.into_future(ctx));
                    }
                }
                let focus = matches!(op, UiOp::FocusEditor { .. });
                self.ui.push(op);
                if focus && self.focus_takes_time {
                    self.held_focus.push(request);
                    return;
                }
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

    /// The messages of the alerts the core asked the shell to show, in order.
    pub fn alerts(&self) -> Vec<String> {
        self.ui
            .iter()
            .filter_map(|op| match op {
                UiOp::Alert(m) => Some(m.clone()),
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
                position.to_string(),
                *level,
                body.clone(),
                id.clone(),
            )),
            _ => None,
        })
        .collect()
}

/// The split calls as `(item, body, cursor)`.
pub fn split_calls(api: &FakeApi) -> Vec<(String, String, usize)> {
    api.calls
        .iter()
        .filter_map(|op| match op {
            ApiOp::SplitItem {
                item_id,
                body,
                cursor,
                ..
            } => Some((item_id.clone(), body.clone(), *cursor)),
            _ => None,
        })
        .collect()
}

/// Types `body` into the editor of the item, as a key press does: the editor is open and
/// the text is shown at once, with its auto-save pending.
pub fn typed(h: &mut Harness, item_id: &str, body: &str) {
    h.model.state.add_editing_item(item_id);
    let now = h.now_ms;
    h.send(Event::BodyChanged {
        item_id: item_id.into(),
        body: body.into(),
        now_ms: now,
    });
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

/// The group delete calls, each the ids sent together.
pub fn delete_calls(api: &FakeApi) -> Vec<Vec<String>> {
    api.calls
        .iter()
        .filter_map(|op| match op {
            ApiOp::DeleteItems { item_ids } => Some(item_ids.clone()),
            _ => None,
        })
        .collect()
}

/// The batches sent, each the operations of one request.
pub fn batch_calls(api: &FakeApi) -> Vec<Vec<BatchOpInput>> {
    api.calls
        .iter()
        .filter_map(|op| match op {
            ApiOp::ApplyBatch { ops } => Some(ops.clone()),
            _ => None,
        })
        .collect()
}

/// The merge calls as `(item, into)`.
pub fn merge_calls(api: &FakeApi) -> Vec<(String, String)> {
    api.calls
        .iter()
        .filter_map(|op| match op {
            ApiOp::MergeItems {
                item_id, into_id, ..
            } => Some((item_id.clone(), into_id.clone())),
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
            } => Some((
                item_ids.clone(),
                target_page_id.clone(),
                position.to_string(),
            )),
            _ => None,
        })
        .collect()
}
