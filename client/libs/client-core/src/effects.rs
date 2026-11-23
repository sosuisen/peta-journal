//! What the core asks the shell to do. Every variant of [`Effect`] carries a request the
//! shell performs and resolves; the core never touches the network, timers or the DOM.

use crate::model::{
    BacklinkEntry, BatchOpInput, Batched, DeferredKey, DocPage, Item, ItemPosition, JournalBefore,
    JournalPage, MovedToJournal, Page, PageWithItems, PasteMode, Pasted, PropertyDefinition,
    SearchResult, ServerConfig, Split, Stashed, TreeExport, Uploaded,
};
use crate::screens::page::split_doc_path;
use crate::util::date::LocalDateTime;
use crate::util::filename::is_journal_file_name;
use crux_core::capability::Operation;
use crux_core::command::CommandContext;
use crux_core::render::RenderOperation;
use serde::{Deserialize, Serialize};

// ---- API ----

/// One call of the journal server's REST API; the same surface as `api.service.ts`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ApiOp {
    GetPageById {
        page_id: String,
    },
    GetMostRecentJournalBefore {
        before: String,
    },
    GetMostRecentWeeklyJournalBefore {
        before: String,
    },
    GetOrCreateJournal {
        file_name: String,
    },
    /// Moves items to the journal of the day `file_name`, which is made when there is none.
    MoveItemsToJournal {
        file_name: String,
        item_ids: Vec<String>,
        position: ItemPosition,
        /// The body the editor of the first item has now, when it is not saved yet.
        body: Option<String>,
    },
    GetOrCreateDoc {
        title: String,
        dir_path: String,
    },
    /// Looks for a journal by its name, or else a doc page by its title and folder, and
    /// makes none
    FindPage {
        journal: Option<String>,
        title: String,
        dir_path: String,
    },
    CreateItem {
        page_id: String,
        position: ItemPosition,
        level: i32,
        body: String,
        id: Option<String>,
    },
    UpdateItem {
        item_id: String,
        body: Option<String>,
        level: Option<i32>,
        collapsed: Option<bool>,
    },
    ShiftItemLevels {
        item_ids: Vec<String>,
        delta: i32,
    },
    MoveItems {
        item_ids: Vec<String>,
        target_page_id: String,
        position: ItemPosition,
        level_delta: Option<i32>,
        /// The body the editor of the first item has now, when it is not saved yet.
        body: Option<String>,
    },
    DeletePage {
        page_id: String,
    },
    /// Deletes items as one group, with their descendants; the server refuses a group that
    /// leaves a descendant behind.
    DeleteItems {
        item_ids: Vec<String>,
    },
    /// Joins `item_id` into `into_id`: the body goes to the end of the receiving item, the
    /// children follow, and the item is deleted.
    MergeItems {
        item_id: String,
        into_id: String,
        /// The body an editor has now for the item that goes away, when it is not saved yet.
        body: Option<String>,
        /// The same for the item that takes the text.
        into_body: Option<String>,
    },
    /// Pastes `text` at the character `cursor` of `body`, the body the item has on screen. The
    /// server cuts the text into items and writes them all, or none.
    PasteText {
        item_id: String,
        body: String,
        cursor: usize,
        text: String,
        mode: PasteMode,
    },
    /// Sends an item with its descendants under the `# Stash` heading of its page, which the
    /// server makes at the end of the page when there is none.
    StashItem {
        item_id: String,
        /// The body the editor of the item has now, when it is not saved yet.
        body: Option<String>,
    },
    /// Splits an item at the character `cursor` of `body`, the body it has on screen, as
    /// Enter does. The server decides where the new item goes and writes both, or neither.
    SplitItem {
        item_id: String,
        /// The transclusion that shows the item, for an item of another page shown below it.
        root_id: Option<String>,
        body: String,
        cursor: usize,
    },
    /// Applies operations to items in their order, all of them or none, as an undo or a redo
    /// sends what it replays.
    ApplyBatch {
        ops: Vec<BatchOpInput>,
    },
    CycleTask {
        item_id: String,
    },
    UpsertItemProperty {
        item_id: String,
        key: String,
        value_type: String,
        value: String,
    },
    DeleteItemProperty {
        item_id: String,
        key: String,
    },
    ChangeAtlas {
        atlas_id: String,
        change: AtlasChange,
    },
    SearchCombined {
        query: String,
        max_results: u32,
    },
    SearchPageNames {
        query: String,
        max_results: u32,
    },
    GetRecentPages {
        max_results: u32,
        page_type: Option<String>,
    },
    /// Answers `PageNameTaken` when another page has the new name and `overwrite` is not set.
    UpdatePage {
        page_id: String,
        title: Option<String>,
        dir_path: Option<String>,
        /// Deletes the other page that has the new name, so this page takes its place.
        overwrite: bool,
    },
    /// Demotes the index page of a directory to the page of the directory's name. The server
    /// deletes the atlas page of the directory in the same write.
    DemotePage {
        page_id: String,
        /// Deletes the other page that has the new name, so this page takes its place.
        overwrite: bool,
    },
    /// Sets a property of a page; `tags` takes a comma separated list. Answers with the page.
    UpsertPageProperty {
        page_id: String,
        key: String,
        value_type: String,
        value: String,
    },
    DeletePageProperty {
        page_id: String,
        key: String,
    },
    GetConfig,
    GetPropertyDefinitions,
    /// Defines a user key; answers with the whole definition list.
    AddPropertyDefinition {
        key: String,
        value_type: String,
    },
    /// Removes a user key with all its values; answers with the whole definition list.
    DeletePropertyDefinition {
        key: String,
    },
    UpdateConfig {
        markdown_dir: Option<String>,
        week_numbering: Option<String>,
        logical_outdent: Option<bool>,
        tags_dir: Option<String>,
        home_link: Option<String>,
    },
    BrowseFolder,
    ExportTreeMarkdown {
        item_ids: Vec<String>,
    },
    /// Which of the items other items transclude; asked before a deletion.
    TranscludedItems {
        item_ids: Vec<String>,
    },
    SetTransclusion {
        item_id: String,
        source_item_id: String,
    },
    RemoveTransclusion {
        item_id: String,
    },
    /// The user's answer to a question that an extension on the server asked.
    AnswerConfirm {
        request_id: String,
        ok: bool,
    },
    GetChildPages {
        parent_id: String,
    },
    GetBacklinks {
        page_id: String,
    },
    /// Uploads a file the shell holds under a handle (a `File` cannot cross into the core).
    UploadAsset {
        file: u64,
    },
    /// `POST /api/launcher/open` with a custom URL scheme.
    LaunchUrl {
        url: String,
    },
    /// `POST /api/pages/{id}/open-folder`: the OS shows the folder of the page's markdown file.
    OpenExportFolder {
        page_id: String,
    },
}

