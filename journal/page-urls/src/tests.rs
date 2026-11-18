use super::*;

// ページのURLは、/pages/の下の、符号化したページ名である。
#[test]
fn the_url_of_a_page_is_its_encoded_name_under_pages() {
    assert_eq!(page_url("What?"), "/pages/What%3F");
}
