use crate::api::{call, Http};
use peta_core::effects::{ApiError, ApiOp, ApiOutput};
use peta_core::rest::{HttpReply, HttpRequest};
use std::cell::RefCell;

/// Answers every request the same way and remembers what was asked.
struct FakeHttp {
    status: u16,
    asked: RefCell<Vec<HttpRequest>>,
}

impl Http for FakeHttp {
    fn send(&self, request: &HttpRequest) -> Result<HttpReply, String> {
        self.asked.borrow_mut().push(request.clone());
        Ok(HttpReply {
            status: self.status,
            body: String::new(),
        })
    }
}

fn fake(status: u16) -> FakeHttp {
    FakeHttp {
        status,
        asked: RefCell::new(Vec::new()),
    }
}

// 操作はコアが決めたリクエストとして送られ、応答はコアの読み方で答えになる。
#[test]
fn an_operation_is_sent_and_its_reply_is_read() {
    let http = fake(204);
    let output = call(
        &http,
        ApiOp::DeleteItem {
            item_id: "a".into(),
        },
    );
    assert_eq!(output, Ok(ApiOutput::Done));
    let asked = http.asked.borrow();
    assert_eq!(asked.len(), 1);
    assert_eq!(
        (asked[0].method, asked[0].path.as_str()),
        ("DELETE", "/api/items/a")
    );
}

// つながらないときは、その理由がエラーになる。
#[test]
fn a_transport_failure_is_an_error() {
    struct Down;
    impl Http for Down {
        fn send(&self, _: &HttpRequest) -> Result<HttpReply, String> {
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
    let http = fake(200);
    assert!(call(&http, ApiOp::UploadAsset { file: 1 }).is_err());
    assert!(http.asked.borrow().is_empty());
}
