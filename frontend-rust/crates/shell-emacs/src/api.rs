//! Performs [`ApiOp`]s against the journal server. The transport is behind [`Http`], so the
//! requests and the answers can be tested without a server.

use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use peta_core::effects::{ApiError, ApiOp, ApiOutput, ApiResult, AtlasChange, FolderChoice};
use peta_core::model::*;
use serde::de::DeserializeOwned;
use serde::Deserialize;
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

pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

/// One HTTP exchange with the journal server. `path` starts at `/api`.
pub trait Http {
    fn send(&self, method: &str, path: &str, body: Option<&Value>) -> Result<HttpResponse, String>;
}

fn enc(s: &str) -> String {
    utf8_percent_encode(s, COMPONENT).to_string()
}

/// The server's error message, or the status when it gave none.
fn error_of(response: &HttpResponse) -> ApiError {
    let message = serde_json::from_str::<Value>(&response.body)
        .ok()
        .and_then(|v| v.get("error")?.as_str().map(str::to_string));
    ApiError(message.unwrap_or_else(|| format!("HTTP {}", response.status)))
}

fn is_ok(response: &HttpResponse) -> bool {
    (200..300).contains(&response.status)
}

fn ensure_ok(response: &HttpResponse) -> ApiResult<()> {
    if is_ok(response) {
        Ok(())
    } else {
        Err(error_of(response))
    }
}

fn decode<T: DeserializeOwned>(response: &HttpResponse) -> ApiResult<T> {
    ensure_ok(response)?;
    serde_json::from_str(&response.body).map_err(|e| ApiError(e.to_string()))
}

/// A JSON object without the fields that are not set, for an update that leaves the other
/// fields as they are.
fn without_nulls(value: Value) -> Value {
    match value {
        Value::Object(fields) => {
            Value::Object(fields.into_iter().filter(|(_, v)| !v.is_null()).collect())
        }
        other => other,
    }
}

