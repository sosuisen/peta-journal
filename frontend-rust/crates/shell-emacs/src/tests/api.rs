use crate::api::{call, Http, HttpResponse};
use peta_core::effects::{ApiError, ApiOp, ApiOutput};
use peta_core::model::ItemPosition;
use serde_json::{json, Value};
use std::cell::RefCell;

/// Answers every request the same way and remembers what was asked.
struct FakeHttp {
    status: u16,
    body: String,
    asked: RefCell<Vec<(String, String, Option<Value>)>>,
}

impl FakeHttp {
    fn answering(status: u16, body: Value) -> FakeHttp {
        FakeHttp {
            status,
            body: body.to_string(),
            asked: RefCell::new(Vec::new()),
        }
    }

    fn only_request(&self) -> (String, String, Option<Value>) {
        let asked = self.asked.borrow();
        assert_eq!(asked.len(), 1);
        asked[0].clone()
    }
}

impl Http for FakeHttp {
    fn send(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
    ) -> Result<HttpResponse, String> {
        self.asked
            .borrow_mut()
            .push((method.to_string(), path.to_string(), body.cloned()));
        Ok(HttpResponse {
            status: self.status,
            body: self.body.clone(),
        })
    }
}

fn item_json(id: &str, body: &str) -> Value {
    json!({
        "id": id, "pageId": "p", "parentItemId": null, "dirPath": "", "name": "n", "type": "doc",
        "createdDate": 0, "modifiedDate": 0, "idx": "a", "level": 1, "body": body, "ast": "",
        "collapsed": false, "types": [], "properties": {}, "transcludeItemId": null,
    })
}

// 項目の作成は、位置をサーバーの形にして送り、返された項目を答える。
#[test]
fn creating_an_item_posts_its_position_and_answers_the_item() {
    let http = FakeHttp::answering(200, json!({ "item": item_json("new", "本文") }));
    let output = call(
        &http,
        ApiOp::CreateItem {
            page_id: "p".into(),
            position: ItemPosition::after("a"),
            level: 2,
            body: "本文".into(),
            id: None,
        },
    )
    .unwrap();

    let (method, path, body) = http.only_request();
    assert_eq!((method.as_str(), path.as_str()), ("POST", "/api/items"));
    assert_eq!(
        body.unwrap(),
        json!({ "pageId": "p", "position": "after:a", "level": 2, "body": "本文", "id": "" })
    );
    let ApiOutput::Item(item) = output else {
        panic!("not an item: {output:?}");
    };
    assert_eq!(item.body, "本文");
}

// 更新は、変えるフィールドだけを送る。
#[test]
fn an_update_sends_only_the_fields_that_change() {
    let http = FakeHttp::answering(200, json!({ "item": item_json("a", "x") }));
    call(
        &http,
        ApiOp::UpdateItem {
            item_id: "a".into(),
            body: Some("x".into()),
            level: None,
            collapsed: None,
        },
    )
    .unwrap();

    let (method, path, body) = http.only_request();
    assert_eq!((method.as_str(), path.as_str()), ("PUT", "/api/items/a"));
    assert_eq!(body.unwrap(), json!({ "body": "x" }));
}

// 検索語はエンコードされ、スペースや日本語を含んでもURLが壊れない。
#[test]
fn a_search_query_is_encoded_into_the_url() {
    let http = FakeHttp::answering(200, json!({ "results": [] }));
    call(
        &http,
        ApiOp::SearchCombined {
            query: "a b&日".into(),
            max_results: 20,
        },
    )
    .unwrap();

    let (_, path, _) = http.only_request();
    assert_eq!(
        path,
        "/api/search/combined?q=a%20b%26%E6%97%A5&maxResults=20"
    );
}

// 本文のない成功は、Doneを答える。
#[test]
fn a_success_without_a_body_answers_done() {
    let http = FakeHttp {
        status: 204,
        body: String::new(),
        asked: RefCell::new(Vec::new()),
    };
    let output = call(
        &http,
        ApiOp::DeleteItems {
            item_ids: vec!["a".into()],
        },
    )
    .unwrap();
    assert_eq!(output, ApiOutput::Done);
}

// サーバーのエラーは、そのメッセージをcoreへ渡す。
#[test]
fn a_server_error_carries_its_message() {
    let http = FakeHttp::answering(400, json!({ "error": "Level must be 1" }));
    let result = call(
        &http,
        ApiOp::DeleteItem {
            item_id: "a".into(),
        },
    );
    assert_eq!(result, Err(ApiError("Level must be 1".into())));
}

// メッセージのないエラーは、HTTPの状態を伝える。
#[test]
fn an_error_without_a_message_names_the_status() {
    let http = FakeHttp {
        status: 500,
        body: "oops".into(),
        asked: RefCell::new(Vec::new()),
    };
    let result = call(
        &http,
        ApiOp::DeleteItem {
            item_id: "a".into(),
        },
    );
    assert_eq!(result, Err(ApiError("HTTP 500".into())));
}

// 名前の重複（409）はエラーではなく、coreが利用者に尋ねるための答えになる。
#[test]
fn a_taken_page_name_is_an_answer_not_an_error() {
    let http = FakeHttp::answering(409, json!({ "error": "taken: x" }));
    let output = call(
        &http,
        ApiOp::UpdatePage {
            page_id: "p".into(),
            title: Some("x".into()),
            dir_path: None,
            overwrite: false,
        },
    )
    .unwrap();
    assert_eq!(output, ApiOutput::PageNameTaken("taken: x".into()));
}

// つながらないときは、その理由がエラーになる。
#[test]
fn a_transport_failure_is_an_error() {
    struct Down;
    impl Http for Down {
        fn send(&self, _: &str, _: &str, _: Option<&Value>) -> Result<HttpResponse, String> {
            Err("connection refused".into())
        }
    }
    assert_eq!(
        call(&Down, ApiOp::GetConfig),
        Err(ApiError("connection refused".into()))
    );
}

// Emacsにはファイルのハンドルがないので、アップロードは失敗として答える。
#[test]
fn an_upload_is_refused_because_emacs_holds_no_files() {
    let http = FakeHttp::answering(200, json!({}));
    assert!(call(&http, ApiOp::UploadAsset { file: 1 }).is_err());
    assert!(http.asked.borrow().is_empty());
}
