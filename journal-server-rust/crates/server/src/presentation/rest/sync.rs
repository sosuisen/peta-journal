//! Folder <-> DB sync: status, a manual scan, and the "files disappeared" confirmation page.

use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Form, Json, Router};
use hmtid::Hmtid;
use serde::{Deserialize, Serialize};

use crate::domain::model::Page;
use crate::error::{AppError, AppResult};
use crate::infrastructure::export::file_sync_state::FileSyncState;
use crate::state::{blocking, SharedState};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/api/sync/status", get(status))
        .route("/api/sync/scan", post(scan))
        .route("/api/sync/pending-deletes", get(pending_page).post(pending_submit))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusResponse {
    import_enabled: bool,
    markdown_root: String,
    root_available: bool,
    tracked_files: usize,
    missing_files: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanResponse {
    imported: i32,
    conflicts: i32,
    new_missing: i32,
    exported: i32,
}

async fn status(State(state): State<SharedState>) -> AppResult<Json<StatusResponse>> {
    blocking(move || {
        let root = state.export.markdown_root();
        Ok(Json(StatusResponse {
            import_enabled: state.settings.import_enabled,
            markdown_root: root.as_ref().map(|r| r.display().to_string()).unwrap_or_default(),
            root_available: root.is_some(),
            tracked_files: state.file_sync_state.find_all()?.len(),
            missing_files: state.file_sync_state.find_missing()?.len(),
        }))
    })
    .await
}

/// Runs a full folder scan now and waits for it. Returns zeros if a scan is already running.
async fn scan(State(state): State<SharedState>) -> AppResult<Json<ScanResponse>> {
    blocking(move || {
        let s = state.reconcile.scan();
        Ok(Json(ScanResponse {
            imported: s.imported,
            conflicts: s.conflicts,
            new_missing: s.new_missing,
            exported: s.exported,
        }))
    })
    .await
}

struct Row {
    state: FileSyncState,
    page: Page,
    item_count: usize,
}

/// The form posts `action` once and `pageId` once per ticked row.
#[derive(Debug, Default)]
struct PendingForm {
    action: String,
    page_ids: Vec<String>,
}

impl<'de> Deserialize<'de> for PendingForm {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let pairs: Vec<(String, String)> = Vec::deserialize(deserializer)?;
        let mut form = PendingForm::default();
        for (k, v) in pairs {
            match k.as_str() {
                "action" => form.action = v,
                "pageId" => form.page_ids.push(v),
                _ => {}
            }
        }
        Ok(form)
    }
}

fn load_rows(state: &SharedState) -> AppResult<Vec<Row>> {
    let mut rows = Vec::new();
    for s in state.file_sync_state.find_missing()? {
        match state.pages.find_by_id(&s.page_id)? {
            None => {
                state.file_sync_state.delete_by_path(&s.rel_path)?; // page already gone
            }
            Some(page) => {
                let item_count = state.items.find_by_page_id(&page.id)?.len();
                rows.push(Row {
                    state: s,
                    page,
                    item_count,
                });
            }
        }
    }
    Ok(rows)
}

async fn pending_page(State(state): State<SharedState>) -> AppResult<Html<String>> {
    blocking(move || Ok(Html(render(&load_rows(&state)?, &[])))).await
}

fn is_local(url: Option<&str>) -> bool {
    match url {
        None => false,
        Some(u) => {
            u.starts_with("http://localhost:")
                || u.starts_with("http://127.0.0.1:")
                || u == "http://localhost"
                || u == "http://127.0.0.1"
        }
    }
}

async fn pending_submit(State(state): State<SharedState>, headers: HeaderMap, Form(form): Form<PendingForm>) -> AppResult<Response> {
    // Only accept submissions from this page itself, not from other sites
    let origin = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok());
    let referer = headers.get(header::REFERER).and_then(|v| v.to_str().ok());
    if !is_local(origin) && !is_local(referer) {
        return Ok((StatusCode::FORBIDDEN, "Forbidden").into_response());
    }
    blocking(move || {
        let mut messages: Vec<String> = Vec::new();
        let restore = form.action == "restore";
        for id_str in &form.page_ids {
            let Ok(id) = Hmtid::parse(id_str) else { continue };
            let page = state.pages.find_by_id(&id)?;
            let label = page
                .as_ref()
                .map(|p| format!("{}s/{}{}.md", p.page_type, p.dir_path, p.name))
                .unwrap_or_else(|| id_str.clone());
            let result: AppResult<String> = if restore {
                // Write now, so the message below is true when the page is shown
                state.export.export_page(&id).map(|_| format!("Restored file: {label}"))
            } else {
                state.page_service.delete_page(&id).map(|_| format!("Deleted: {label}"))
            };
            match result {
                Ok(m) => messages.push(m),
                Err(AppError::BadRequest(m)) => messages.push(format!("Skipped {label}: {m}")),
                Err(e) => {
                    tracing::warn!("Failed to {} page {id}: {e}", if restore { "restore" } else { "delete" });
                    messages.push(format!("Failed {label}: {e}"));
                }
            }
        }
        Ok(Html(render(&load_rows(&state)?, &messages)).into_response())
    })
    .await
}

