//! Talks to a running server over HTTP the way the frontend does. The whole test binary shares
//! one server with its database, search index, markdown folder and config file under a temporary
//! directory, so the tests never touch `~/.peta-journal`.

#![allow(dead_code)]

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use axum::serve::ListenerExt;
use journal_server::config::Settings;
use journal_server::state::{AppState, BuildOptions, SharedState};
use serde_json::{json, Value};

pub struct TestServer {
    pub base_url: String,
    pub root: PathBuf,
    pub markdown_dir: PathBuf,
    /// The state of the server, for what a test can only set up under the API
    pub state: SharedState,
    _keep: tempfile::TempDir,
}

static SERVER: OnceLock<TestServer> = OnceLock::new();
static COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn server() -> &'static TestServer {
    SERVER.get_or_init(|| {
        let dir = tempfile::Builder::new()
            .prefix("peta-journal-it-")
            .tempdir()
            .expect("temp dir");
        let root = dir.path().to_path_buf();
        let markdown_dir = root.join("markdown");
        std::fs::create_dir_all(&markdown_dir).unwrap();
        let config_dir = root.join("config");
        std::fs::create_dir_all(&config_dir).unwrap();
        // The markdown folder is written into the config file before the server starts;
        // without it the startup would open the native folder chooser.
        std::fs::write(
            config_dir.join("config.json"),
            json!({ "markdownDir": markdown_dir.to_string_lossy(), "weekNumbering": "iso" })
                .to_string(),
        )
        .unwrap();

        let (tx, rx) = std::sync::mpsc::channel::<(SocketAddr, SharedState)>();
        let settings_root = root.clone();
        std::thread::Builder::new()
            .name("test-server".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                rt.block_on(async move {
                    let mut settings = Settings::under(&settings_root, 0);
                    settings.export_interval_secs = 1;
                    let state = AppState::build(settings, BuildOptions { interactive: false })
                        .expect("state");
                    state.start_background_tasks();
                    let app = journal_server::build_router(state.clone());
                    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                    tx.send((listener.local_addr().unwrap(), state)).unwrap();
                    let listener = listener.tap_io(|tcp| {
                        let _ = tcp.set_nodelay(true);
                    });
                    axum::serve(listener, app).await.unwrap();
                });
            })
            .unwrap();
        let (addr, state) = rx.recv().expect("server address");
        TestServer {
            base_url: format!("http://{addr}"),
            root,
            markdown_dir,
            state,
            _keep: dir,
        }
    })
}

pub fn client() -> reqwest::Client {
    reqwest::Client::new()
}

pub fn url(path: &str) -> String {
    format!("{}{}", server().base_url, path)
}

pub async fn get(path: &str) -> (u16, Value) {
    let resp = client().get(url(path)).send().await.expect("GET");
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    (status, serde_json::from_str(&text).unwrap_or(Value::Null))
}

pub async fn send(method: reqwest::Method, path: &str, body: Option<Value>) -> (u16, Value) {
    let mut req = client().request(method, url(path));
    if let Some(b) = body {
        req = req.json(&b);
    }
    let resp = req.send().await.expect("request");
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    (status, serde_json::from_str(&text).unwrap_or(Value::Null))
}

pub async fn post(path: &str, body: Value) -> (u16, Value) {
    send(reqwest::Method::POST, path, Some(body)).await
}

pub async fn post_empty(path: &str) -> (u16, Value) {
    send(reqwest::Method::POST, path, None).await
}

pub async fn put(path: &str, body: Value) -> (u16, Value) {
    send(reqwest::Method::PUT, path, Some(body)).await
}

pub async fn delete(path: &str) -> u16 {
    send(reqwest::Method::DELETE, path, None).await.0
}

/// An item as the server returns it; only the fields the tests look at.
#[derive(Debug, Clone)]
pub struct ItemRow {
    pub id: String,
    pub level: i64,
    pub body: String,
    pub types: Vec<String>,
    pub properties: serde_json::Map<String, Value>,
    pub collapsed: bool,
    pub transclude_item_id: Option<String>,
}

