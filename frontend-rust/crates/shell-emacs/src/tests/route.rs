use crate::route::parse_url;
use peta_core::Route;

fn page(name: &str, fragment: Option<&str>) -> Option<Route> {
    Some(Route::Page {
        name: name.to_string(),
        fragment: fragment.map(str::to_string),
    })
}

// /pages/以下のパスは、そのページ名になる。ディレクトリも名前に含まれる。
#[test]
fn a_pages_url_names_the_page_with_its_directory() {
    assert_eq!(parse_url("/pages/notes/rust"), page("notes/rust", None));
}

// /journalsと/docsも、ページ名として読まれる。
#[test]
fn journals_and_docs_urls_name_a_page_too() {
    assert_eq!(parse_url("/journals/2026-11-06"), page("2026-11-06", None));
    assert_eq!(parse_url("/docs/guide"), page("guide", None));
}

// #の後ろは、開いた後で移動する項目のidになる。
#[test]
fn the_fragment_is_the_item_to_go_to() {
    assert_eq!(parse_url("/pages/a#item1"), page("a", Some("item1")));
}

// /todayと、名前のない/pages/は、今日のページである。
#[test]
fn today_and_a_page_url_without_a_name_open_today() {
    assert_eq!(parse_url("/today"), page("today", None));
    assert_eq!(parse_url("/pages/"), page("today", None));
}

// /settingsは設定画面である。
#[test]
fn settings_is_its_own_route() {
    assert_eq!(parse_url("/settings"), Some(Route::Settings));
}

// 空のURLには行き先がない。
#[test]
fn an_empty_url_has_no_route() {
    assert_eq!(parse_url("/"), None);
}

// Emacsに届くURLはエンコードされていないので、日本語のページ名はそのまま読まれる。
#[test]
fn a_japanese_page_name_is_read_as_it_is() {
    assert_eq!(parse_url("/pages/日記/メモ"), page("日記/メモ", None));
}
