//! What the core asks the shell to do. Every variant of [`Effect`] carries a request the
//! shell performs and resolves; the core never touches the network, timers or the DOM.

use crate::model::{
    BacklinkEntry, DeferredKey, Item, ItemPosition, JournalBefore, Page, PageWithItems,
    ParsedFlatItem, PropertyDefinition, SearchResult, ServerConfig, TreeExport,
};
use crate::util::date::LocalDateTime;
use crux_core::capability::Operation;
use crux_core::command::CommandContext;
use crux_core::render::RenderOperation;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    GetOrCreateDoc {
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
    },
    DeletePage {
        page_id: String,
    },
    DeleteItem {
        item_id: String,
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
    ImportTreeMarkdown {
        tree_markdown: String,
    },
    ParseFlatMarkdown {
        markdown: String,
    },
    SetTransclusion {
        item_id: String,
        source_item_id: String,
    },
    RemoveTransclusion {
        item_id: String,
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
    GetFunctions,
    /// `POST /api/launcher/open` with a custom URL scheme.
    LaunchUrl {
        url: String,
    },
}

/// A registered extension function (`GET /api/functions`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FolderChoice {
    pub selected: bool,
    #[serde(default)]
    pub path: String,
}

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
    FlatMarkdown(String),
    ParsedItems(Vec<ParsedFlatItem>),
    FileName(String),
    Functions(Vec<FunctionInfo>),
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

/// The server's error message, or a transport error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiError(pub String);

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
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
    CopyText(String),
    /// Copies tree markdown as text and, rendered to HTML, as rich text. The shell may have
    /// begun the clipboard write on the user's key already and only fills it in here.
    CopyMarkdown {
        markdown: String,
    },
    /// There is nothing to copy after all: the shell gives up a clipboard write it began.
    CopyFailed,
    /// Hosts the extension functions in hidden iframes.
    LoadExtensions {
        functions: Vec<FunctionInfo>,
    },
    /// Resolved with `UiOutput::Text`.
    ReadClipboardText,
    Alert(String),
    /// Resolved with `UiOutput::Bool`.
    Confirm(String),
    /// Resolved with `UiOutput::Text`; None when dismissed.
    Prompt(String),
    OpenWindow {
        url: String,
        new_window: bool,
    },
    /// Tells the extension iframes that a page was created.
    ExtensionPageCreate {
        page_id: String,
        file_name: String,
        dir_name: String,
        page_type: String,
    },
    /// Tells the extension iframes that tags were added to the tags property of a page.
    /// `tags` holds the added ones only.
    ExtensionPageTagAdd {
        page_id: String,
        file_name: String,
        dir_name: String,
        page_type: String,
        tags: Vec<String>,
    },
    /// Answers an extension's message. `source` is the shell's handle of the window.
    ExtensionReply {
        source: u64,
        message: Value,
    },
    /// Resolved with `UiOutput::Now`.
    LocalNow,
}

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LocalDateTimeDto {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
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

    pub async fn get_or_create_journal(&self, file_name: &str) -> ApiResult<PageWithItems> {
        expect_output!(
            self.call(ApiOp::GetOrCreateJournal {
                file_name: file_name.into()
            })
            .await,
            Page
        )
    }

    pub async fn get_or_create_doc(&self, title: &str, dir_path: &str) -> ApiResult<PageWithItems> {
        expect_output!(
            self.call(ApiOp::GetOrCreateDoc {
                title: title.into(),
                dir_path: dir_path.into()
            })
            .await,
            Page
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
                level_delta
            })
            .await,
            Items
        )
    }

    pub async fn delete_item(&self, item_id: &str) -> ApiResult<()> {
        self.call(ApiOp::DeleteItem {
            item_id: item_id.into(),
        })
        .await
        .map(|_| ())
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
        match self
            .call(ApiOp::UpdatePage {
                page_id: page_id.into(),
                title,
                dir_path,
                overwrite,
            })
            .await?
        {
            ApiOutput::PageInfo(page) => Ok(PageUpdate::Updated(page)),
            ApiOutput::PageNameTaken(message) => Ok(PageUpdate::NameTaken(message)),
            other => Err(ApiError(format!("unexpected API answer: {other:?}"))),
        }
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

    pub async fn export_tree_markdown(&self, item_ids: Vec<String>) -> ApiResult<TreeExport> {
        expect_output!(
            self.call(ApiOp::ExportTreeMarkdown { item_ids }).await,
            TreeExport
        )
    }

    pub async fn import_tree_markdown(&self, tree_markdown: &str) -> ApiResult<String> {
        expect_output!(
            self.call(ApiOp::ImportTreeMarkdown {
                tree_markdown: tree_markdown.into()
            })
            .await,
            FlatMarkdown
        )
    }

    pub async fn parse_flat_markdown(&self, markdown: &str) -> ApiResult<Vec<ParsedFlatItem>> {
        expect_output!(
            self.call(ApiOp::ParseFlatMarkdown {
                markdown: markdown.into()
            })
            .await,
            ParsedItems
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

    pub async fn upload_asset(&self, file: u64) -> ApiResult<String> {
        expect_output!(self.call(ApiOp::UploadAsset { file }).await, FileName)
    }

    pub async fn get_functions(&self) -> ApiResult<Vec<FunctionInfo>> {
        expect_output!(self.call(ApiOp::GetFunctions).await, Functions)
    }

    pub async fn launch_url(&self, url: &str) -> ApiResult<bool> {
        expect_output!(
            self.call(ApiOp::LaunchUrl { url: url.into() }).await,
            Launched
        )
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

    pub fn send_event(&self, event: Event) {
        self.ctx.send_event(event);
    }

    pub fn ctx(&self) -> &CommandContext<Effect, Event> {
        &self.ctx
    }
}
