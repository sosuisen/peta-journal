//! The header button that opens the folder of the tag pages.

use crate::app::Event;
use crate::harness::Harness;

// タグボタンはタグフォルダの index ページを開く。
#[test]
fn the_tags_button_opens_the_index_page_of_the_tags_folder() {
    let mut h = Harness::new();
    h.model.config.tags_dir = "tags/".into();
    h.send(Event::NavigateToTags);

    assert_eq!(h.navigations.last().unwrap().url, "/pages/tags/index");
}

// タグがルートにあるときは、タグボタンはルートの index を開く。
#[test]
fn with_the_tags_in_the_root_the_tags_button_opens_the_root_index() {
    let mut h = Harness::new();
    h.model.config.tags_dir = String::new();
    h.send(Event::NavigateToTags);

    assert_eq!(h.navigations.last().unwrap().url, "/pages/index");
}
