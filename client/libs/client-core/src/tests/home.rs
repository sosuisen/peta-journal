//! Where the Home button goes.

use crate::app::Event;
use crate::harness::Harness;

// ホームリンクがなければ、ホームは今日のページを開く。
#[test]
fn home_opens_today_without_a_link() {
    let mut h = Harness::new();
    h.send(Event::NavigateHome);
    assert_eq!(h.navigations.last().unwrap().url, "/today");
}

// ホームはホームリンクのページを開く。
#[test]
fn home_opens_the_page_of_its_link() {
    let mut h = Harness::new();
    h.model.config.home_link = "sosuisen.net/article/Foo".into();
    h.send(Event::NavigateHome);
    assert_eq!(
        h.navigations.last().unwrap().url,
        "/pages/sosuisen.net/article/Foo"
    );
}

// ホームはフォルダを最後に見たときの表示で開く。
#[test]
fn home_opens_a_folder_as_it_was_last_seen() {
    let mut h = Harness::new();
    h.model.sidebar.atlas_dirs.insert("a/".into());
    h.model.config.home_link = "a/index".into();
    h.send(Event::NavigateHome);
    assert_eq!(h.navigations.last().unwrap().url, "/pages/a/index_atlas");
}
