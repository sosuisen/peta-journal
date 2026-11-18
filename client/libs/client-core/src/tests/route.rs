use crate::Route;

const ITEM: &str = "20260426035700_QZ998PX";

fn page(name: &str, fragment: Option<&str>) -> Option<Route> {
    Some(Route::Page {
        name: name.to_string(),
        fragment: fragment.map(str::to_string),
    })
}

// /pages/以下のパスは、そのページ名になる。ディレクトリも名前に含まれる。
#[test]
fn a_pages_path_names_the_page_with_its_directory() {
    assert_eq!(
        Route::parse_url("/pages/notes/rust"),
        page("notes/rust", None)
    );
}

// /journalsと/docsと、接頭辞のないパスは、ページを指さない。URLの形は/pages/だけである（ADR 061）。
#[test]
fn journals_docs_and_paths_without_a_prefix_name_no_page() {
    assert_eq!(Route::parse_url("/journals/2026-11-06"), None);
    assert_eq!(Route::parse_url("/docs/guide"), None);
    assert_eq!(Route::parse_url("/notes/rust"), None);
}

// 名前の末尾のスラッシュは、名前に入らない。
#[test]
fn a_slash_at_the_end_is_not_part_of_the_name() {
    assert_eq!(Route::parse_url("/pages/notes/"), page("notes", None));
}

// #の後ろは、開いた後で移動する項目のidである。
#[test]
fn what_follows_the_hash_of_a_url_is_the_item_to_go_to() {
    assert_eq!(
        Route::parse_url(&format!("/pages/a#{ITEM}")),
        page("a", Some(ITEM))
    );
}

// 空の#は、移動する項目がないことと同じである。
#[test]
fn an_empty_fragment_is_no_fragment() {
    assert_eq!(Route::parse_url("/pages/a#"), page("a", None));
}

// /todayと、名前のない/pages/は、今日のページである。
#[test]
fn today_and_a_page_path_without_a_name_open_today() {
    assert_eq!(Route::parse_url("/today"), page("today", None));
    assert_eq!(Route::parse_url("/pages/"), page("today", None));
}

// /settingsは設定画面である。
#[test]
fn settings_is_its_own_route() {
    assert_eq!(Route::parse_url("/settings"), Some(Route::Settings));
}

// 空のパスには行き先がない。
#[test]
fn an_empty_path_has_no_route() {
    assert_eq!(Route::parse_url("/"), None);
}

// 符号化したページ名は、1回だけ復号される。名前の#は%23で、項目のidの前の#と区別される。
#[test]
fn an_encoded_page_name_is_decoded_once() {
    assert_eq!(
        Route::parse_url("/pages/%E6%97%A5%E8%A8%98/C%23"),
        page("日記/C#", None)
    );
    assert_eq!(Route::parse_url("/pages/a%2520b"), page("a%20b", None));
    assert_eq!(
        Route::parse_url(&format!("/pages/C%23#{ITEM}")),
        page("C#", Some(ITEM))
    );
}

// 符号化していない日本語のページ名も、そのまま読まれる。
#[test]
fn a_japanese_page_name_that_is_not_encoded_is_read_as_it_is() {
    assert_eq!(
        Route::parse_url("/pages/日記/メモ"),
        page("日記/メモ", None)
    );
}