/// The folder the person chose in the dialog of the server. The server and the client share
/// the type (ADR 082).
pub use journal_api_types::FolderChoice;

/// A change of the outline of an atlas page, which the server makes. Each card is named by the
/// id of its item, or a page card by the id of its page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AtlasChange {
    AddLink {
        target: String,
        bounds: String,
    },
    SetLink {
        item_id: String,
        target: String,
    },
    Group {
        cards: Vec<String>,
        target: String,
    },
    Ungroup {
        cards: Vec<String>,
    },
    SetGroupTitle {
        item_id: String,
        side: String,
        title: String,
    },
    AddCallout {
        bounds: String,
        side: String,
        text: String,
    },
    /// An empty text takes the callout away
    SetCallout {
        item_id: String,
        side: String,
        text: String,
    },
    /// Takes away a link card or a callout
    Remove {
        item_id: String,
    },
}

/// The answer to an [`ApiOp`]; the variant matches the operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ApiOutput {
    Page(PageWithItems),
    Journal(JournalPage),
    DocPage(DocPage),
    MovedToJournal(MovedToJournal),
    /// The page looked for, or none when it is not there
    FoundPage(Option<Page>),
    JournalBefore(JournalBefore),
    Item(Item),
    Items(Vec<Item>),
    PageInfo(Page),
    Pages(Vec<Page>),
    Backlinks(Vec<BacklinkEntry>),
    SearchResults(Vec<SearchResult>),
    Config(ServerConfig),
    PropertyDefinitions(Vec<PropertyDefinition>),
    Folder(FolderChoice),
    TreeExport(TreeExport),
    ItemIds(Vec<String>),
    Pasted(Pasted),
    Split(Box<Split>),
    Stashed(Box<Stashed>),
    Batched(Box<Batched>),
    Uploaded(Uploaded),
    Launched(bool),
    /// Another page has the name a page was to take; the server's message says which.
    PageNameTaken(String),
    Done,
}

