use crate::Route;

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
        Route::parse("/pages/notes/rust", None),
        page("notes/rust", None)
    );
}

// /journalsと/docsも、ページ名として読まれる。
#[test]
fn journals_and_docs_paths_name_a_page_too() {
    assert_eq!(
        Route::parse("/journals/2026-11-06", None),
        page("2026-11-06", None)
    );
    assert_eq!(Route::parse("/docs/guide", None), page("guide", None));
}

// 接頭辞のないパスは、そのままページ名になる。
#[test]
fn a_path_without_a_prefix_is_the_name_of_the_page() {
    assert_eq!(Route::parse("/notes/rust", None), page("notes/rust", None));
}

// 名前の末尾のスラッシュは、名前に入らない。
#[test]
fn a_slash_at_the_end_is_not_part_of_the_name() {
    assert_eq!(Route::parse("/pages/notes/", None), page("notes", None));
}

// 項目のidは、ページと一緒に渡る。
#[test]
fn the_fragment_is_the_item_to_go_to() {
    assert_eq!(
        Route::parse("/pages/a", Some("item1")),
        page("a", Some("item1"))
    );
}

// 空の#は、移動する項目がないことと同じである。
#[test]
fn an_empty_fragment_is_no_fragment() {
    assert_eq!(Route::parse("/pages/a", Some("")), page("a", None));
}

// /todayと、名前のない/pages/は、今日のページである。
#[test]
fn today_and_a_page_path_without_a_name_open_today() {
    assert_eq!(Route::parse("/today", None), page("today", None));
    assert_eq!(Route::parse("/pages/", None), page("today", None));
}

// /settingsは設定画面である。
#[test]
fn settings_is_its_own_route() {
    assert_eq!(Route::parse("/settings", None), Some(Route::Settings));
}

// 空のパスには行き先がない。
#[test]
fn an_empty_path_has_no_route() {
    assert_eq!(Route::parse("/", None), None);
}

// 日本語のページ名は、そのまま読まれる。デコードはシェルの仕事である。
#[test]
fn a_japanese_page_name_is_read_as_it_is() {
    assert_eq!(
        Route::parse("/pages/日記/メモ", None),
        page("日記/メモ", None)
    );
}
