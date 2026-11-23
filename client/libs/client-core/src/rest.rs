//! How each [`ApiOp`] travels over the journal server's REST API. This knows the requests and the
//! answers, and sends nothing: the shells do that, each with its own HTTP client.

use crate::effects::{ApiError, ApiOp, ApiOutput, ApiResult, AtlasChange};
use crate::model::{BacklinkEntry, Item, Page, PropertyDefinition, SearchResult};
pub use journal_api_types::routes;
use journal_api_types::routes::Route;
use journal_api_types::{
    AddPropertyDefinitionInput, AnswerConfirmInput, AtlasCalloutInput, AtlasGroupInput,
    AtlasLinkInput, AtlasTitleInput, BatchInput, CreateItemInput, DemotePageInput,
    GetOrCreateDocInput, ItemIdsInput, LaunchUrlInput, MergeItemsInput, MoveItemsInput,
    MoveToJournalInput, PasteInput, PropertyInput, ShiftLevelsInput, SplitInput, StashInput,
    TransclusionInput, UpdateConfigInput, UpdateItemInput, UpdatePageInput,
};
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// What `encodeURIComponent` leaves as it is.
const COMPONENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'!')
    .remove(b'~')
    .remove(b'*')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')');

/// The ID a shell names itself with: a new UUID for each start (ADR-015). The shell sends it
/// with every request and gives it to the core with `Event::Start`.
///
/// Only [`ClientId::new`] makes one, so a shell cannot name itself with a text of its own.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct ClientId(String);

impl ClientId {
    /// The header that carries the ID. The server refuses a change without it.
    pub const HEADER: &str = journal_api_types::CLIENT_ID_HEADER;

