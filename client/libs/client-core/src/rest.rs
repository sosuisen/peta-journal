//! How each [`ApiOp`] travels over the journal server's REST API. This knows the requests and the
//! answers, and sends nothing: the shells do that, each with its own HTTP client.

use crate::effects::{ApiError, ApiOp, ApiOutput, ApiResult, AtlasChange};
use crate::model::{BacklinkEntry, Item, Page, PropertyDefinition, SearchResult};
use journal_api_types::{
    AddPropertyDefinitionInput, AnswerConfirmInput, AtlasCalloutInput, AtlasGroupInput,
    AtlasLinkInput, AtlasTitleInput, CreateItemInput, DemotePageInput, GetOrCreateDocInput,
    ItemIdsInput, LaunchUrlInput, MergeItemsInput, MoveItemsInput, MoveToJournalInput, PasteInput,
    PropertyInput, ShiftLevelsInput, SplitInput, StashInput, TransclusionInput, UpdateConfigInput,
    UpdateItemInput, UpdatePageInput,
};
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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

/// The method, the path under `/api/pages/{id}/atlas/` and the body of a change of an atlas.
fn atlas_request(change: AtlasChange) -> (&'static str, String, Option<Value>) {
    match change {
        AtlasChange::AddLink { target, bounds } => (
            "POST",
            "links".into(),
            body(AtlasLinkInput { target, bounds }),
        ),
        AtlasChange::SetLink { item_id, target } => (
            "PUT",
            format!("links/{item_id}"),
            body(AtlasLinkInput {
                target,
                ..Default::default()
            }),
        ),
        AtlasChange::Group { cards, target } => (
            "POST",
            "group".into(),
            body(AtlasGroupInput { cards, target }),
        ),
        AtlasChange::Ungroup { cards } => (
            "POST",
            "ungroup".into(),
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
            "PUT",
            format!("groups/{item_id}"),
            body(AtlasTitleInput { side, title }),
        ),
        AtlasChange::AddCallout { bounds, side, text } => (
            "POST",
            "callouts".into(),
            body(AtlasCalloutInput { bounds, side, text }),
        ),
        AtlasChange::SetCallout {
            item_id,
            side,
            text,
        } => (
            "PUT",
            format!("callouts/{item_id}"),
            body(AtlasCalloutInput {
                side,
                text,
                ..Default::default()
            }),
        ),
        AtlasChange::Remove { item_id } => ("DELETE", format!("items/{item_id}"), None),
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

fn page_info(reply: &HttpReply) -> ApiResult<ApiOutput> {
    parsed::<PageEnvelope>(reply).map(|e| ApiOutput::PageInfo(e.page))
}

fn pages(reply: &HttpReply) -> ApiResult<ApiOutput> {
    parsed::<PagesEnvelope>(reply).map(|e| ApiOutput::Pages(e.pages))
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

fn call(
    method: &'static str,
    path: impl Into<String>,
    body: Option<Value>,
    read: Reader,
) -> RestCall {
    RestCall {
        request: HttpRequest {
            method,
            path: path.into(),
            body,
        },
        read,
    }
}

/// The REST call of an operation. None when only the shell can do it: an upload needs a file that
/// the shell holds.
pub fn rest_call(op: ApiOp) -> Option<RestCall> {
    Some(match op {
        ApiOp::GetPageById { page_id } => call("GET", format!("/api/pages/{page_id}"), None, page),
        ApiOp::GetMostRecentJournalBefore { before } => call(
            "GET",
            format!("/api/journals/before?date={}", enc(&before)),
            None,
            journal_before,
        ),
        ApiOp::GetMostRecentWeeklyJournalBefore { before } => call(
            "GET",
            format!("/api/journals/weekly/before?week={}", enc(&before)),
            None,
            journal_before,
        ),
        ApiOp::GetOrCreateJournal { file_name } => call(
            "POST",
            format!("/api/journals/{}", enc(&file_name)),
            None,
            page,
        ),
        ApiOp::MoveItemsToJournal {
            file_name,
            item_ids,
            position,
            body,
        } => call(
            "POST",
            format!("/api/journals/{}/move", enc(&file_name)),
            self::body(MoveToJournalInput {
                item_ids,
                position: position.to_api(),
                body,
            }),
            page,
        ),
        ApiOp::GetOrCreateDoc { title, dir_path } => call(
            "POST",
            "/api/pages/doc",
            body(GetOrCreateDocInput {
                title: Some(title),
                dir_path: Some(dir_path),
            }),
            page,
        ),
        ApiOp::CreateItem {
            page_id,
            position,
            level,
            body,
            id,
        } => call(
            "POST",
            "/api/items",
            self::body(CreateItemInput {
                page_id: Some(page_id),
                position: Some(position.to_api()),
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
            "PUT",
            format!("/api/items/{item_id}"),
            self::body(UpdateItemInput {
                body,
                level,
                collapsed,
            }),
            item,
        ),
        ApiOp::ShiftItemLevels { item_ids, delta } => call(
            "POST",
            "/api/items/shift-level",
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
            "POST",
            "/api/items/move",
            self::body(MoveItemsInput {
                item_ids: Some(item_ids),
                target_page_id: Some(target_page_id),
                position: Some(position.to_api()),
                level_delta,
                body,
            }),
            items,
        ),
        ApiOp::DeletePage { page_id } => {
            call("DELETE", format!("/api/pages/{page_id}"), None, done)
        }
        ApiOp::DeleteItems { item_ids } => call(
            "POST",
            "/api/items/delete",
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
            "POST",
            "/api/items/merge",
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
            "POST",
            format!("/api/items/{item_id}/paste"),
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
            "POST",
            format!("/api/items/{item_id}/split"),
            self::body(SplitInput {
                body: Some(body),
                cursor,
                root_id,
            }),
            |reply| parsed(reply).map(|split| ApiOutput::Split(Box::new(split))),
        ),
        ApiOp::StashItem { item_id, body } => call(
            "POST",
            format!("/api/items/{item_id}/stash"),
            self::body(StashInput { body }),
            |reply| parsed(reply).map(|stashed| ApiOutput::Stashed(Box::new(stashed))),
        ),
        ApiOp::ApplyBatch { ops } => call(
            "POST",
            "/api/items/batch",
            Some(json!({ "ops": ops })),
            |reply| parsed(reply).map(|batched| ApiOutput::Batched(Box::new(batched))),
        ),
        ApiOp::CycleTask { item_id } => call(
            "POST",
            format!("/api/items/{item_id}/cycle-task"),
            None,
            item,
        ),
        ApiOp::UpsertItemProperty {
            item_id,
            key,
            value_type,
            value,
        } => call(
            "PUT",
            format!("/api/items/{item_id}/properties/{}", enc(&key)),
            body(PropertyInput {
                value_type: Some(value_type),
                value: Some(value),
            }),
            item,
        ),
        ApiOp::DeleteItemProperty { item_id, key } => call(
            "DELETE",
            format!("/api/items/{item_id}/properties/{}", enc(&key)),
            None,
            item,
        ),
        ApiOp::ChangeAtlas { atlas_id, change } => {
            let (method, path, body) = atlas_request(change);
            call(
                method,
                format!("/api/pages/{atlas_id}/atlas/{path}"),
                body,
                items,
            )
        }
        ApiOp::SearchCombined { query, max_results } => call(
            "GET",
            format!(
                "/api/search/combined?q={}&maxResults={max_results}",
                enc(&query)
            ),
            None,
            search_results,
        ),
        ApiOp::SearchPageNames { query, max_results } => call(
            "GET",
            format!(
                "/api/search/pages?q={}&maxResults={max_results}",
                enc(&query)
            ),
            None,
            search_results,
        ),
        ApiOp::GetRecentPages {
            max_results,
            page_type,
        } => {
            let mut path = format!("/api/pages/recent?maxResults={max_results}");
            if let Some(t) = page_type {
                path.push_str(&format!("&type={}", enc(&t)));
            }
            call("GET", path, None, pages)
        }
        ApiOp::UpdatePage {
            page_id,
            title,
            dir_path,
            overwrite,
        } => call(
            "PUT",
            format!("/api/pages/{page_id}"),
            body(UpdatePageInput {
                title,
                dir_path,
                overwrite,
            }),
            page_update,
        ),
        ApiOp::DemotePage { page_id, overwrite } => call(
            "POST",
            format!("/api/pages/{page_id}/demote"),
            body(DemotePageInput { overwrite }),
            page_update,
        ),
        ApiOp::UpsertPageProperty {
            page_id,
            key,
            value_type,
            value,
        } => call(
            "PUT",
            format!("/api/pages/{page_id}/properties/{}", enc(&key)),
            body(PropertyInput {
                value_type: Some(value_type),
                value: Some(value),
            }),
            page_info,
        ),
        ApiOp::DeletePageProperty { page_id, key } => call(
            "DELETE",
            format!("/api/pages/{page_id}/properties/{}", enc(&key)),
            None,
            page_info,
        ),
        ApiOp::GetConfig => call("GET", "/api/config", None, config),
        ApiOp::GetPropertyDefinitions => {
            call("GET", "/api/config/property-definitions", None, definitions)
        }
        ApiOp::AddPropertyDefinition { key, value_type } => call(
            "POST",
            "/api/config/property-definitions",
            body(AddPropertyDefinitionInput {
                key: Some(key),
                value_type: Some(value_type),
                page: None,
            }),
            definitions,
        ),
        ApiOp::DeletePropertyDefinition { key } => call(
            "DELETE",
            format!("/api/config/property-definitions/{}", enc(&key)),
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
            "PUT",
            "/api/config",
            body(UpdateConfigInput {
                markdown_dir,
                week_numbering,
                logical_outdent,
                tags_dir,
                home_link,
            }),
            config,
        ),
        ApiOp::BrowseFolder => call("POST", "/api/config/browse-folder", None, |reply| {
            parsed(reply).map(ApiOutput::Folder)
        }),
        ApiOp::ExportTreeMarkdown { item_ids } => call(
            "POST",
            "/api/items/export",
            body(ItemIdsInput {
                item_ids: Some(item_ids),
            }),
            |reply| parsed(reply).map(ApiOutput::TreeExport),
        ),
        ApiOp::TranscludedItems { item_ids } => call(
            "POST",
            "/api/items/transcluded",
            body(ItemIdsInput {
                item_ids: Some(item_ids),
            }),
            |reply| parsed::<ItemIdsEnvelope>(reply).map(|e| ApiOutput::ItemIds(e.item_ids)),
        ),
        ApiOp::SetTransclusion {
            item_id,
            source_item_id,
        } => call(
            "POST",
            format!("/api/items/{item_id}/transclude"),
            body(TransclusionInput {
                source_item_id: Some(source_item_id),
            }),
            item,
        ),
        ApiOp::AnswerConfirm { request_id, ok } => call(
            "POST",
            format!("/api/confirm/{}", enc(&request_id)),
            body(AnswerConfirmInput { ok }),
            done,
        ),
        ApiOp::RemoveTransclusion { item_id } => call(
            "DELETE",
            format!("/api/items/{item_id}/transclude"),
            None,
            item,
        ),
        ApiOp::GetChildPages { parent_id } => call(
            "GET",
            format!("/api/pages/{parent_id}/children"),
            None,
            pages,
        ),
        ApiOp::GetBacklinks { page_id } => call(
            "GET",
            format!("/api/pages/{page_id}/backlinks"),
            None,
            |reply| parsed::<BacklinksEnvelope>(reply).map(|e| ApiOutput::Backlinks(e.entries)),
        ),
        ApiOp::UploadAsset { .. } => return None,
        ApiOp::LaunchUrl { url } => call(
            "POST",
            "/api/launcher/open",
            body(LaunchUrlInput { url: Some(url) }),
            |reply| parsed::<LaunchEnvelope>(reply).map(|e| ApiOutput::Launched(e.success)),
        ),
        ApiOp::OpenExportFolder { page_id } => call(
            "POST",
            format!("/api/pages/{page_id}/open-folder"),
            None,
            done,
        ),
    })
}
