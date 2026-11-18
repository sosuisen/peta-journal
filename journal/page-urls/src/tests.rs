use super::*;
use proptest::prelude::*;

const ITEM: &str = "20260426035700_QZ998PX";

fn address(name: &str, item: Option<&str>) -> Option<PageAddress> {
    Some(PageAddress {
        name: name.to_string(),
        item: item.map(str::to_string),
    })
}

// ページのURLは、/pages/の下の、符号化したページ名である。
#[test]
fn the_url_of_a_page_is_its_encoded_name_under_pages() {
    assert_eq!(page_url("What?"), "/pages/What%3F");
}

// 項目のURLは、ページのURLの後ろに#と項目のidを付けたものである。
#[test]
fn the_url_of_an_item_is_the_url_of_its_page_with_the_item_id_after_a_hash() {
    assert_eq!(item_url("C#", ITEM), format!("/pages/C%23#{ITEM}"));
}

// ハッシュタグのリンクは、タグのページのURLと同じである。
#[test]
fn the_link_of_a_hashtag_is_the_url_of_the_page_of_the_tag() {
    let link = flatmarkdown_ast2html::fill_href(&hashtag_href("タグ/"), "日記");
    assert_eq!(link, page_url(&tag_page_name("タグ/", "日記")));
    assert_eq!(
        page_url(&tag_page_name("tags/", "Rust")),
        "/pages/tags/Rust"
    );
}

// wikilinkの中身は、最後の#の後ろが項目のidのときだけ、ページ名と項目のidに分かれる。
#[test]
fn a_wikilink_splits_only_when_an_item_id_follows_the_last_hash() {
    assert_eq!(split_wikilink(&format!("C##{ITEM}")), ("C#", Some(ITEM)));
    assert_eq!(
        split_wikilink("Page#2026-04-26-03-57-00-QZ998PX"),
        ("Page", Some("2026-04-26-03-57-00-QZ998PX"))
    );
    assert_eq!(split_wikilink("C#入門"), ("C#入門", None));
    assert_eq!(split_wikilink("Page"), ("Page", None));
}

// URLを読むと、ページ名と項目のidが、それぞれ1回だけ復号される。
#[test]
fn reading_a_url_decodes_the_page_name_and_the_item_id_once() {
    assert_eq!(
        read_url(&format!("/pages/C%23#{ITEM}")),
        address("C#", Some(ITEM))
    );
    assert_eq!(read_url("/pages/a%2520b"), address("a%20b", None));
}

// 符号化していないURLも、そのまま読める。
#[test]
fn a_url_that_is_not_encoded_is_read_as_it_is() {
    assert_eq!(read_url("/pages/日記/メモ 1"), address("日記/メモ 1", None));
}

// 末尾の/と、空の項目のidは、読み飛ばされる。
#[test]
fn a_slash_at_the_end_and_an_empty_item_id_are_left_out() {
    assert_eq!(read_url("/pages/notes/#"), address("notes", None));
}

// /pages/の下でないURLは、ページを指さない。
#[test]
fn a_url_outside_pages_names_no_page() {
    assert_eq!(read_url("/docs/guide"), None);
    assert_eq!(read_url("/journals/2026-11-06"), None);
    assert_eq!(read_url("/today"), None);
}

/// Names of pages: no control characters, and no `/` at either end, as `page-names` asks.
fn page_name() -> impl Strategy<Value = String> {
    "[^\\x00-\\x1f/]([^\\x00-\\x1f]{0,20}[^\\x00-\\x1f/])?"
}

fn item_id() -> impl Strategy<Value = String> {
    let layout = (any::<bool>(), any::<bool>());
    (0u64..4_102_444_800_000, layout).prop_map(|(ms, (dash, separate_time))| {
        hmtid::Generator::new(if dash { '-' } else { '_' }, separate_time)
            .generate_at(ms)
            .as_str()
            .to_string()
    })
}

proptest! {
    // 作ったページのURLを読むと、元のページ名に戻る。
    #[test]
    fn reading_the_url_of_a_page_gives_back_its_name(name in page_name()) {
        prop_assert_eq!(read_url(&page_url(&name)), address(&name, None));
    }

    // 作った項目のURLを読むと、元のページ名と項目のidに戻る。
    #[test]
    fn reading_the_url_of_an_item_gives_back_its_page_name_and_id(
        name in page_name(),
        item in item_id(),
    ) {
        prop_assert_eq!(read_url(&item_url(&name, &item)), address(&name, Some(&item)));
    }

    // Hmtid::parseが通すidは、どれも項目のidの形に一致する。
    #[test]
    fn every_id_that_hmtid_accepts_is_an_item_id(item in item_id()) {
        prop_assert!(hmtid::Hmtid::parse(&item).is_ok());
        let content = format!("Page#{item}");
        prop_assert_eq!(split_wikilink(&content), ("Page", Some(item.as_str())));
    }
}
