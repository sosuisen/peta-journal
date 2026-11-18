//! Performs [`ApiOp`]s against the journal server with `fetch`. The core knows the requests and
//! the answers.

use crate::core::CoreHandle;
use client_core::effects::{ApiError, ApiOp, ApiOutput, ApiResult};
use client_core::model::Uploaded;
use client_core::policy::InOrder;
use client_core::rest::{rest_call, ClientId, HttpReply, HttpRequest};
use crux_core::Request;
use gloo_net::http::Request as Http;
use std::cell::RefCell;
use wasm_bindgen_futures::spawn_local;

/// A transport error as the core sees it.
fn api_err(e: impl std::fmt::Display) -> ApiError {
    ApiError(e.to_string())
}

/// Sends a request with a JSON body, when there is one, and gives the answer back as it is,
/// whatever its status. Every request names this tab, as the server refuses a change that
/// does not (ADR-015).
async fn send(handle: &CoreHandle, request: &HttpRequest) -> ApiResult<HttpReply> {
    let builder = match request.method {
        "POST" => Http::post(&request.path),
        "PUT" => Http::put(&request.path),
        "DELETE" => Http::delete(&request.path),
        _ => Http::get(&request.path),
    }
    .header(ClientId::HEADER, handle.inner().client_id.as_str());
    let sent = match &request.body {
        Some(body) => builder
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .map_err(api_err)?,
        None => builder.build().map_err(api_err)?,
    };
    let response = sent.send().await.map_err(api_err)?;
    Ok(HttpReply {
        status: response.status(),
        body: response.text().await.map_err(api_err)?,
    })
}

/// Uploads a file the shell holds. Not a REST call of the core: a `File` cannot cross into it.
async fn upload(handle: &CoreHandle, file: u64) -> ApiResult<ApiOutput> {
    let file = handle
        .take_file(file)
        .ok_or_else(|| ApiError("file is gone".into()))?;
    let form = web_sys::FormData::new().map_err(|_| ApiError("FormData".into()))?;
    form.append_with_blob("file", &file)
        .map_err(|_| ApiError("FormData".into()))?;
    let resp = Http::post("/api/assets/upload")
        .header(ClientId::HEADER, handle.inner().client_id.as_str())
        .body(form)
        .map_err(api_err)?
        .send()
        .await
        .map_err(api_err)?;
    if !resp.ok() {
        return Err(ApiError(format!("Upload failed: {}", resp.status())));
    }
    let uploaded: Uploaded = resp.json().await.map_err(api_err)?;
    Ok(ApiOutput::Uploaded(uploaded))
}

thread_local! {
    static REQUESTS: RefCell<InOrder<Request<ApiOp>>> = const { RefCell::new(InOrder::new()) };
}

/// Asks the server, one request at a time and in the order the core made them, as every
/// shell does (`client_core::policy`). The browser sends the requests of a page side by side,
/// so with a task for each request a later one overtook a slow one: a join reached the server
/// before the save of the text just typed, and the text was lost.
pub fn perform(handle: CoreHandle, request: Request<ApiOp>) {
    if !REQUESTS.with_borrow_mut(|requests| requests.push(request)) {
        return;
    }
    spawn_local(async move {
        // The answer of a request may make the core ask again: that request goes to the
        // end of the line, behind those that wait already
        while let Some(mut request) = REQUESTS.with_borrow_mut(InOrder::take_next) {
            let output = call(&handle, request.operation.clone()).await;
            handle.resolve(&mut request, output);
        }
    });
}

async fn call(handle: &CoreHandle, op: ApiOp) -> ApiResult<ApiOutput> {
    if let ApiOp::UploadAsset { file } = op {
        return upload(handle, file).await;
    }
    let rest = rest_call(op).ok_or_else(|| ApiError("not a REST call".into()))?;
    let reply = send(handle, &rest.request).await?;
    rest.decode(&reply)
}
