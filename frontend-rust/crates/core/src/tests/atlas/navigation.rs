//! Opening an atlas: the map button, the up button and the view a folder opens at.

use super::*;

// インデックスページのマップボタンは、そのディレクトリのアトラスを開く。
#[test]
fn the_map_button_of_an_index_page_opens_the_atlas_of_its_directory() {
    let mut h = showing(doc("a/b/", "index"));
    h.send(Event::ToggleAtlas);

    assert_eq!(h.navigations.last().unwrap().url, "/pages/a/b/index_atlas");
}

// ルートのインデックスページのマップボタンは、ルートのアトラスを開く。
#[test]
fn the_map_button_of_the_root_index_opens_the_root_atlas() {
    let mut h = showing(doc("", "index"));
    h.send(Event::ToggleAtlas);

    assert_eq!(h.navigations.last().unwrap().url, "/pages/index_atlas");
}

// アトラスページのマップボタンは、インデックスページに戻る。
#[test]
fn the_map_button_of_an_atlas_page_goes_back_to_the_index_page() {
    let mut h = showing(doc("a/b/", ATLAS_NAME));
    h.send(Event::ToggleAtlas);

    assert_eq!(h.navigations.last().unwrap().url, "/pages/a/b/index");
}

// アトラスページのタイトルは、そのディレクトリ名から付けられる。ルートのアトラスでは "(root)/" と表される。
#[test]
fn an_atlas_page_is_titled_by_its_directory() {
    assert_eq!(page_title(&doc("a/b/", ATLAS_NAME)), "Atlas of a/b/");
    assert_eq!(page_title(&doc("", ATLAS_NAME)), "Atlas of (root)/");
}

fn index_with_atlas() -> Harness {
    let mut h = showing(doc("a/b/", "index"));
    h.api.answer_next(
        "GetChildPages",
        Ok(ApiOutput::Pages(vec![doc("a/b/", ATLAS_NAME)])),
    );
    h
}

/// The demotions sent, each the page and whether it may overwrite
fn demotions(h: &Harness) -> Vec<(String, bool)> {
    h.api
        .calls
        .iter()
        .filter_map(|op| match op {
            ApiOp::DemotePage { page_id, overwrite } => Some((page_id.clone(), *overwrite)),
            _ => None,
        })
        .collect()
}

// アトラスを持つインデックスページの降格では、まず確認が求められ、承諾後に降格が1つの要求で送られる。
// アトラスの削除と改名は別には送らない。降格したページが開く。
#[test]
fn demoting_an_index_page_with_an_atlas_asks_first_then_sends_one_request() {
    let mut h = index_with_atlas();
    h.confirm = true;
    h.send(Event::DemoteIndex);

    assert!(h.ui.iter().any(|op| matches!(op, UiOp::Confirm(_))));
    assert_eq!(demotions(&h), [("a/b/index".to_string(), false)]);
    assert!(!h.api.called("DeletePage"));
    assert!(!h.api.called("UpdatePage"));
    assert!(h.api.called("GetRecentPages"));
    assert_eq!(h.navigations.last().unwrap().url, "/pages/a/b");
}

// 確認に「いいえ」と答えると、降格は送られない。
#[test]
fn a_no_to_the_question_keeps_the_atlas_and_the_index_page() {
    let mut h = index_with_atlas();
    h.confirm = false;
    h.send(Event::DemoteIndex);

    assert!(demotions(&h).is_empty());
    assert!(h.navigations.is_empty());
}

// アトラスを持たないインデックスページは、確認なしに降格が送られる。
#[test]
fn an_index_page_without_an_atlas_is_demoted_without_a_question() {
    let mut h = showing(doc("a/b/", "index"));
    h.send(Event::DemoteIndex);

    assert!(!h.ui.iter().any(|op| matches!(op, UiOp::Confirm(_))));
    assert_eq!(demotions(&h), [("a/b/index".to_string(), false)]);
}

// 降格先の名前を別のページが持っていれば、上書きを確認し、承諾後に上書きの降格を送る。
#[test]
fn a_demotion_to_a_taken_name_asks_and_then_overwrites() {
    let mut h = showing(doc("a/b/", "index"));
    h.confirm = true;
    h.api
        .answer_next("DemotePage", Ok(ApiOutput::PageNameTaken("taken".into())));
    h.send(Event::DemoteIndex);

    assert!(h.ui.iter().any(
        |op| matches!(op, UiOp::Confirm(m) if m.starts_with("A page named \"a/b\" already exists"))
    ));
    assert_eq!(
        demotions(&h),
        [
            ("a/b/index".to_string(), false),
            ("a/b/index".to_string(), true)
        ]
    );
    assert_eq!(h.navigations.last().unwrap().url, "/pages/a/b");
}

// サーバーが降格を断ると、理由が出て、ページは移らない。
#[test]
fn a_refused_demotion_is_told_and_the_page_stays() {
    let mut h = showing(doc("a/b/", "index"));
    h.api.fail_next("DemotePage", "has child pages");
    h.send(Event::DemoteIndex);

    assert!(h
        .ui
        .iter()
        .any(|op| matches!(op, UiOp::Alert(m) if m == "has child pages")));
    assert!(h.navigations.is_empty());
}