    #[allow(clippy::new_without_default)] // each call makes another ID, which a default is not
    pub fn new() -> ClientId {
        ClientId(uuid::Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One request to the journal server. `path` starts at `/api`.
#[derive(Debug, Clone, PartialEq)]
pub struct HttpRequest {
    pub method: &'static str,
    pub path: String,
    pub body: Option<Value>,
}

/// The answer of the server, whatever its status.
#[derive(Debug, Clone, PartialEq)]
pub struct HttpReply {
    pub status: u16,
    pub body: String,
}

/// The request of an operation and how to read its answer. They are made together, so a new
/// operation cannot have one without the other.
pub struct RestCall {
    pub request: HttpRequest,
    read: Reader,
}

type Reader = fn(&HttpReply) -> ApiResult<ApiOutput>;

impl RestCall {
    pub fn decode(&self, reply: &HttpReply) -> ApiResult<ApiOutput> {
        (self.read)(reply)
    }
}

fn enc(s: &str) -> String {
    utf8_percent_encode(s, COMPONENT).to_string()
}

/// The server's error message, or the status when it gave none.
fn error_of(reply: &HttpReply) -> ApiError {
    let message = serde_json::from_str::<Value>(&reply.body)
        .ok()
        .and_then(|v| v.get("error")?.as_str().map(str::to_string));
    ApiError(message.unwrap_or_else(|| format!("HTTP {}", reply.status)))
}

fn ensure_ok(reply: &HttpReply) -> ApiResult<()> {
    if (200..300).contains(&reply.status) {
        Ok(())
    } else {
        Err(error_of(reply))
    }
}

fn parsed<T: DeserializeOwned>(reply: &HttpReply) -> ApiResult<T> {
    ensure_ok(reply)?;
    serde_json::from_str(&reply.body).map_err(|e| ApiError(e.to_string()))
}

/// The body of a request, written with the type the journal reads it with
/// (`journal-api-types`). A field that is not set is not written.
fn body(input: impl Serialize) -> Option<Value> {
    Some(serde_json::to_value(input).expect("a request is plain data"))
}

/// The route of a change of an atlas, the item it names in its path and its body.
fn atlas_request(change: AtlasChange) -> (Route, Option<String>, Option<Value>) {
    match change {
        AtlasChange::AddLink { target, bounds } => (
            routes::ADD_ATLAS_LINK,
            None,
            body(AtlasLinkInput { target, bounds }),
        ),
        AtlasChange::SetLink { item_id, target } => (
            routes::SET_ATLAS_LINK,
            Some(item_id),
            body(AtlasLinkInput {
                target,
                ..Default::default()
            }),
        ),
        AtlasChange::Group { cards, target } => (
            routes::GROUP_ATLAS_CARDS,
            None,
            body(AtlasGroupInput { cards, target }),
        ),
        AtlasChange::Ungroup { cards } => (
            routes::UNGROUP_ATLAS_CARDS,
            None,
            body(AtlasGroupInput {
                cards,
                ..Default::default()
            }),
        ),
        AtlasChange::SetGroupTitle {
            item_id,
            side,
            title,
        } => (
            routes::SET_ATLAS_GROUP_TITLE,
            Some(item_id),
            body(AtlasTitleInput { side, title }),
        ),
        AtlasChange::AddCallout { bounds, side, text } => (
            routes::ADD_ATLAS_CALLOUT,
            None,
            body(AtlasCalloutInput { bounds, side, text }),
        ),
        AtlasChange::SetCallout {
            item_id,
            side,
            text,
        } => (
            routes::SET_ATLAS_CALLOUT,
            Some(item_id),
            body(AtlasCalloutInput {
                side,
                text,
                ..Default::default()
            }),
        ),
        AtlasChange::Remove { item_id } => (routes::REMOVE_ATLAS_ITEM, Some(item_id), None),
    }
}

#[derive(Deserialize)]
struct ItemEnvelope {
    item: Item,
}
#[derive(Deserialize)]
struct ItemsEnvelope {
    items: Vec<Item>,
}
#[derive(Deserialize)]
struct CreatedEnvelope {
    created: Vec<Item>,
}
#[derive(Deserialize)]
struct FoundPageEnvelope {
    page: Option<Page>,
}
#[derive(Deserialize)]
struct PageEnvelope {
    page: Page,
}
#[derive(Deserialize)]
struct PagesEnvelope {
    pages: Vec<Page>,
}
#[derive(Deserialize)]
struct BacklinksEnvelope {
    entries: Vec<BacklinkEntry>,
}
#[derive(Deserialize)]
struct ResultsEnvelope {
    results: Vec<SearchResult>,
}
#[derive(Deserialize)]
struct DefinitionsEnvelope {
    definitions: Vec<PropertyDefinition>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ItemIdsEnvelope {
    item_ids: Vec<String>,
}
#[derive(Deserialize)]
struct LaunchEnvelope {
    success: bool,
}

fn done(reply: &HttpReply) -> ApiResult<ApiOutput> {
    ensure_ok(reply)?;
    Ok(ApiOutput::Done)
}

fn item(reply: &HttpReply) -> ApiResult<ApiOutput> {
    parsed::<ItemEnvelope>(reply).map(|e| ApiOutput::Item(e.item))
}

fn items(reply: &HttpReply) -> ApiResult<ApiOutput> {
    parsed::<ItemsEnvelope>(reply).map(|e| ApiOutput::Items(e.items))
}

fn page(reply: &HttpReply) -> ApiResult<ApiOutput> {
    parsed(reply).map(ApiOutput::Page)
}

fn found_page(reply: &HttpReply) -> ApiResult<ApiOutput> {
    parsed::<FoundPageEnvelope>(reply).map(|e| ApiOutput::FoundPage(e.page))
}

fn page_info(reply: &HttpReply) -> ApiResult<ApiOutput> {
    parsed::<PageEnvelope>(reply).map(|e| ApiOutput::PageInfo(e.page))
}

fn pages(reply: &HttpReply) -> ApiResult<ApiOutput> {
    parsed::<PagesEnvelope>(reply).map(|e| ApiOutput::Pages(e.pages))
}

fn doc_page(reply: &HttpReply) -> ApiResult<ApiOutput> {
    parsed(reply).map(ApiOutput::DocPage)
}

fn moved_to_journal(reply: &HttpReply) -> ApiResult<ApiOutput> {
    parsed(reply).map(ApiOutput::MovedToJournal)
}

fn journal_before(reply: &HttpReply) -> ApiResult<ApiOutput> {
    parsed(reply).map(ApiOutput::JournalBefore)
}

fn search_results(reply: &HttpReply) -> ApiResult<ApiOutput> {
    parsed::<ResultsEnvelope>(reply).map(|e| ApiOutput::SearchResults(e.results))
}

fn definitions(reply: &HttpReply) -> ApiResult<ApiOutput> {
    parsed::<DefinitionsEnvelope>(reply).map(|e| ApiOutput::PropertyDefinitions(e.definitions))
}

fn config(reply: &HttpReply) -> ApiResult<ApiOutput> {
    parsed(reply).map(ApiOutput::Config)
}

/// 409: another page has the name, which the core asks the user about.
fn page_update(reply: &HttpReply) -> ApiResult<ApiOutput> {
    if reply.status == 409 {
        let ApiError(message) = error_of(reply);
        return Ok(ApiOutput::PageNameTaken(message));
    }
    page_info(reply)
}

/// A call of a route of the server (`journal_api_types::routes`). `args` are the changing
/// parts of its path, in the order the pattern names them. They are encoded here.
fn call(route: Route, args: &[&str], body: Option<Value>, read: Reader) -> RestCall {
    let args: Vec<String> = args.iter().map(|arg| enc(arg)).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    RestCall {
        request: HttpRequest {
            method: route.method.as_str(),
            path: route.path(&args),
            body,
        },
        read,
    }
}

impl RestCall {
    /// The same call with a query after its path. The values are encoded here.
    fn asking(mut self, query: &[(&str, &str)]) -> RestCall {
        let pairs: Vec<String> = query
            .iter()
            .map(|(name, value)| format!("{name}={}", enc(value)))
            .collect();
        self.request.path = format!("{}?{}", self.request.path, pairs.join("&"));
        self
    }
}

/// The REST call of an operation. None when only the shell can do it: an upload needs a file that
/// the shell holds.
pub fn rest_call(op: ApiOp) -> Option<RestCall> {
    Some(match op {
        ApiOp::GetPageById { page_id } => call(routes::GET_PAGE, &[&page_id], None, page),
        ApiOp::GetMostRecentJournalBefore { before } => {
            call(routes::JOURNAL_BEFORE, &[], None, journal_before).asking(&[("date", &before)])
        }
        ApiOp::GetMostRecentWeeklyJournalBefore { before } => {
            call(routes::WEEKLY_JOURNAL_BEFORE, &[], None, journal_before)
                .asking(&[("week", &before)])
        }
        ApiOp::GetOrCreateJournal { file_name } => {
            call(routes::GET_OR_CREATE_JOURNAL, &[&file_name], None, page)
        }
        ApiOp::MoveItemsToJournal {
            file_name,
            item_ids,
            position,
            body,
        } => call(
            routes::MOVE_ITEMS_TO_JOURNAL,
            &[&file_name],
            self::body(MoveToJournalInput {
                item_ids,
                position: position.clone(),
                body,
            }),
            moved_to_journal,
        ),
        ApiOp::FindPage {
            journal,
            title,
            dir_path,
        } => {
            let query = match &journal {
                Some(journal) => vec![("journal", journal.as_str())],
                None => vec![("title", title.as_str()), ("dirPath", dir_path.as_str())],
            };
            call(routes::FIND_PAGE, &[], None, found_page).asking(&query)
        }
        ApiOp::GetOrCreateDoc { title, dir_path } => call(
            routes::GET_OR_CREATE_DOC,
            &[],
            body(GetOrCreateDocInput {
                title: Some(title),
                dir_path: Some(dir_path),
            }),
            doc_page,
        ),
        ApiOp::CreateItem {
            page_id,
            position,
            level,
            body,
            id,
        } => call(
            routes::CREATE_ITEM,
            &[],
            self::body(CreateItemInput {
                page_id: Some(page_id),
                position: Some(position.clone()),
                level,
                body: Some(body),
                id,
            }),
            item,
        ),
        ApiOp::UpdateItem {
            item_id,
            body,
            level,
            collapsed,
        } => call(
            routes::UPDATE_ITEM,
            &[&item_id],
            self::body(UpdateItemInput {
                body,
                level,
                collapsed,
            }),
            item,
        ),
        ApiOp::ShiftItemLevels { item_ids, delta } => call(
            routes::SHIFT_ITEM_LEVELS,
            &[],
            body(ShiftLevelsInput {
                item_ids: Some(item_ids),
                delta,
            }),
            items,
        ),
        ApiOp::MoveItems {
            item_ids,
            target_page_id,
            position,
            level_delta,
            body,
        } => call(
            routes::MOVE_ITEMS,
            &[],
            self::body(MoveItemsInput {
                item_ids: Some(item_ids),
                target_page_id: Some(target_page_id),
                position: Some(position.clone()),
                level_delta,
                body,
            }),
            items,
        ),
        ApiOp::DeletePage { page_id } => call(routes::DELETE_PAGE, &[&page_id], None, done),
        ApiOp::DeleteItems { item_ids } => call(
            routes::DELETE_ITEMS,
            &[],
            body(ItemIdsInput {
                item_ids: Some(item_ids),
            }),
            |reply| parsed::<CreatedEnvelope>(reply).map(|e| ApiOutput::Items(e.created)),
        ),
        ApiOp::MergeItems {
            item_id,
            into_id,
            body,
            into_body,
        } => call(
            routes::MERGE_ITEMS,
            &[],
            self::body(MergeItemsInput {
                item_id: Some(item_id),
                into_id: Some(into_id),
                body,
                into_body,
            }),
            item,
        ),
        ApiOp::PasteText {
            item_id,
            body,
            cursor,
            text,
            mode,
        } => call(
            routes::PASTE_TEXT,
            &[&item_id],
            self::body(PasteInput {
                body: Some(body),
                cursor,
                text: Some(text),
                mode: Some(mode.as_api().to_string()),
            }),
            |reply| parsed(reply).map(ApiOutput::Pasted),
        ),
        ApiOp::SplitItem {
            item_id,
            root_id,
            body,
            cursor,
        } => call(
            routes::SPLIT_ITEM,
            &[&item_id],
            self::body(SplitInput {
                body: Some(body),
                cursor,
                root_id,
            }),
            |reply| parsed(reply).map(|split| ApiOutput::Split(Box::new(split))),
        ),
        ApiOp::StashItem { item_id, body } => call(
            routes::STASH_ITEM,
            &[&item_id],
            self::body(StashInput { body }),
            |reply| parsed(reply).map(|stashed| ApiOutput::Stashed(Box::new(stashed))),
        ),
        ApiOp::ApplyBatch { ops } => call(
            routes::APPLY_BATCH,
            &[],
            self::body(BatchInput { ops }),
            |reply| parsed(reply).map(|batched| ApiOutput::Batched(Box::new(batched))),
        ),
        ApiOp::CycleTask { item_id } => call(routes::CYCLE_TASK, &[&item_id], None, item),
        ApiOp::UpsertItemProperty {
            item_id,
            key,
            value_type,
            value,
        } => call(
            routes::UPSERT_ITEM_PROPERTY,
            &[&item_id, &key],
            body(PropertyInput {
                value_type: Some(value_type),
                value: Some(value),
            }),
            item,
        ),
        ApiOp::DeleteItemProperty { item_id, key } => {
            call(routes::DELETE_ITEM_PROPERTY, &[&item_id, &key], None, item)
        }
        ApiOp::ChangeAtlas { atlas_id, change } => {
            let (route, item_id, body) = atlas_request(change);
            let args: Vec<&str> = [Some(&atlas_id), item_id.as_ref()]
                .into_iter()
                .flatten()
                .map(String::as_str)
                .collect();
            call(route, &args, body, items)
        }
        ApiOp::SearchCombined { query, max_results } => {
            call(routes::SEARCH_COMBINED, &[], None, search_results)
                .asking(&[("q", &query), ("maxResults", &max_results.to_string())])
        }
        ApiOp::SearchPageNames { query, max_results } => {
            call(routes::SEARCH_PAGE_NAMES, &[], None, search_results)
                .asking(&[("q", &query), ("maxResults", &max_results.to_string())])
        }
        ApiOp::GetRecentPages {
            max_results,
            page_type,
        } => {
            let max_results = max_results.to_string();
            let mut query = vec![("maxResults", max_results.as_str())];
            if let Some(page_type) = &page_type {
                query.push(("type", page_type));
            }
            call(routes::RECENT_PAGES, &[], None, pages).asking(&query)
        }
        ApiOp::UpdatePage {
            page_id,
            title,
            dir_path,
            overwrite,
        } => call(
            routes::UPDATE_PAGE,
            &[&page_id],
            body(UpdatePageInput {
                title,
                dir_path,
                overwrite,
            }),
            page_update,
        ),
        ApiOp::DemotePage { page_id, overwrite } => call(
            routes::DEMOTE_PAGE,
            &[&page_id],
            body(DemotePageInput { overwrite }),
            page_update,
        ),
        ApiOp::UpsertPageProperty {
            page_id,
            key,
            value_type,
            value,
        } => call(
            routes::UPSERT_PAGE_PROPERTY,
            &[&page_id, &key],
            body(PropertyInput {
                value_type: Some(value_type),
                value: Some(value),
            }),
            page_info,
        ),
        ApiOp::DeletePageProperty { page_id, key } => call(
            routes::DELETE_PAGE_PROPERTY,
            &[&page_id, &key],
            None,
            page_info,
        ),
        ApiOp::GetConfig => call(routes::GET_CONFIG, &[], None, config),
        ApiOp::GetPropertyDefinitions => call(routes::PROPERTY_DEFINITIONS, &[], None, definitions),
        ApiOp::AddPropertyDefinition { key, value_type } => call(
            routes::ADD_PROPERTY_DEFINITION,
            &[],
            body(AddPropertyDefinitionInput {
                key: Some(key),
                value_type: Some(value_type),
                page: None,
            }),
            definitions,
        ),
        ApiOp::DeletePropertyDefinition { key } => call(
            routes::DELETE_PROPERTY_DEFINITION,
            &[&key],
            None,
            definitions,
        ),
        ApiOp::UpdateConfig {
            markdown_dir,
            week_numbering,
            logical_outdent,
            tags_dir,
            home_link,
        } => call(
            routes::UPDATE_CONFIG,
            &[],
            body(UpdateConfigInput {
                markdown_dir,
                week_numbering,
                logical_outdent,
                tags_dir,
                home_link,
            }),
            config,
        ),
        ApiOp::BrowseFolder => call(routes::BROWSE_FOLDER, &[], None, |reply| {
            parsed(reply).map(ApiOutput::Folder)
        }),
        ApiOp::ExportTreeMarkdown { item_ids } => call(
            routes::EXPORT_TREE_MARKDOWN,
            &[],
            body(ItemIdsInput {
                item_ids: Some(item_ids),
            }),
            |reply| parsed(reply).map(ApiOutput::TreeExport),
        ),
        ApiOp::TranscludedItems { item_ids } => call(
            routes::TRANSCLUDED_ITEMS,
            &[],
            body(ItemIdsInput {
                item_ids: Some(item_ids),
            }),
            |reply| parsed::<ItemIdsEnvelope>(reply).map(|e| ApiOutput::ItemIds(e.item_ids)),
        ),
        ApiOp::SetTransclusion {
            item_id,
            source_item_id,
        } => call(
            routes::SET_TRANSCLUSION,
            &[&item_id],
            body(TransclusionInput {
                source_item_id: Some(source_item_id),
            }),
            item,
        ),
        ApiOp::AnswerConfirm { request_id, ok } => call(
            routes::ANSWER_CONFIRM,
            &[&request_id],
            body(AnswerConfirmInput { ok }),
            done,
        ),
        ApiOp::RemoveTransclusion { item_id } => {
            call(routes::REMOVE_TRANSCLUSION, &[&item_id], None, item)
        }
        ApiOp::GetChildPages { parent_id } => call(routes::CHILD_PAGES, &[&parent_id], None, pages),
        ApiOp::GetBacklinks { page_id } => call(routes::BACKLINKS, &[&page_id], None, |reply| {
            parsed::<BacklinksEnvelope>(reply).map(|e| ApiOutput::Backlinks(e.entries))
        }),
        ApiOp::UploadAsset { .. } => return None,
        ApiOp::LaunchUrl { url } => call(
            routes::LAUNCH_URL,
            &[],
            body(LaunchUrlInput { url: Some(url) }),
            |reply| parsed::<LaunchEnvelope>(reply).map(|e| ApiOutput::Launched(e.success)),
        ),
        ApiOp::OpenExportFolder { page_id } => {
            call(routes::OPEN_EXPORT_FOLDER, &[&page_id], None, done)
        }
    })
}
