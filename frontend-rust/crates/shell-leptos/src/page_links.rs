//! Where a link to a page of this app leads. The shown items and the copied HTML both use
//! this, so that a link opens the same page from the app and from another app (ADR 027).

use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

/// The path of the page that a wikilink names.
pub fn wikilink_path(page: &str) -> String {
    format!("/pages/{page}")
}

/// The path of the page of a tag. A tag page lives under the configured tag directory, so
/// that it can have its own folder (e.g. `tags/topic`).
pub fn hashtag_path(tags_dir: &str, tag: &str) -> String {
    format!("/pages/{tags_dir}{tag}")
}

/// What is encoded in a path that leaves the app: the same characters as `encodeURI` of
/// JavaScript, with which the app encodes its own URLs. `#` stays, so a link to a place in a
/// page still leads there.
const OUTSIDE_A_URL: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');

/// The links to pages as another app can open them: whole URLs of this app.
///
/// Only this type knows how such a URL is made. Another form, such as a URL scheme of the
/// app, changes it here alone.
#[derive(Clone, Copy, Debug)]
pub struct AppLinks<'a> {
    /// The origin the app is open at, such as `http://localhost:8080`.
    pub app_url: &'a str,
    pub tags_dir: &'a str,
}

impl AppLinks<'_> {
    pub fn wikilink(&self, page: &str) -> String {
        self.url_of(&wikilink_path(page))
    }

    pub fn hashtag(&self, tag: &str) -> String {
        self.url_of(&hashtag_path(self.tags_dir, tag))
    }

    fn url_of(&self, path: &str) -> String {
        format!(
            "{}{}",
            self.app_url,
            utf8_percent_encode(path, OUTSIDE_A_URL)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINKS: AppLinks = AppLinks {
        app_url: "http://localhost:8080",
        tags_dir: "tags/",
    };

    // ウィキリンクのページは、/pages/ の下にある。
    #[test]
    fn the_page_of_a_wikilink_is_under_pages() {
        assert_eq!(wikilink_path("Dir/Page"), "/pages/Dir/Page");
    }

    // タグのページは、タグ用ディレクトリの下にある。ディレクトリが空なら、/pages/ の直下にある。
    #[test]
    fn the_page_of_a_tag_is_under_the_tag_directory() {
        assert_eq!(hashtag_path("tags/", "topic"), "/pages/tags/topic");
        assert_eq!(hashtag_path("", "topic"), "/pages/topic");
    }

    // ほかのアプリから開けるリンクは、アプリのURLから始まる、まるごとのURLである。
    #[test]
    fn a_link_for_another_app_is_a_whole_url_of_this_app() {
        assert_eq!(
            LINKS.wikilink("Dir/Page"),
            "http://localhost:8080/pages/Dir/Page"
        );
        assert_eq!(
            LINKS.hashtag("topic"),
            "http://localhost:8080/pages/tags/topic"
        );
    }

    // 空白や日本語を含むページ名は、1つのURLとして読めるように符号化される。
    #[test]
    fn a_page_name_with_spaces_or_japanese_is_encoded_into_one_url() {
        assert_eq!(
            LINKS.wikilink("Other Page"),
            "http://localhost:8080/pages/Other%20Page"
        );
        assert_eq!(
            LINKS.wikilink("メモ"),
            "http://localhost:8080/pages/%E3%83%A1%E3%83%A2"
        );
    }

    // ページの中の場所を指す # は、符号化されずに残る。
    #[test]
    fn the_hash_of_a_place_in_a_page_stays() {
        assert_eq!(
            LINKS.wikilink("Page#place"),
            "http://localhost:8080/pages/Page#place"
        );
    }
}
