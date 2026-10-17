//! Performs [`ApiOp`]s against the journal server with `fetch`, like `api.service.ts`.

use crate::core::CoreHandle;
use crux_core::Request;
use gloo_net::http::{Request as Http, Response};
use peta_core::effects::{ApiError, ApiOp, ApiOutput, ApiResult, FolderChoice, FunctionInfo};
use peta_core::model::*;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use wasm_bindgen_futures::spawn_local;

/// `encodeURIComponent`.
fn enc(s: &str) -> String {
    js_sys::encode_uri_component(s)
        .as_string()
        .unwrap_or_default()
}

/// Throws with the server's error message when the response is not 2xx.
async fn ensure_ok(resp: &Response) -> ApiResult<()> {
    if resp.ok() {
        return Ok(());
    }
    let status = resp.status();
    let message = match resp.json::<Value>().await {
        Ok(v) => v.get("error").and_then(|e| e.as_str()).map(str::to_string),
        Err(_) => None,
    };
    Err(ApiError(
        message.unwrap_or_else(|| format!("HTTP {status}")),
    ))
}

async fn json<T: DeserializeOwned>(resp: Response) -> ApiResult<T> {
    ensure_ok(&resp).await?;
    resp.json::<T>().await.map_err(|e| ApiError(e.to_string()))
}

async fn get<T: DeserializeOwned>(url: &str) -> ApiResult<T> {
    let resp = Http::get(url)
        .send()
        .await
        .map_err(|e| ApiError(e.to_string()))?;
    json(resp).await
}

async fn send_json<T: DeserializeOwned>(
    method: &str,
    url: &str,
    body: Option<Value>,
) -> ApiResult<T> {
    let builder = match method {
        "POST" => Http::post(url),
        "PUT" => Http::put(url),
        "DELETE" => Http::delete(url),
        _ => Http::get(url),
    };
    let request = match body {
        Some(body) => builder
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .map_err(|e| ApiError(e.to_string()))?,
        None => builder.build().map_err(|e| ApiError(e.to_string()))?,
    };
    let resp = request.send().await.map_err(|e| ApiError(e.to_string()))?;
    json(resp).await
}

async fn send_empty(method: &str, url: &str) -> ApiResult<()> {
    let builder = match method {
        "POST" => Http::post(url),
        "DELETE" => Http::delete(url),
        _ => Http::get(url),
    };
    let resp = builder.send().await.map_err(|e| ApiError(e.to_string()))?;
    ensure_ok(&resp).await
}

