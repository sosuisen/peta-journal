use crate::api::{call, upload, Http};
use peta_core::effects::{ApiError, ApiOp, ApiOutput};
use peta_core::rest::{HttpReply, HttpRequest};
use std::cell::RefCell;
use std::path::PathBuf;

/// Answers every request the same way and remembers what was asked.
struct FakeHttp {
    status: u16,
    body: String,
    asked: RefCell<Vec<HttpRequest>>,
    uploaded: RefCell<Vec<(String, Vec<u8>)>>,
}

impl FakeHttp {
    fn reply(&self) -> HttpReply {
        HttpReply {
            status: self.status,
            body: self.body.clone(),
        }
    }
}

impl Http for FakeHttp {
    fn send(&self, request: &HttpRequest) -> Result<HttpReply, String> {
        self.asked.borrow_mut().push(request.clone());
        Ok(self.reply())
    }

    fn upload(&self, file_name: &str, bytes: &[u8]) -> Result<HttpReply, String> {
        self.uploaded
            .borrow_mut()
            .push((file_name.to_string(), bytes.to_vec()));
        Ok(self.reply())
    }
}

fn fake_with_body(status: u16, body: &str) -> FakeHttp {
    FakeHttp {
        status,
        body: body.to_string(),
        asked: RefCell::new(Vec::new()),
        uploaded: RefCell::new(Vec::new()),
    }
}

fn fake(status: u16) -> FakeHttp {
    fake_with_body(status, "")
}

/// A file with these bytes in a folder of this test. The test removes the folder.
fn file_in_folder_of(test: &str, name: &str, bytes: &[u8]) -> (PathBuf, String) {
    let folder =
        std::env::temp_dir().join(format!("peta-shell-emacs-{}-{test}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let file = folder.join(name);
    std::fs::write(&file, bytes).unwrap();
    (folder, file.to_string_lossy().into_owned())
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
        fn upload(&self, _: &str, _: &[u8]) -> Result<HttpReply, String> {
            Err("connection refused".into())
        }
    }
    assert_eq!(
        call(&Down, ApiOp::GetConfig),
        Err(ApiError("connection refused".into()))
    );
    let (folder, path) = file_in_folder_of("down", "pic.png", b"png");
    let output = upload(&Down, &path);
    std::fs::remove_dir_all(folder).unwrap();
    assert_eq!(output, Err(ApiError("connection refused".into())));
}

// アップロードはRESTの呼び出しではないので、リクエストとしては送らない。
#[test]
fn an_upload_is_not_sent_as_a_request() {
    let http = fake(200);
    assert!(call(&http, ApiOp::UploadAsset { file: 1 }).is_err());
    assert!(http.asked.borrow().is_empty());
}

// ファイルの中身が元の名前で送られ、サーバーが付けた名前が答えになる。
#[test]
fn a_file_is_uploaded_under_its_name_and_the_saved_name_is_the_answer() {
    let http = fake_with_body(200, r#"{"fileName":"pic-1.png"}"#);
    let (folder, path) = file_in_folder_of("upload", "pic.png", b"png bytes");
    let output = upload(&http, &path);
    std::fs::remove_dir_all(folder).unwrap();
    assert_eq!(output, Ok(ApiOutput::FileName("pic-1.png".into())));
    assert_eq!(
        *http.uploaded.borrow(),
        [("pic.png".to_string(), b"png bytes".to_vec())]
    );
}

// 読めないファイルは、サーバーへ何も送らずにエラーになる。
#[test]
fn a_file_that_cannot_be_read_is_an_error_and_nothing_is_sent() {
    let http = fake_with_body(200, r#"{"fileName":"pic.png"}"#);
    assert!(upload(&http, "no/such/folder/pic.png").is_err());
    assert!(http.uploaded.borrow().is_empty());
}

// サーバーが断ったら、その状態コードがエラーになる。
#[test]
fn a_refused_upload_is_an_error_with_the_status() {
    let http = fake(413);
    let (folder, path) = file_in_folder_of("refused", "big.png", b"big");
    let output = upload(&http, &path);
    std::fs::remove_dir_all(folder).unwrap();
    assert_eq!(output, Err(ApiError("Upload failed: 413".into())));
}