/// The method, the path under `/api/pages/{id}/atlas/` and the body of a change of an atlas.
fn atlas_request(change: AtlasChange) -> (&'static str, String, Option<Value>) {
    match change {
        AtlasChange::AddLink { target, bounds } => (
            "POST",
            "links".into(),
            Some(json!({ "target": target, "bounds": bounds })),
        ),
        AtlasChange::SetLink { item_id, target } => (
            "PUT",
            format!("links/{item_id}"),
            Some(json!({ "target": target })),
        ),
        AtlasChange::Group { cards, target } => (
            "POST",
            "group".into(),
            Some(json!({ "cards": cards, "target": target })),
        ),
        AtlasChange::Ungroup { cards } => {
            ("POST", "ungroup".into(), Some(json!({ "cards": cards })))
        }
        AtlasChange::SetGroupTitle {
            item_id,
            side,
            title,
        } => (
            "PUT",
            format!("groups/{item_id}"),
            Some(json!({ "side": side, "title": title })),
        ),
        AtlasChange::AddCallout { bounds, side, text } => (
            "POST",
            "callouts".into(),
            Some(json!({ "bounds": bounds, "side": side, "text": text })),
        ),
        AtlasChange::SetCallout {
            item_id,
            side,
            text,
        } => (
            "PUT",
            format!("callouts/{item_id}"),
            Some(json!({ "side": side, "text": text })),
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
struct FlatEnvelope {
    flat_markdown: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ItemIdsEnvelope {
    item_ids: Vec<String>,
}
#[derive(Deserialize)]
struct ParsedEnvelope {
    items: Vec<ParsedFlatItem>,
}
#[derive(Deserialize)]
struct LaunchEnvelope {
    success: bool,
}

/// The requests of one [`call`], with the transport errors already turned into [`ApiError`].
struct Client<'a, H: Http> {
    http: &'a H,
}

impl<H: Http> Client<'_, H> {
    fn send(&self, method: &str, path: &str, body: Option<Value>) -> ApiResult<HttpResponse> {
        self.http
            .send(method, path, body.as_ref())
            .map_err(ApiError)
    }

    fn json<T: DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> ApiResult<T> {
        decode(&self.send(method, path, body)?)
    }

    fn get<T: DeserializeOwned>(&self, path: &str) -> ApiResult<T> {
        self.json("GET", path, None)
    }

    fn empty(&self, method: &str, path: &str, body: Option<Value>) -> ApiResult<ApiOutput> {
        ensure_ok(&self.send(method, path, body)?)?;
        Ok(ApiOutput::Done)
    }

    fn item(&self, method: &str, path: &str, body: Option<Value>) -> ApiResult<ApiOutput> {
        let envelope: ItemEnvelope = self.json(method, path, body)?;
        Ok(ApiOutput::Item(envelope.item))
    }

    fn items(&self, method: &str, path: &str, body: Option<Value>) -> ApiResult<ApiOutput> {
        let envelope: ItemsEnvelope = self.json(method, path, body)?;
        Ok(ApiOutput::Items(envelope.items))
    }

    fn page_info(&self, method: &str, path: &str, body: Option<Value>) -> ApiResult<ApiOutput> {
        let envelope: PageEnvelope = self.json(method, path, body)?;
        Ok(ApiOutput::PageInfo(envelope.page))
    }

    fn pages(&self, path: &str) -> ApiResult<ApiOutput> {
        let envelope: PagesEnvelope = self.get(path)?;
        Ok(ApiOutput::Pages(envelope.pages))
    }

    fn search(&self, path: &str) -> ApiResult<ApiOutput> {
        let envelope: ResultsEnvelope = self.get(path)?;
        Ok(ApiOutput::SearchResults(envelope.results))
    }

    fn definitions(&self, method: &str, path: &str, body: Option<Value>) -> ApiResult<ApiOutput> {
        let envelope: DefinitionsEnvelope = self.json(method, path, body)?;
        Ok(ApiOutput::PropertyDefinitions(envelope.definitions))
    }
}

/// Performs one operation and answers it the way the core expects.
pub fn call<H: Http>(http: &H, op: ApiOp) -> ApiResult<ApiOutput> {
    let client = Client { http };
    match op {
        ApiOp::GetPageById { page_id } => Ok(ApiOutput::Page(
            client.get(&format!("/api/pages/{page_id}"))?,
        )),
        ApiOp::GetMostRecentJournalBefore { before } => Ok(ApiOutput::JournalBefore(
            client.get(&format!("/api/journals/before?date={}", enc(&before)))?,
        )),
        ApiOp::GetMostRecentWeeklyJournalBefore { before } => {
            Ok(ApiOutput::JournalBefore(client.get(&format!(
                "/api/journals/weekly/before?week={}",
                enc(&before)
            ))?))
        }
        ApiOp::GetOrCreateJournal { file_name } => Ok(ApiOutput::Page(client.json(
            "POST",
            &format!("/api/journals/{}", enc(&file_name)),
            None,
        )?)),
        ApiOp::GetOrCreateDoc { title, dir_path } => Ok(ApiOutput::Page(client.json(
            "POST",
            "/api/pages/doc",
            Some(json!({ "title": title, "dirPath": dir_path })),
        )?)),
        ApiOp::CreateItem {
            page_id,
            position,
            level,
            body,
            id,
        } => client.item(
            "POST",
            "/api/items",
            Some(json!({
                "pageId": page_id, "position": position.to_api(), "level": level, "body": body,
                "id": id.unwrap_or_default(),
            })),
        ),
        ApiOp::UpdateItem {
            item_id,
            body,
            level,
            collapsed,
        } => client.item(
            "PUT",
            &format!("/api/items/{item_id}"),
            Some(without_nulls(
                json!({ "body": body, "level": level, "collapsed": collapsed }),
            )),
        ),
        ApiOp::ShiftItemLevels { item_ids, delta } => client.items(
            "POST",
            "/api/items/shift-level",
            Some(json!({ "itemIds": item_ids, "delta": delta })),
        ),
        ApiOp::MoveItems {
            item_ids,
            target_page_id,
            position,
            level_delta,
        } => client.items(
            "POST",
            "/api/items/move",
            Some(json!({
                "itemIds": item_ids, "targetPageId": target_page_id,
                "position": position.to_api(), "levelDelta": level_delta,
            })),
        ),
        ApiOp::DeletePage { page_id } => {
            client.empty("DELETE", &format!("/api/pages/{page_id}"), None)
        }
        ApiOp::DeleteItem { item_id } => {
            client.empty("DELETE", &format!("/api/items/{item_id}"), None)
        }
        ApiOp::DeleteItems { item_ids } => client.empty(
            "POST",
            "/api/items/delete",
            Some(json!({ "itemIds": item_ids })),
        ),
        ApiOp::MergeItems { item_id, into_id } => client.item(
            "POST",
            "/api/items/merge",
            Some(json!({ "itemId": item_id, "intoId": into_id })),
        ),
        ApiOp::CycleTask { item_id } => {
            client.item("POST", &format!("/api/items/{item_id}/cycle-task"), None)
        }
        ApiOp::UpsertItemProperty {
            item_id,
            key,
            value_type,
            value,
        } => client.item(
            "PUT",
            &format!("/api/items/{item_id}/properties/{}", enc(&key)),
            Some(json!({ "valueType": value_type, "value": value })),
        ),
        ApiOp::DeleteItemProperty { item_id, key } => client.item(
            "DELETE",
            &format!("/api/items/{item_id}/properties/{}", enc(&key)),
            None,
        ),
        ApiOp::ChangeAtlas { atlas_id, change } => {
            let (method, path, body) = atlas_request(change);
            client.items(method, &format!("/api/pages/{atlas_id}/atlas/{path}"), body)
        }
        ApiOp::SearchCombined { query, max_results } => client.search(&format!(
            "/api/search/combined?q={}&maxResults={max_results}",
            enc(&query)
        )),
        ApiOp::SearchPageNames { query, max_results } => client.search(&format!(
            "/api/search/pages?q={}&maxResults={max_results}",
            enc(&query)
        )),
        ApiOp::GetRecentPages {
            max_results,
            page_type,
        } => {
            let mut path = format!("/api/pages/recent?maxResults={max_results}");
            if let Some(t) = page_type {
                path.push_str(&format!("&type={}", enc(&t)));
            }
            client.pages(&path)
        }
        ApiOp::UpdatePage {
            page_id,
            title,
            dir_path,
            overwrite,
        } => {
            let payload = without_nulls(json!({
                "title": title, "dirPath": dir_path, "overwrite": overwrite.then_some(true),
            }));
            let response = client.send("PUT", &format!("/api/pages/{page_id}"), Some(payload))?;
            // 409: another page has the name, which the core asks the user about
            if response.status == 409 {
                let ApiError(message) = error_of(&response);
                return Ok(ApiOutput::PageNameTaken(message));
            }
            let envelope: PageEnvelope = decode(&response)?;
            Ok(ApiOutput::PageInfo(envelope.page))
        }
        ApiOp::UpsertPageProperty {
            page_id,
            key,
            value_type,
            value,
        } => client.page_info(
            "PUT",
            &format!("/api/pages/{page_id}/properties/{}", enc(&key)),
            Some(json!({ "valueType": value_type, "value": value })),
        ),
        ApiOp::DeletePageProperty { page_id, key } => client.page_info(
            "DELETE",
            &format!("/api/pages/{page_id}/properties/{}", enc(&key)),
            None,
        ),
        ApiOp::GetConfig => Ok(ApiOutput::Config(client.get("/api/config")?)),
        ApiOp::GetPropertyDefinitions => {
            client.definitions("GET", "/api/config/property-definitions", None)
        }
        ApiOp::AddPropertyDefinition { key, value_type } => client.definitions(
            "POST",
            "/api/config/property-definitions",
            Some(json!({ "key": key, "valueType": value_type })),
        ),
        ApiOp::DeletePropertyDefinition { key } => client.definitions(
            "DELETE",
            &format!("/api/config/property-definitions/{}", enc(&key)),
            None,
        ),
        ApiOp::UpdateConfig {
            markdown_dir,
            week_numbering,
            logical_outdent,
            tags_dir,
            home_link,
        } => Ok(ApiOutput::Config(client.json(
            "PUT",
            "/api/config",
            Some(without_nulls(json!({
                "markdownDir": markdown_dir, "weekNumbering": week_numbering,
                "logicalOutdent": logical_outdent, "tagsDir": tags_dir, "homeLink": home_link,
            }))),
        )?)),
        ApiOp::BrowseFolder => {
            let choice: FolderChoice = client.json("POST", "/api/config/browse-folder", None)?;
            Ok(ApiOutput::Folder(choice))
        }
        ApiOp::ExportTreeMarkdown { item_ids } => Ok(ApiOutput::TreeExport(client.json(
            "POST",
            "/api/items/export",
            Some(json!({ "itemIds": item_ids })),
        )?)),
        ApiOp::TranscludedItems { item_ids } => {
            let envelope: ItemIdsEnvelope = client.json(
                "POST",
                "/api/items/transcluded",
                Some(json!({ "itemIds": item_ids })),
            )?;
            Ok(ApiOutput::ItemIds(envelope.item_ids))
        }
        ApiOp::ImportTreeMarkdown { tree_markdown } => {
            let envelope: FlatEnvelope = client.json(
                "POST",
                "/api/items/import",
                Some(json!({ "treeMarkdown": tree_markdown })),
            )?;
            Ok(ApiOutput::FlatMarkdown(envelope.flat_markdown))
        }
        ApiOp::ParseFlatMarkdown { markdown } => {
            let envelope: ParsedEnvelope = client.json(
                "POST",
                "/api/items/parse",
                Some(json!({ "markdown": markdown })),
            )?;
            Ok(ApiOutput::ParsedItems(envelope.items))
        }
        ApiOp::SetTransclusion {
            item_id,
            source_item_id,
        } => client.item(
            "POST",
            &format!("/api/items/{item_id}/transclude"),
            Some(json!({ "sourceItemId": source_item_id })),
        ),
        ApiOp::RemoveTransclusion { item_id } => {
            client.item("DELETE", &format!("/api/items/{item_id}/transclude"), None)
        }
        ApiOp::GetChildPages { parent_id } => {
            client.pages(&format!("/api/pages/{parent_id}/children"))
        }
        ApiOp::GetBacklinks { page_id } => {
            let envelope: BacklinksEnvelope =
                client.get(&format!("/api/pages/{page_id}/backlinks"))?;
            Ok(ApiOutput::Backlinks(envelope.entries))
        }
        // Why not a request: the handle names a browser `File`, and nothing in Emacs makes one
        ApiOp::UploadAsset { .. } => Err(ApiError("Emacs cannot upload files".into())),
        ApiOp::GetFunctions => Ok(ApiOutput::Functions(client.get("/api/functions")?)),
        ApiOp::LaunchUrl { url } => {
            let envelope: LaunchEnvelope =
                client.json("POST", "/api/launcher/open", Some(json!({ "url": url })))?;
            Ok(ApiOutput::Launched(envelope.success))
        }
        ApiOp::OpenExportFolder { page_id } => {
            client.empty("POST", &format!("/api/pages/{page_id}/open-folder"), None)
        }
    }
}