#[derive(serde::Deserialize)]
struct ItemEnvelope {
    item: Item,
}
#[derive(serde::Deserialize)]
struct ItemsEnvelope {
    items: Vec<Item>,
}
#[derive(serde::Deserialize)]
struct PageEnvelope {
    page: Page,
}
#[derive(serde::Deserialize)]
struct PagesEnvelope {
    pages: Vec<Page>,
}
#[derive(serde::Deserialize)]
struct ResultsEnvelope {
    results: Vec<SearchResult>,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PropertyDefinitions {
    definitions: Vec<peta_core::model::PropertyDefinition>,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FlatEnvelope {
    flat_markdown: String,
}
#[derive(serde::Deserialize)]
struct ParsedEnvelope {
    items: Vec<ParsedFlatItem>,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadEnvelope {
    file_name: String,
}
#[derive(serde::Deserialize)]
struct LaunchEnvelope {
    success: bool,
}

pub fn perform(handle: CoreHandle, mut request: Request<ApiOp>) {
    let op = request.operation.clone();
    spawn_local(async move {
        let output = call(&handle, op).await;
        handle.resolve(&mut request, output);
    });
}

async fn call(handle: &CoreHandle, op: ApiOp) -> ApiResult<ApiOutput> {
    Ok(match op {
        ApiOp::GetPageById { page_id } => {
            ApiOutput::Page(get(&format!("/api/pages/{page_id}")).await?)
        }
        ApiOp::GetMostRecentJournalBefore { before } => ApiOutput::JournalBefore(
            get(&format!("/api/journals/before?date={}", enc(&before))).await?,
        ),
        ApiOp::GetMostRecentWeeklyJournalBefore { before } => ApiOutput::JournalBefore(
            get(&format!(
                "/api/journals/weekly/before?week={}",
                enc(&before)
            ))
            .await?,
        ),
        ApiOp::GetOrCreateJournal { file_name } => ApiOutput::Page(
            send_json("POST", &format!("/api/journals/{}", enc(&file_name)), None).await?,
        ),
        ApiOp::GetOrCreateDoc { title, dir_path } => ApiOutput::Page(
            send_json(
                "POST",
                "/api/pages/doc",
                Some(json!({ "title": title, "dirPath": dir_path })),
            )
            .await?,
        ),
        ApiOp::CreateItem {
            page_id,
            position,
            level,
            body,
            id,
        } => {
            let payload = json!({
                "pageId": page_id, "position": position.to_api(), "level": level, "body": body,
                "id": id.unwrap_or_default(),
            });
            let env: ItemEnvelope = send_json("POST", "/api/items", Some(payload)).await?;
            ApiOutput::Item(env.item)
        }
        ApiOp::UpdateItem {
            item_id,
            body,
            level,
            collapsed,
        } => {
            let mut payload = serde_json::Map::new();
            if let Some(b) = body {
                payload.insert("body".into(), Value::String(b));
            }
            if let Some(l) = level {
                payload.insert("level".into(), json!(l));
            }
            if let Some(c) = collapsed {
                payload.insert("collapsed".into(), json!(c));
            }
            let env: ItemEnvelope = send_json(
                "PUT",
                &format!("/api/items/{item_id}"),
                Some(Value::Object(payload)),
            )
            .await?;
            ApiOutput::Item(env.item)
        }
        ApiOp::ShiftItemLevels { item_ids, delta } => {
            let env: ItemsEnvelope = send_json(
                "POST",
                "/api/items/shift-level",
                Some(json!({ "itemIds": item_ids, "delta": delta })),
            )
            .await?;
            ApiOutput::Items(env.items)
        }
        ApiOp::MoveItems {
            item_ids,
            target_page_id,
            position,
            level_delta,
        } => {
            let env: ItemsEnvelope = send_json(
                "POST",
                "/api/items/move",
                Some(json!({
                    "itemIds": item_ids, "targetPageId": target_page_id,
                    "position": position.to_api(), "levelDelta": level_delta,
                })),
            )
            .await?;
            ApiOutput::Items(env.items)
        }
        ApiOp::DeletePage { page_id } => {
            send_empty("DELETE", &format!("/api/pages/{page_id}")).await?;
            ApiOutput::Done
        }
        ApiOp::DeleteItem { item_id } => {
            send_empty("DELETE", &format!("/api/items/{item_id}")).await?;
            ApiOutput::Done
        }
        ApiOp::CycleTask { item_id } => {
            let env: ItemEnvelope =
                send_json("POST", &format!("/api/items/{item_id}/cycle-task"), None).await?;
            ApiOutput::Item(env.item)
        }
        ApiOp::UpsertItemProperty {
            item_id,
            key,
            value_type,
            value,
        } => {
            let env: ItemEnvelope = send_json(
                "PUT",
                &format!("/api/items/{item_id}/properties/{}", enc(&key)),
                Some(json!({ "valueType": value_type, "value": value })),
            )
            .await?;
            ApiOutput::Item(env.item)
        }
        ApiOp::SearchCombined { query, max_results } => {
            let env: ResultsEnvelope = get(&format!(
                "/api/search/combined?q={}&maxResults={max_results}",
                enc(&query)
            ))
            .await?;
            ApiOutput::SearchResults(env.results)
        }
        ApiOp::SearchPageNames { query, max_results } => {
            let env: ResultsEnvelope = get(&format!(
                "/api/search/pages?q={}&maxResults={max_results}",
                enc(&query)
            ))
            .await?;
            ApiOutput::SearchResults(env.results)
        }
        ApiOp::GetRecentPages {
            max_results,
            page_type,
        } => {
            let mut url = format!("/api/pages/recent?maxResults={max_results}");
            if let Some(t) = page_type {
                url.push_str(&format!("&type={}", enc(&t)));
            }
            let env: PagesEnvelope = get(&url).await?;
            ApiOutput::Pages(env.pages)
        }
        ApiOp::UpdatePage {
            page_id,
            title,
            dir_path,
        } => {
            let mut payload = serde_json::Map::new();
            if let Some(t) = title {
                payload.insert("title".into(), Value::String(t));
            }
            if let Some(d) = dir_path {
                payload.insert("dirPath".into(), Value::String(d));
            }
            let env: PageEnvelope = send_json(
                "PUT",
                &format!("/api/pages/{page_id}"),
                Some(Value::Object(payload)),
            )
            .await?;
            ApiOutput::PageInfo(env.page)
        }
        ApiOp::UpsertPageProperty {
            page_id,
            key,
            value_type,
            value,
        } => {
            let env: PageEnvelope = send_json(
                "PUT",
                &format!("/api/pages/{page_id}/properties/{}", enc(&key)),
                Some(json!({ "valueType": value_type, "value": value })),
            )
            .await?;
            ApiOutput::PageInfo(env.page)
        }
        ApiOp::DeletePageProperty { page_id, key } => {
            let env: PageEnvelope = send_json(
                "DELETE",
                &format!("/api/pages/{page_id}/properties/{}", enc(&key)),
                None,
            )
            .await?;
            ApiOutput::PageInfo(env.page)
        }
        ApiOp::GetConfig => ApiOutput::Config(get("/api/config").await?),
        ApiOp::GetPropertyDefinitions => {
            let env: PropertyDefinitions = get("/api/config/property-definitions").await?;
            ApiOutput::PropertyDefinitions(env.definitions)
        }
        ApiOp::AddPropertyDefinition { key, value_type } => {
            let env: PropertyDefinitions = send_json(
                "POST",
                "/api/config/property-definitions",
                Some(json!({ "key": key, "valueType": value_type })),
            )
            .await?;
            ApiOutput::PropertyDefinitions(env.definitions)
        }
        ApiOp::UpdateConfig {
            markdown_dir,
            week_numbering,
            logical_outdent,
        } => {
            let mut payload = serde_json::Map::new();
            if let Some(v) = markdown_dir {
                payload.insert("markdownDir".into(), Value::String(v));
            }
            if let Some(v) = week_numbering {
                payload.insert("weekNumbering".into(), Value::String(v));
            }
            if let Some(v) = logical_outdent {
                payload.insert("logicalOutdent".into(), json!(v));
            }
            ApiOutput::Config(send_json("PUT", "/api/config", Some(Value::Object(payload))).await?)
        }
        ApiOp::BrowseFolder => {
            let choice: FolderChoice = send_json("POST", "/api/config/browse-folder", None).await?;
            ApiOutput::Folder(choice)
        }
        ApiOp::ExportTreeMarkdown { item_ids } => ApiOutput::TreeExport(
            send_json(
                "POST",
                "/api/items/export",
                Some(json!({ "itemIds": item_ids })),
            )
            .await?,
        ),
        ApiOp::ImportTreeMarkdown { tree_markdown } => {
            let env: FlatEnvelope = send_json(
                "POST",
                "/api/items/import",
                Some(json!({ "treeMarkdown": tree_markdown })),
            )
            .await?;
            ApiOutput::FlatMarkdown(env.flat_markdown)
        }
        ApiOp::ParseFlatMarkdown { markdown } => {
            let env: ParsedEnvelope = send_json(
                "POST",
                "/api/items/parse",
                Some(json!({ "markdown": markdown })),
            )
            .await?;
            ApiOutput::ParsedItems(env.items)
        }
        ApiOp::SetTransclusion {
            item_id,
            source_item_id,
        } => {
            let env: ItemEnvelope = send_json(
                "POST",
                &format!("/api/items/{item_id}/transclude"),
                Some(json!({ "sourceItemId": source_item_id })),
            )
            .await?;
            ApiOutput::Item(env.item)
        }
        ApiOp::RemoveTransclusion { item_id } => {
            let env: ItemEnvelope =
                send_json("DELETE", &format!("/api/items/{item_id}/transclude"), None).await?;
            ApiOutput::Item(env.item)
        }
        ApiOp::GetChildPages { parent_id } => {
            let env: PagesEnvelope = get(&format!("/api/pages/{parent_id}/children")).await?;
            ApiOutput::Pages(env.pages)
        }
        ApiOp::UploadAsset { file } => {
            let file = handle
                .take_file(file)
                .ok_or_else(|| ApiError("file is gone".into()))?;
            let form = web_sys::FormData::new().map_err(|_| ApiError("FormData".into()))?;
            form.append_with_blob("file", &file)
                .map_err(|_| ApiError("FormData".into()))?;
            let resp = Http::post("/api/assets/upload")
                .body(form)
                .map_err(|e| ApiError(e.to_string()))?
                .send()
                .await
                .map_err(|e| ApiError(e.to_string()))?;
            if !resp.ok() {
                return Err(ApiError(format!("Upload failed: {}", resp.status())));
            }
            let env: UploadEnvelope = resp.json().await.map_err(|e| ApiError(e.to_string()))?;
            ApiOutput::FileName(env.file_name)
        }
        ApiOp::GetFunctions => {
            let functions: Vec<FunctionInfo> = get("http://localhost:8087/api/functions").await?;
            ApiOutput::Functions(functions)
        }
        ApiOp::LaunchUrl { url } => {
            let env: LaunchEnvelope = send_json(
                "POST",
                "http://localhost:8087/api/launcher/open",
                Some(json!({ "url": url })),
            )
            .await?;
            ApiOutput::Launched(env.success)
        }
    })
}
