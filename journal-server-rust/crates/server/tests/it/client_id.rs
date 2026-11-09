//! The ID that a client names itself with on every change request (ADR-015).

use serde_json::{json, Value};

use crate::common::*;

/// A client that names no ID, unlike [`client`].
fn nameless() -> reqwest::Client {
    reqwest::Client::new()
}

/// The status of an answer and the message of its error.
async fn refusal(response: reqwest::Response) -> (u16, String) {
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap_or(Value::Null);
    (
        status,
        body["error"].as_str().unwrap_or_default().to_string(),
    )
}

// IDを名乗らない変更は400で断られ、メッセージがX-Client-Idヘッダーを挙げる。何も変わらない。
#[tokio::test]
async fn a_change_without_an_id_is_refused_and_the_message_names_the_header() {
    let page = new_doc_page("client-id-missing").await;
    let before = outline_of(&page.id).await;

    let response = nameless()
        .post(url("/api/items"))
        .json(&json!({
            "pageId": page.id, "position": "last", "level": 1, "body": "x", "id": ""
        }))
        .send()
        .await
        .unwrap();

    let (status, message) = refusal(response).await;
    assert_eq!(status, 400);
    assert!(message.contains("X-Client-Id"), "{message}");
    assert_eq!(outline_of(&page.id).await, before);
}

// データを変えないPOSTも、PUTも、DELETEも、IDを名乗らなければ断られる。
#[tokio::test]
async fn every_post_put_and_delete_of_the_api_needs_an_id() {
    let requests = [
        nameless()
            .post(url("/api/items/parse"))
            .json(&json!({ "markdown": "x" })),
        nameless().put(url("/api/config")).json(&json!({})),
        nameless().delete(url("/api/pages/none/atlas/items/none")),
    ];

    for request in requests {
        let (status, message) = refusal(request.send().await.unwrap()).await;

        assert_eq!(status, 400);
        assert!(message.contains("X-Client-Id"), "{message}");
    }
}

// UUIDとして読めないIDの変更は、400で断られる。
#[tokio::test]
async fn a_change_with_an_id_that_is_not_a_uuid_is_refused() {
    let response = nameless()
        .post(url("/api/items/parse"))
        .header("X-Client-Id", "emacs")
        .json(&json!({ "markdown": "x" }))
        .send()
        .await
        .unwrap();

    let (status, message) = refusal(response).await;
    assert_eq!(status, 400);
    assert!(message.contains("X-Client-Id"), "{message}");
}

// GETは、IDを名乗らなくても答えが返る。
#[tokio::test]
async fn a_get_needs_no_id() {
    let response = nameless().get(url("/api/config")).send().await.unwrap();

    assert_eq!(response.status(), 200);
}

// ファイルが消えたときの確認ページのフォームは、IDを名乗らなくても受け付けられる。
#[tokio::test]
async fn the_form_of_the_page_about_missing_files_needs_no_id() {
    let response = nameless()
        .post(url("/api/sync/pending-deletes"))
        .header("Origin", "http://localhost:8087")
        .form(&[("unused", "")])
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}

// ほかのオリジンのページは、X-Client-Idヘッダーを付けたリクエストを送ってよい。
#[tokio::test]
async fn a_page_of_another_origin_may_send_the_header() {
    let response = nameless()
        .request(reqwest::Method::OPTIONS, url("/api/items"))
        .header("Origin", "http://localhost:4200")
        .header("Access-Control-Request-Method", "POST")
        .header("Access-Control-Request-Headers", "x-client-id")
        .send()
        .await
        .unwrap();

    let allowed = response.headers()["access-control-allow-headers"]
        .to_str()
        .unwrap()
        .to_lowercase();
    assert!(allowed.contains("x-client-id"), "{allowed}");
}