impl ItemRow {
    pub fn of(json: &Value) -> ItemRow {
        ItemRow {
            id: json["id"].as_str().unwrap_or("").to_string(),
            level: json["level"].as_i64().unwrap_or(0),
            body: json["body"].as_str().unwrap_or("").to_string(),
            types: json["types"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            properties: json["properties"].as_object().cloned().unwrap_or_default(),
            collapsed: json["collapsed"].as_bool().unwrap_or(false),
            transclude_item_id: json["transcludeItemId"].as_str().map(String::from),
        }
    }

    /// "level:body", the shape most assertions compare.
    pub fn outline(&self) -> String {
        format!("{}:{}", self.level, self.body)
    }
}

#[derive(Debug, Clone)]
pub struct PageRef {
    pub id: String,
    pub title: String,
}

/// Creates an empty doc page with a unique title; a new page starts with one empty item.
pub async fn new_doc_page(prefix: &str) -> PageRef {
    let title = format!(
        "{prefix}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    );
    let (status, resp) = post("/api/pages/doc", json!({ "title": title, "dirPath": "" })).await;
    assert_eq!(status, 200, "{resp}");
    PageRef {
        id: resp["page"]["id"].as_str().unwrap().to_string(),
        title,
    }
}

pub async fn items(page_id: &str) -> Vec<ItemRow> {
    let (status, resp) = get(&format!("/api/pages/{page_id}")).await;
    assert_eq!(status, 200, "{resp}");
    resp["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(ItemRow::of)
        .collect()
}

pub async fn outline_of(page_id: &str) -> Vec<String> {
    items(page_id).await.iter().map(ItemRow::outline).collect()
}

pub async fn create_item(page_id: &str, position: &str, level: i64, body: &str) -> ItemRow {
    let (status, resp) = post(
        "/api/items",
        json!({ "pageId": page_id, "position": position, "level": level, "body": body, "id": "" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    ItemRow::of(&resp["item"])
}

/// Fills a page with items given as "body" or "  body" (two spaces per level below 1),
/// replacing the empty item a new page starts with, and returns them in page order.
pub async fn fill(page_id: &str, lines: &[&str]) -> Vec<ItemRow> {
    let first = items(page_id).await.into_iter().next().unwrap();
    let mut after = first.id.clone();
    for line in lines {
        let body = line.trim_start();
        let level = 1 + (line.len() - body.len()) as i64 / 2;
        after = create_item(page_id, &format!("after:{after}"), level, body)
            .await
            .id;
    }
    assert_eq!(delete(&format!("/api/items/{}", first.id)).await, 204);
    items(page_id).await
}

pub async fn update_item(item_id: &str, changes: Value) -> ItemRow {
    let (status, resp) = put(&format!("/api/items/{item_id}"), changes).await;
    assert_eq!(status, 200, "{resp}");
    ItemRow::of(&resp["item"])
}

pub async fn move_items(
    item_ids: &[&str],
    target_page_id: &str,
    position: &str,
    level_delta: Option<i64>,
) -> Vec<ItemRow> {
    let mut req =
        json!({ "itemIds": item_ids, "targetPageId": target_page_id, "position": position });
    if let Some(d) = level_delta {
        req["levelDelta"] = json!(d);
    }
    let (status, resp) = post("/api/items/move", req).await;
    assert_eq!(status, 200, "{resp}");
    resp["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(ItemRow::of)
        .collect()
}

pub fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

pub fn ints(v: &Value) -> Vec<i64> {
    v.as_array()
        .map(|a| a.iter().filter_map(|x| x.as_i64()).collect())
        .unwrap_or_default()
}

/// `items[*].field` of a response.
pub fn field_of_each(v: &Value, list: &str, field: &str) -> Vec<Value> {
    v[list]
        .as_array()
        .map(|a| a.iter().map(|x| x[field].clone()).collect())
        .unwrap_or_default()
}
