use crate::{multipart_body, sse_data};

// イベントストリームのdata行だけが、メッセージになる。
#[test]
fn only_the_data_lines_of_an_event_stream_are_messages() {
    assert_eq!(sse_data("data: {\"a\":1}"), Some("{\"a\":1}"));
    assert_eq!(sse_data("data:{\"a\":1}"), Some("{\"a\":1}"));
    assert_eq!(sse_data(": keep-alive"), None);
    assert_eq!(sse_data("event: ping"), None);
    assert_eq!(sse_data(""), None);
}

// アップロードのフォームは、ファイルの中身を変えずに運ぶ。
#[test]
fn the_form_of_an_upload_carries_the_bytes_as_they_are() {
    let body = multipart_body("pic.png", &[0, 159, 255]);
    let head = b"filename=\"pic.png\"\r\nContent-Type: application/octet-stream\r\n\r\n";
    let start = body.windows(head.len()).position(|part| part == head);
    let bytes = start.unwrap() + head.len();
    assert_eq!(body[bytes..bytes + 5], [0, 159, 255, b'\r', b'\n']);
}

// ファイル名の引用符と改行は、フォームのヘッダーを途中で終わらせるので、_に置き換わる。
#[test]
fn a_quote_or_a_line_end_in_the_file_name_of_an_upload_becomes_an_underscore() {
    let body = multipart_body("a\"b\rc\nd.png", b"");
    let form = String::from_utf8(body).unwrap();
    assert!(form.contains("filename=\"a_b_c_d.png\"\r\n"), "{form}");
}