// アトラスの上へボタンは、一つ上のディレクトリのアトラスを開く。
#[test]
fn the_up_button_of_an_atlas_opens_the_atlas_of_the_directory_above() {
    let mut h = showing(doc("a/b/", ATLAS_NAME));
    h.send(Event::AtlasUp);

    assert_eq!(h.navigations.last().unwrap().url, "/pages/a/index_atlas");
}

// 最上位ディレクトリの上へボタンは、ルートのアトラスを開く。
#[test]
fn the_up_button_of_a_top_directory_opens_the_root_atlas() {
    let mut h = showing(doc("a/", ATLAS_NAME));
    h.send(Event::AtlasUp);

    assert_eq!(h.navigations.last().unwrap().url, "/pages/index_atlas");
}

// ルートのアトラスには上のディレクトリがなく、上へボタンでは遷移しない。
#[test]
fn the_root_atlas_has_no_directory_above() {
    let mut h = showing(doc("", ATLAS_NAME));
    h.send(Event::AtlasUp);

    assert!(h.navigations.is_empty());
}

// 上へボタンには一つ上のディレクトリ名が表示される。最上位ディレクトリでは "(root)/" である。
#[test]
fn the_up_button_names_the_directory_above() {
    use crate::screens::atlas::parent_dir_label;
    assert_eq!(parent_dir_label("a/b/"), "a/");
    assert_eq!(parent_dir_label("a/b/c/"), "a/b/");
    assert_eq!(parent_dir_label("a/"), "(root)/");
}

fn visit(h: &mut Harness, name: &str) {
    h.send(Event::Route {
        route: crate::app::Route::Page {
            name: name.into(),
            fragment: None,
        },
        today: "2031-01-05".into(),
    });
}

fn go_to(h: &mut Harness, path: &str) -> String {
    h.send(Event::GoToPage {
        path: path.into(),
        new_tab: false,
        new_window: false,
    });
    h.navigations.last().unwrap().url.clone()
}

// 最後にマップとして見たフォルダは、そのアトラスとして開かれる。他のフォルダはインデックスページのまま開かれる。
#[test]
fn a_folder_last_seen_as_a_map_opens_as_its_atlas() {
    let mut h = Harness::new();
    visit(&mut h, "a/b/index_atlas");

    assert_eq!(go_to(&mut h, "a/b/index"), "/pages/a/b/index_atlas");
    // Another folder keeps its index page
    assert_eq!(go_to(&mut h, "a/c/index"), "/pages/a/c/index");
}

// 最後にインデックスとして見たフォルダは、再びインデックスとして開かれる。
#[test]
fn a_folder_last_seen_as_its_index_opens_as_its_index_again() {
    let mut h = Harness::new();
    visit(&mut h, "a/b/index_atlas");
    visit(&mut h, "a/b/index");

    assert_eq!(go_to(&mut h, "a/b/index"), "/pages/a/b/index");
}

// ルートとタグフォルダも、最後に見た形で開かれる。
#[test]
fn the_root_and_the_tags_folder_open_as_they_were_last_seen() {
    let mut h = Harness::new();
    h.model.config.tags_dir = "tags/".into();
    visit(&mut h, "index_atlas");
    visit(&mut h, "tags/index_atlas");

    h.send(Event::NavigateToRoot);
    assert_eq!(h.navigations.last().unwrap().url, "/pages/index_atlas");
    h.send(Event::NavigateToTags);
    assert_eq!(h.navigations.last().unwrap().url, "/pages/tags/index_atlas");
}

// フォルダをマップとして見た後でも、マップボタンはインデックスページに戻る。
#[test]
fn the_map_button_still_goes_back_to_the_index_page() {
    let mut h = Harness::new();
    visit(&mut h, "a/b/index_atlas");
    h.model.screen = Screen::Page(PageState {
        page_name: "a/b/index_atlas".into(),
        page: Some(doc("a/b/", ATLAS_NAME)),
        ..PageState::default()
    });
    h.send(Event::ToggleAtlas);

    assert_eq!(h.navigations.last().unwrap().url, "/pages/a/b/index");
}

// マップとして見たフォルダの一覧はシェルに保存され、復元されるとそのフォルダはアトラスとして開かれる。
#[test]
fn the_folders_seen_as_maps_are_kept_by_the_shell_and_restored() {
    let mut h = Harness::new();
    visit(&mut h, "a/b/index_atlas");
    assert!(h
        .ui
        .iter()
        .any(|op| matches!(op, UiOp::SaveAtlasDirs(dirs) if dirs == &vec!["a/b/".to_string()])));
    visit(&mut h, "a/b/index");
    assert!(h
        .ui
        .iter()
        .any(|op| matches!(op, UiOp::SaveAtlasDirs(dirs) if dirs.is_empty())));

    let mut h = Harness::new();
    h.send(Event::AtlasDirsRestored(vec!["a/b/".into()]));
    assert_eq!(go_to(&mut h, "a/b/index"), "/pages/a/b/index_atlas");
}