/// What updating the title or folder of a page came to.
#[derive(Debug, Clone, PartialEq)]
pub enum PageUpdate {
    Updated(Page),
    /// Another page has the new name; the server's message says which.
    NameTaken(String),
}

fn page_update_of(output: ApiOutput) -> ApiResult<PageUpdate> {
    match output {
        ApiOutput::PageInfo(page) => Ok(PageUpdate::Updated(page)),
        ApiOutput::PageNameTaken(message) => Ok(PageUpdate::NameTaken(message)),
        other => Err(ApiError(format!("unexpected API answer: {other:?}"))),
    }
}

/// The server's error message, or a transport error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiError(pub String);

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl ApiError {
    /// The server's message, or the fallback when the server gave none.
    pub fn message_or(self, fallback: &str) -> String {
        if self.0.is_empty() {
            fallback.to_string()
        } else {
            self.0
        }
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

impl Operation for ApiOp {
    type Output = ApiResult<ApiOutput>;
}

// ---- Timers ----

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimerOp {
    /// Fires after the delay unless cancelled.
    Start { id: u64, ms: u32 },
    /// Stops a timer; its request is resolved as cancelled.
    Cancel { id: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimerOutput {
    Fired,
    Cancelled,
}

impl Operation for TimerOp {
    type Output = TimerOutput;
}

// ---- Server-sent events ----

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SseOp {
    /// Opens `/api/events` and streams what happens on it until it closes.
    Connect,
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SseEvent {
    Open,
    /// The `data` of a message, as JSON text.
    Message(String),
    /// The connection failed; the shell closed it.
    Error,
}

impl Operation for SseOp {
    type Output = SseEvent;
}

// ---- The screen ----

/// Things only the shell can do with the page: focus, scroll, clipboard, dialogs.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UiOp {
    /// Focuses the editor of an item after the next render (the first editor when None) and
    /// plays the held keys into it. Resolved when done.
    FocusEditor {
        item_id: Option<String>,
        replay_keys: Vec<DeferredKey>,
    },
    ScrollToItem {
        item_id: String,
        smooth: bool,
    },
    SetTitle(String),
    /// Keeps the folders last seen as their atlas, so a reload opens them the same way.
    SaveAtlasDirs(Vec<String>),
    /// Copies tree markdown as text and, rendered to HTML, as rich text. The shell may have
    /// begun the clipboard write on the user's key already and only fills it in here.
    ///
    /// `ast` is the AST of the markdown as JSON, which the core parses as the journal parses a
    /// body (ADR 041). A shell makes the HTML from it and does not parse the markdown (ADR
    /// 027). It is empty when the markdown is empty.
    CopyMarkdown {
        markdown: String,
        ast: String,
    },
    /// There is nothing to copy after all: the shell gives up a clipboard write it began.
    CopyFailed,
    /// Resolved with `UiOutput::Text`.
    ReadClipboardText,
    Alert(String),
    /// Resolved with `UiOutput::Bool`.
    Confirm(String),
    /// A confirmation of something hard to undo: the shell's dialog answers No by default,
    /// so Enter or Escape alone never says yes. Resolved with `UiOutput::Bool`.
    ConfirmDestructive(String),
    /// Resolved with `UiOutput::Text`; None when dismissed.
    Prompt(String),
    OpenWindow {
        url: String,
        new_window: bool,
    },
    /// Resolved with `UiOutput::Now`.
    LocalNow,
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UiOutput {
    Done,
    Bool(bool),
    Text(Option<String>),
    Now {
        local: LocalDateTimeDto,
        now_ms: u64,
    },
}

/// `LocalDateTime` in a serializable shape.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LocalDateTimeDto {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
}

impl From<LocalDateTime> for LocalDateTimeDto {
    fn from(d: LocalDateTime) -> Self {
        LocalDateTimeDto {
            year: d.year,
            month: d.month,
            day: d.day,
            hour: d.hour,
            minute: d.minute,
        }
    }
}

impl LocalDateTimeDto {
    /// The name of the journal of this day, `YYYY-MM-DD`.
    pub fn day_name(self) -> String {
        LocalDateTime::from(self).day_name()
    }
}

impl From<LocalDateTimeDto> for LocalDateTime {
    fn from(d: LocalDateTimeDto) -> Self {
        LocalDateTime {
            year: d.year,
            month: d.month,
            day: d.day,
            hour: d.hour,
            minute: d.minute,
        }
    }
}

impl Operation for UiOp {
    type Output = UiOutput;
}

// ---- Navigation ----

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavigateOp {
    pub url: String,
    /// Replace the current history entry instead of pushing one.
    pub replace: bool,
}

impl Operation for NavigateOp {
    type Output = ();
}

#[crux_core::macros::effect]
pub enum Effect {
    Render(RenderOperation),
    Api(ApiOp),
    Timer(TimerOp),
    Sse(SseOp),
    Ui(UiOp),
    Navigate(NavigateOp),
}

// ---- Awaiting the API from a command ----

/// Typed `await`s on the server, for use inside `Command::new(|ctx| async move { ... })`.
pub struct Api<Event> {
    ctx: CommandContext<Effect, Event>,
}

macro_rules! expect_output {
    ($result:expr, $variant:ident) => {
        match $result {
            Ok(ApiOutput::$variant(v)) => Ok(v),
            Ok(other) => Err(ApiError(format!("unexpected API answer: {other:?}"))),
            Err(e) => Err(e),
        }
    };
}

impl<Event: Send + 'static> Api<Event> {
    pub fn new(ctx: CommandContext<Effect, Event>) -> Self {
        Api { ctx }
    }

    pub async fn call(&self, op: ApiOp) -> ApiResult<ApiOutput> {
        self.ctx.request_from_shell(op).await
    }

    pub async fn get_page_by_id(&self, page_id: &str) -> ApiResult<PageWithItems> {
        expect_output!(
            self.call(ApiOp::GetPageById {
                page_id: page_id.into()
            })
            .await,
            Page
        )
    }

    pub async fn get_or_create_journal(&self, file_name: &str) -> ApiResult<JournalPage> {
        expect_output!(
            self.call(ApiOp::GetOrCreateJournal {
                file_name: file_name.into()
            })
            .await,
            Journal
        )
    }

    /// Moves items to the journal of a day as a move key at the edge of a day does: the
    /// journal is made when there is none, and `body`, the text the editor of the first item
    /// has now when it is not saved yet, is saved with the move. Answers with the journal and
    /// the moved items.
    pub async fn move_items_to_journal(
        &self,
        file_name: &str,
        item_ids: Vec<String>,
        position: ItemPosition,
        body: Option<&str>,
    ) -> ApiResult<MovedToJournal> {
        expect_output!(
            self.call(ApiOp::MoveItemsToJournal {
                file_name: file_name.into(),
                item_ids,
                position,
                body: body.map(str::to_string),
            })
            .await,
            MovedToJournal
        )
    }

    /// The page of the name, a journal or a doc page, without making it.
    pub async fn find_page(&self, name: &str) -> ApiResult<Option<Page>> {
        let op = if is_journal_file_name(name) {
            ApiOp::FindPage {
                journal: Some(name.into()),
                title: String::new(),
                dir_path: String::new(),
            }
        } else {
            let (dir_path, title) = split_doc_path(name);
            ApiOp::FindPage {
                journal: None,
                title,
                dir_path,
            }
        };
        expect_output!(self.call(op).await, FoundPage)
    }

    pub async fn get_or_create_doc(&self, title: &str, dir_path: &str) -> ApiResult<DocPage> {
        expect_output!(
            self.call(ApiOp::GetOrCreateDoc {
                title: title.into(),
                dir_path: dir_path.into()
            })
            .await,
            DocPage
        )
    }

    pub async fn journal_before(&self, before: &str, weekly: bool) -> ApiResult<JournalBefore> {
        let op = if weekly {
            ApiOp::GetMostRecentWeeklyJournalBefore {
                before: before.into(),
            }
        } else {
            ApiOp::GetMostRecentJournalBefore {
                before: before.into(),
            }
        };
        expect_output!(self.call(op).await, JournalBefore)
    }

    pub async fn create_item(
        &self,
        page_id: &str,
        position: ItemPosition,
        level: i32,
        body: &str,
        id: Option<String>,
    ) -> ApiResult<Item> {
        expect_output!(
            self.call(ApiOp::CreateItem {
                page_id: page_id.into(),
                position,
                level,
                body: body.into(),
                id
            })
            .await,
            Item
        )
    }

    pub async fn update_body(&self, item_id: &str, body: &str) -> ApiResult<Item> {
        expect_output!(
            self.call(ApiOp::UpdateItem {
                item_id: item_id.into(),
                body: Some(body.into()),
                level: None,
                collapsed: None
            })
            .await,
            Item
        )
    }

    pub async fn update_item(
        &self,
        item_id: &str,
        body: Option<String>,
        level: Option<i32>,
        collapsed: Option<bool>,
    ) -> ApiResult<Item> {
        expect_output!(
            self.call(ApiOp::UpdateItem {
                item_id: item_id.into(),
                body,
                level,
                collapsed
            })
            .await,
            Item
        )
    }

    pub async fn shift_item_levels(
        &self,
        item_ids: Vec<String>,
        delta: i32,
    ) -> ApiResult<Vec<Item>> {
        expect_output!(
            self.call(ApiOp::ShiftItemLevels { item_ids, delta }).await,
            Items
        )
    }

    pub async fn move_items(
        &self,
        item_ids: Vec<String>,
        target_page_id: &str,
        position: ItemPosition,
        level_delta: Option<i32>,
    ) -> ApiResult<Vec<Item>> {
        expect_output!(
            self.call(ApiOp::MoveItems {
                item_ids,
                target_page_id: target_page_id.into(),
                position,
                level_delta,
                body: None,
            })
            .await,
            Items
        )
    }

    /// Moves items on their page as a key in an editor does, `level_delta` levels deeper.
    /// `body` is the text the editor of the first item has now, when it is not saved yet:
    /// the server saves it with the move.
    pub async fn move_edited_items(
        &self,
        item_ids: Vec<String>,
        page_id: &str,
        position: ItemPosition,
        level_delta: i32,
        body: Option<&str>,
    ) -> ApiResult<Vec<Item>> {
        expect_output!(
            self.call(ApiOp::MoveItems {
                item_ids,
                target_page_id: page_id.into(),
                position,
                level_delta: (level_delta != 0).then_some(level_delta),
                body: body.map(str::to_string),
            })
            .await,
            Items
        )
    }

    /// Deletes items as one group. The server keeps every page non-empty: the answer is
    /// the empty items it made on the pages the deletion left without an item.
    pub async fn delete_items(&self, item_ids: Vec<String>) -> ApiResult<Vec<Item>> {
        expect_output!(self.call(ApiOp::DeleteItems { item_ids }).await, Items)
    }

    pub async fn merge_items(
        &self,
        item_id: &str,
        into_id: &str,
        body: Option<&str>,
        into_body: Option<&str>,
    ) -> ApiResult<Item> {
        expect_output!(
            self.call(ApiOp::MergeItems {
                item_id: item_id.into(),
                into_id: into_id.into(),
                body: body.map(Into::into),
                into_body: into_body.map(Into::into),
            })
            .await,
            Item
        )
    }

    pub async fn paste_text(
        &self,
        item_id: &str,
        body: &str,
        cursor: usize,
        text: &str,
        mode: PasteMode,
    ) -> ApiResult<Pasted> {
        expect_output!(
            self.call(ApiOp::PasteText {
                item_id: item_id.into(),
                body: body.into(),
                cursor,
                text: text.into(),
                mode,
            })
            .await,
            Pasted
        )
    }

    pub async fn split_item(
        &self,
        item_id: &str,
        root_id: Option<&str>,
        body: &str,
        cursor: usize,
    ) -> ApiResult<Box<Split>> {
        expect_output!(
            self.call(ApiOp::SplitItem {
                item_id: item_id.into(),
                root_id: root_id.map(Into::into),
                body: body.into(),
                cursor,
            })
            .await,
            Split
        )
    }

    /// Sends an item with its descendants to the stash of its page. `body` is the text its
    /// editor has now, when it is not saved yet: the server saves it with the move.
    pub async fn stash_item(&self, item_id: &str, body: Option<&str>) -> ApiResult<Box<Stashed>> {
        expect_output!(
            self.call(ApiOp::StashItem {
                item_id: item_id.into(),
                body: body.map(str::to_string),
            })
            .await,
            Stashed
        )
    }

    /// The items the batch wrote, as they are after it.
    pub async fn apply_batch(&self, ops: Vec<BatchOpInput>) -> ApiResult<Box<Batched>> {
        expect_output!(self.call(ApiOp::ApplyBatch { ops }).await, Batched)
    }

    pub async fn delete_page(&self, page_id: &str) -> ApiResult<()> {
        self.call(ApiOp::DeletePage {
            page_id: page_id.into(),
        })
        .await
        .map(|_| ())
    }

    pub async fn cycle_task(&self, item_id: &str) -> ApiResult<Item> {
        expect_output!(
            self.call(ApiOp::CycleTask {
                item_id: item_id.into()
            })
            .await,
            Item
        )
    }

    pub async fn upsert_item_property(
        &self,
        item_id: &str,
        key: &str,
        value_type: &str,
        value: &str,
    ) -> ApiResult<Item> {
        expect_output!(
            self.call(ApiOp::UpsertItemProperty {
                item_id: item_id.into(),
                key: key.into(),
                value_type: value_type.into(),
                value: value.into(),
            })
            .await,
            Item
        )
    }

    pub async fn delete_item_property(&self, item_id: &str, key: &str) -> ApiResult<Item> {
        expect_output!(
            self.call(ApiOp::DeleteItemProperty {
                item_id: item_id.into(),
                key: key.into(),
            })
            .await,
            Item
        )
    }

    /// Changes the outline of an atlas page and answers with the items of the page.
    pub async fn change_atlas(&self, atlas_id: &str, change: AtlasChange) -> ApiResult<Vec<Item>> {
        expect_output!(
            self.call(ApiOp::ChangeAtlas {
                atlas_id: atlas_id.into(),
                change,
            })
            .await,
            Items
        )
    }

    pub async fn search_combined(
        &self,
        query: &str,
        max_results: u32,
    ) -> ApiResult<Vec<SearchResult>> {
        expect_output!(
            self.call(ApiOp::SearchCombined {
                query: query.into(),
                max_results
            })
            .await,
            SearchResults
        )
    }

    pub async fn search_page_names(
        &self,
        query: &str,
        max_results: u32,
    ) -> ApiResult<Vec<SearchResult>> {
        expect_output!(
            self.call(ApiOp::SearchPageNames {
                query: query.into(),
                max_results
            })
            .await,
            SearchResults
        )
    }

    pub async fn get_recent_pages(
        &self,
        max_results: u32,
        page_type: Option<&str>,
    ) -> ApiResult<Vec<Page>> {
        expect_output!(
            self.call(ApiOp::GetRecentPages {
                max_results,
                page_type: page_type.map(str::to_string)
            })
            .await,
            Pages
        )
    }

    pub async fn update_page(
        &self,
        page_id: &str,
        title: Option<String>,
        dir_path: Option<String>,
        overwrite: bool,
    ) -> ApiResult<PageUpdate> {
        let updated = self
            .call(ApiOp::UpdatePage {
                page_id: page_id.into(),
                title,
                dir_path,
                overwrite,
            })
            .await?;
        page_update_of(updated)
    }

    pub async fn demote_page(&self, page_id: &str, overwrite: bool) -> ApiResult<PageUpdate> {
        let demoted = self
            .call(ApiOp::DemotePage {
                page_id: page_id.into(),
                overwrite,
            })
            .await?;
        page_update_of(demoted)
    }

    pub async fn upsert_page_property(
        &self,
        page_id: &str,
        key: &str,
        value_type: &str,
        value: &str,
    ) -> ApiResult<Page> {
        expect_output!(
            self.call(ApiOp::UpsertPageProperty {
                page_id: page_id.into(),
                key: key.into(),
                value_type: value_type.into(),
                value: value.into(),
            })
            .await,
            PageInfo
        )
    }

    pub async fn delete_page_property(&self, page_id: &str, key: &str) -> ApiResult<Page> {
        expect_output!(
            self.call(ApiOp::DeletePageProperty {
                page_id: page_id.into(),
                key: key.into(),
            })
            .await,
            PageInfo
        )
    }

    pub async fn get_config(&self) -> ApiResult<ServerConfig> {
        expect_output!(self.call(ApiOp::GetConfig).await, Config)
    }

    pub async fn get_property_definitions(&self) -> ApiResult<Vec<PropertyDefinition>> {
        expect_output!(
            self.call(ApiOp::GetPropertyDefinitions).await,
            PropertyDefinitions
        )
    }

    pub async fn delete_property_definition(
        &self,
        key: &str,
    ) -> ApiResult<Vec<PropertyDefinition>> {
        expect_output!(
            self.call(ApiOp::DeletePropertyDefinition { key: key.into() })
                .await,
            PropertyDefinitions
        )
    }

    pub async fn add_property_definition(
        &self,
        key: &str,
        value_type: &str,
    ) -> ApiResult<Vec<PropertyDefinition>> {
        expect_output!(
            self.call(ApiOp::AddPropertyDefinition {
                key: key.into(),
                value_type: value_type.into(),
            })
            .await,
            PropertyDefinitions
        )
    }

    pub async fn update_config(
        &self,
        markdown_dir: Option<String>,
        week_numbering: Option<String>,
        logical_outdent: Option<bool>,
        tags_dir: Option<String>,
        home_link: Option<String>,
    ) -> ApiResult<ServerConfig> {
        expect_output!(
            self.call(ApiOp::UpdateConfig {
                markdown_dir,
                week_numbering,
                logical_outdent,
                tags_dir,
                home_link
            })
            .await,
            Config
        )
    }

    pub async fn browse_folder(&self) -> ApiResult<FolderChoice> {
        expect_output!(self.call(ApiOp::BrowseFolder).await, Folder)
    }

    pub async fn transcluded_items(&self, item_ids: Vec<String>) -> ApiResult<Vec<String>> {
        expect_output!(
            self.call(ApiOp::TranscludedItems { item_ids }).await,
            ItemIds
        )
    }

    pub async fn export_tree_markdown(&self, item_ids: Vec<String>) -> ApiResult<TreeExport> {
        expect_output!(
            self.call(ApiOp::ExportTreeMarkdown { item_ids }).await,
            TreeExport
        )
    }

    pub async fn set_transclusion(&self, item_id: &str, source_item_id: &str) -> ApiResult<Item> {
        expect_output!(
            self.call(ApiOp::SetTransclusion {
                item_id: item_id.into(),
                source_item_id: source_item_id.into()
            })
            .await,
            Item
        )
    }

    pub async fn answer_confirm(&self, request_id: &str, ok: bool) -> ApiResult<()> {
        self.call(ApiOp::AnswerConfirm {
            request_id: request_id.into(),
            ok,
        })
        .await
        .map(|_| ())
    }

    pub async fn remove_transclusion(&self, item_id: &str) -> ApiResult<Item> {
        expect_output!(
            self.call(ApiOp::RemoveTransclusion {
                item_id: item_id.into()
            })
            .await,
            Item
        )
    }

    pub async fn get_child_pages(&self, parent_id: &str) -> ApiResult<Vec<Page>> {
        expect_output!(
            self.call(ApiOp::GetChildPages {
                parent_id: parent_id.into()
            })
            .await,
            Pages
        )
    }

    pub async fn get_backlinks(&self, page_id: &str) -> ApiResult<Vec<BacklinkEntry>> {
        expect_output!(
            self.call(ApiOp::GetBacklinks {
                page_id: page_id.into()
            })
            .await,
            Backlinks
        )
    }

    pub async fn upload_asset(&self, file: u64) -> ApiResult<Uploaded> {
        expect_output!(self.call(ApiOp::UploadAsset { file }).await, Uploaded)
    }

    pub async fn launch_url(&self, url: &str) -> ApiResult<bool> {
        expect_output!(
            self.call(ApiOp::LaunchUrl { url: url.into() }).await,
            Launched
        )
    }

    pub async fn open_export_folder(&self, page_id: &str) -> ApiResult<()> {
        self.call(ApiOp::OpenExportFolder {
            page_id: page_id.into(),
        })
        .await
        .map(|_| ())
    }

    // ---- Other shell requests ----

    /// Focuses an editor after the next render; resolves when it is focused.
    pub async fn focus_editor(&self, item_id: Option<String>, replay_keys: Vec<DeferredKey>) {
        let _ = self
            .ctx
            .request_from_shell(UiOp::FocusEditor {
                item_id,
                replay_keys,
            })
            .await;
    }

    pub async fn ui(&self, op: UiOp) -> UiOutput {
        self.ctx.request_from_shell(op).await
    }

    pub async fn confirm(&self, message: &str) -> bool {
        matches!(
            self.ui(UiOp::Confirm(message.into())).await,
            UiOutput::Bool(true)
        )
    }

    pub async fn confirm_destructive(&self, message: &str) -> bool {
        matches!(
            self.ui(UiOp::ConfirmDestructive(message.into())).await,
            UiOutput::Bool(true)
        )
    }

    pub async fn prompt(&self, message: &str) -> Option<String> {
        match self.ui(UiOp::Prompt(message.into())).await {
            UiOutput::Text(t) => t,
            _ => None,
        }
    }

    pub async fn read_clipboard_text(&self) -> String {
        match self.ui(UiOp::ReadClipboardText).await {
            UiOutput::Text(Some(t)) => t,
            _ => String::new(),
        }
    }

    pub async fn local_now(&self) -> (LocalDateTime, u64) {
        match self.ui(UiOp::LocalNow).await {
            UiOutput::Now { local, now_ms } => (local.into(), now_ms),
            _ => (LocalDateTime::default(), 0),
        }
    }

    pub async fn sleep(&self, id: u64, ms: u32) -> TimerOutput {
        self.ctx.request_from_shell(TimerOp::Start { id, ms }).await
    }

    pub fn notify(&self, op: UiOp) {
        self.ctx.notify_shell(op);
    }

    /// Shows a message in a dialog.
    pub fn alert(&self, message: impl Into<String>) {
        self.notify(UiOp::Alert(message.into()));
    }

    pub fn send_event(&self, event: Event) {
        self.ctx.send_event(event);
    }

    pub fn ctx(&self) -> &CommandContext<Effect, Event> {
        &self.ctx
    }
}