fn escape(s: &str) -> String {
    html_escape::encode_double_quoted_attribute(s).into_owned()
}

fn render(rows: &[Row], messages: &[String]) -> String {
    let mut sb = String::from(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>PetaJournal - Files disappeared</title>
<style>
  body { font-family: system-ui, sans-serif; max-width: 860px; margin: 2rem auto; padding: 0 1rem; color: #222; }
  h1 { font-size: 1.4rem; }
  .note { background: #fff8e1; border: 1px solid #f0d58c; padding: .75rem 1rem; border-radius: 6px; }
  table { border-collapse: collapse; width: 100%; margin: 1rem 0; }
  th, td { text-align: left; padding: .4rem .6rem; border-bottom: 1px solid #ddd; vertical-align: top; }
  th { background: #f5f5f5; }
  code { font-family: ui-monospace, monospace; }
  .actions button { padding: .5rem 1rem; margin-right: .5rem; }
  .danger { background: #c62828; color: #fff; border: 0; border-radius: 4px; }
  .msg { background: #e8f5e9; border: 1px solid #a5d6a7; padding: .5rem 1rem; border-radius: 6px; margin-bottom: 1rem; }
  .muted { color: #666; }
</style>
</head>
<body>
<h1>Markdown files disappeared</h1>
<p class="note">
  The files below were removed from the markdown folder (or are not visible right now).
  Dropbox <b>selective sync</b>, folders that have not finished syncing, and
  <b>online-only</b> files show up here too. Nothing is deleted until you tick a file and press
  <b>Delete selected</b>. Untick anything you are not sure about.
</p>
"#,
    );
    for m in messages {
        sb.push_str(&format!("<div class=\"msg\">{}</div>\n", escape(m)));
    }
    if rows.is_empty() {
        sb.push_str("<p>No pending files. You can close this tab.</p>\n</body></html>");
        return sb;
    }
    sb.push_str(
        r#"<form method="post">
<table>
<thead><tr>
  <th><input type="checkbox" id="selectAll" title="Select all"
             onchange="document.querySelectorAll('input[name=pageId]').forEach(function(c){c.checked=this.checked}, this)"></th>
  <th>File</th><th>Items</th><th>Missing since</th>
</tr></thead>
<tbody>
"#,
    );
    for row in rows {
        let since = row
            .state
            .missing_since
            .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis)
            .map(|t| t.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_default();
        sb.push_str(&format!(
            "<tr><td><input type=\"checkbox\" name=\"pageId\" value=\"{}\"></td><td><code>{}</code></td><td>{}</td><td class=\"muted\">{}</td></tr>\n",
            escape(row.page.id.as_str()),
            escape(&row.state.rel_path),
            row.item_count,
            escape(&since)
        ));
    }
    sb.push_str(
        r#"</tbody>
</table>
<p class="actions">
  <button type="submit" name="action" value="delete" class="danger"
          onclick="return confirm('Delete the selected pages from the database? This cannot be undone.')">Delete selected</button>
  <button type="submit" name="action" value="restore">Write selected back to the folder</button>
</p>
<p class="muted">Pages you leave unticked stay in the database and will be listed again next time.</p>
</form>
<script>
  // Keep the header checkbox in sync when rows are ticked one by one
  document.querySelectorAll('input[name=pageId]').forEach(function (c) {
    c.addEventListener('change', function () {
      var all = document.querySelectorAll('input[name=pageId]');
      var checked = document.querySelectorAll('input[name=pageId]:checked');
      var head = document.getElementById('selectAll');
      head.checked = checked.length === all.length;
      head.indeterminate = checked.length > 0 && checked.length < all.length;
    });
  });
</script>
</body></html>
"#,
    );
    sb
}
