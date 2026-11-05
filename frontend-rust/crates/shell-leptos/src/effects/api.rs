//! Performs [`ApiOp`]s against the journal server with `fetch`. The core knows the requests and
//! the answers.

use crate::core::CoreHandle;
use crux_core::Request;
use gloo_net::http::Request as Http;
use peta_core::effects::{ApiError, ApiOp, ApiOutput, ApiResult};
use peta_core::rest::{rest_call, HttpReply, HttpRequest};
use wasm_bindgen_futures::spawn_local;

/// A transport error as the core sees it.
fn api_err(e: impl std::fmt::Display) -> ApiError {
    ApiError(e.to_string())
}

/// Sends a request with a JSON body, when there is one, and gives the answer back as it is,
/// whatever its status.
async fn send(request: &HttpRequest) -> ApiResult<HttpReply> {
    let builder = match request.method {
        "POST" => Http::post(&request.path),
        "PUT" => Http::put(&request.path),
        "DELETE" => Http::delete(&request.path),
        _ => Http::get(&request.path),
    };
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

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadEnvelope {
    file_name: String,
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
        .body(form)
        .map_err(api_err)?
        .send()
        .await
        .map_err(api_err)?;
    if !resp.ok() {
        return Err(ApiError(format!("Upload failed: {}", resp.status())));
    }
    let env: UploadEnvelope = resp.json().await.map_err(api_err)?;
    Ok(ApiOutput::FileName(env.file_name))
}

pub fn perform(handle: CoreHandle, mut request: Request<ApiOp>) {
    let op = request.operation.clone();
    spawn_local(async move {
        let output = call(&handle, op).await;
        handle.resolve(&mut request, output);
    });
}

async fn call(handle: &CoreHandle, op: ApiOp) -> ApiResult<ApiOutput> {
    if let ApiOp::UploadAsset { file } = op {
        return upload(handle, file).await;
    }
    let rest = rest_call(op).ok_or_else(|| ApiError("not a REST call".into()))?;
    let reply = send(&rest.request).await?;
    rest.decode(&reply)
}
