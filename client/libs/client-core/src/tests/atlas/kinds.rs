//! How the items of the atlas page are drawn (ADR 053): page cards, plain cards and the
//! blobs of the parents.

use super::*;
use crate::model::PropertyValue;
use crate::rich_text::RichLink;
use crate::screens::atlas::{AtlasGroup, CardTone, GroupLook};
use crate::view::LoadState;

/// An item of level 1 placed at the origin
fn placed(id: &str, body: &str) -> Item {
    atlas_item(id, 1, body, &[("atlas_bounds", "0,0,0,40")])
}

// ほかのフォルダのページへのカードは、リンクのラベルを出す。ラベルがなければ、リンク先の名前を出す。
#[test]
fn a_card_to_a_page_of_another_folder_shows_the_label_of_its_link() {
    let h = atlas_of(
        vec![],
        vec![
            placed("a", "[[x/Target|Shown]]"),
            placed("b", "[[x/y/Other]]"),
        ],
    );

    let labels: Vec<String> = h.view().atlas_links.into_iter().map(|l| l.label).collect();
    assert_eq!(labels, ["Shown", "Other"]);
}

// 1行目が1つのwikilinkだけでない項目は、ノーマルカードである。1行目の文字を出し、リンク先を持たない。
#[test]
fn an_item_whose_first_line_is_not_one_link_is_a_plain_card_showing_its_first_line() {
    let h = atlas_of(
        vec![],
        vec![placed("p", "**Note** with [[x/A|a link]]\nmore lines")],
    );

    let card = &h.view().atlas_links[0];
    assert_eq!(
        (card.tone, card.label.as_str(), card.target.as_deref()),
        (CardTone::Plain, "Note with a link", None)
    );
}

// カードの1行目のリンクは、ページのリンクもウェブのリンクも、押せる文字の並びとして渡される。
#[test]
fn the_links_of_the_first_line_of_a_card_go_with_its_letters() {
    let h = atlas_of(
        vec![],
        vec![placed(
            "p",
            "see [[x/A|a page]] or [a site](https://example.com)\n[[x/B]]",
        )],
    );

    let links: Vec<RichLink> = h.view().atlas_links[0]
        .runs
        .iter()
        .filter_map(|run| run.link.clone())
        .collect();
    assert_eq!(
        links,
        [
            RichLink::Page {
                url: "/pages/x/A".into()
            },
            RichLink::Web {
                url: "https://example.com".into()
            },
        ]
    );
}

// カードは、2行目より後も含む本文の全体を持つ。右クリックのメニューから、全文を見せるためである。
#[test]
fn a_card_carries_the_whole_text_of_its_item() {
    let h = atlas_of(
        vec![child("one")],
        vec![
            atlas_item(
                "i-one",
                1,
                "[[a/b/one]]\nmore of one",
                &[("atlas_bounds", "0,0,0,40")],
            ),
            placed("p", "note\nmore of the note"),
        ],
    );

    let view = h.view();
    assert_eq!(
        (
            view.atlas_cards[0].text.as_str(),
            view.atlas_links[0].text.as_str()
        ),
        ("[[a/b/one]]\nmore of one", "note\nmore of the note")
    );
}

// 同じフォルダのページへの2枚目のカードも、青のページカードである。
#[test]
fn a_second_card_to_a_page_of_the_folder_is_blue_too() {
    let h = atlas_of(
        vec![child("one")],
        vec![
            placed("first", "[[a/b/one]]"),
            placed("second", "[[a/b/one|again]]"),
        ],
    );

    let link = &h.view().atlas_links[0];
    assert_eq!(
        (link.item_id.as_str(), link.tone, link.label.as_str()),
        ("second", CardTone::Page, "again")
    );
}

// 同じフォルダにないページへのカードは、フォルダの一覧が届いていれば、灰色のリンク先のないカードである。届く前は、オレンジのままである。
#[test]
fn a_card_to_a_page_the_folder_does_not_have_is_gray_once_the_list_is_in() {
    let mut h = atlas_of(vec![child("one")], vec![placed("gone", "[[a/b/gone]]")]);
    let before = h.view().atlas_links[0].tone;

    h.model.sidebar.index_state = LoadState::Loaded;

    assert_eq!(
        (before, h.view().atlas_links[0].tone),
        (CardTone::Link, CardTone::Missing)
    );
}

/// The ids of the parents and their titles
fn titled_groups(groups: &[AtlasGroup]) -> Vec<(&str, Option<&str>)> {
    groups
        .iter()
        .map(|g| (g.id.as_str(), g.title.as_ref().map(|t| t.text.as_str())))
        .collect()
}

// カードは、その上の親をすべて、外から順に、そのタイトルとともに持つ。入れ子の深さに上限はない。
#[test]
fn a_card_carries_all_its_parents_from_the_outermost() {
    let h = atlas_of(
        vec![child("one")],
        vec![
            parent_item("outer", 1, "Outer", &[]),
            parent_item("inner", 2, "", &[]),
            parent_item("deep", 3, "Deep", &[]),
            card_item("one", 4, Some("0,0,0,40")),
            atlas_item("p", 2, "note", &[("atlas_bounds", "9,0,0,40")]),
        ],
    );

    let view = h.view();
    assert_eq!(
        titled_groups(&view.atlas_cards[0].groups),
        [
            ("outer", Some("Outer")),
            ("inner", None),
            ("deep", Some("Deep"))
        ]
    );
    assert_eq!(
        titled_groups(&view.atlas_links[0].groups),
        [("outer", Some("Outer"))]
    );
}

/// An item of level 2 placed at the origin
fn placed_under(id: &str, body: &str) -> Item {
    atlas_item(id, 2, body, &[("atlas_bounds", "0,0,0,40")])
}

// 親がページカードなら、その親はページを開くパスと、アイコンのタグと、色を持つ。リンク先のページがない親は、何も開かない。
#[test]
fn a_parent_that_is_a_page_card_opens_its_page_and_has_its_icon() {
    let mut one = child("one");
    one.properties
        .insert("tags".into(), PropertyValue::List(vec!["sun".into()]));
    let mut h = atlas_of(
        vec![one],
        vec![
            parent_item("pa", 1, "[[a/b/one]]", &[]),
            placed_under("na", "note"),
            parent_item("pb", 1, "[[a/b/gone]]", &[]),
            placed_under("nb", "note"),
        ],
    );
    h.model.sidebar.index_state = LoadState::Loaded;
    h.model.config.tags_dir = "tags/".into();

    let looks: Vec<GroupLook> = h
        .view()
        .atlas_links
        .into_iter()
        .map(|l| l.groups[0].look.clone())
        .collect();
    assert_eq!(
        looks,
        [
            GroupLook::Page {
                path: Some("a/b/one".into()),
                tone: CardTone::Page,
                icon_tags: vec!["sun".into()],
            },
            GroupLook::Page {
                path: None,
                tone: CardTone::Missing,
                icon_tags: vec![],
            },
        ]
    );
}

// コールアウトの親は、グループのタイトルの代わりに、キャラクターとフキダシで描かれる。フキダシの文字は本文で、辺はatlas_sideである。ひとりで立つコールアウトには数えない。
#[test]
fn a_parent_that_is_a_callout_is_drawn_as_its_character_and_bubble() {
    let h = atlas_of(
        vec![],
        vec![
            atlas_item(
                "c",
                1,
                "Hello",
                &[
                    ("atlas_type", "callout"),
                    ("atlas_side", "right"),
                    ("atlas_bounds", "5,5,0,40"),
                ],
            ),
            placed_under("p", "note"),
        ],
    );

    let view = h.view();
    let group = &view.atlas_links[0].groups[0];
    assert_eq!(
        (&group.look, &group.title),
        (
            &GroupLook::Callout,
            &Some(GroupTitle {
                side: "right".into(),
                text: "Hello".into()
            })
        )
    );
    assert!(view.atlas_callouts.is_empty());
}

// ページの一覧を読んだあとも、灰色になるのは、atlasのフォルダの中のページか、子のフォルダのindexだけである。ほかのフォルダのページや、子のフォルダの中のページは、オレンジのままである。
#[test]
fn only_a_page_of_the_folder_or_the_index_of_a_child_folder_is_gray_when_it_is_not_there() {
    let mut h = atlas_of(
        vec![child("one")],
        vec![
            placed("other", "[[x/gone]]"),
            placed("index", "[[a/b/sub/index]]"),
            placed("deeper", "[[a/b/sub/gone]]"),
        ],
    );
    h.model.sidebar.index_state = LoadState::Loaded;

    let tones: Vec<CardTone> = h.view().atlas_links.iter().map(|l| l.tone).collect();
    assert_eq!(tones, [CardTone::Link, CardTone::Missing, CardTone::Link]);
}

// 子を持つ項目は、置いてあっても、カードとしては描かれない。グループとして描かれる。
#[test]
fn an_item_with_items_under_it_is_no_card_even_when_placed() {
    let h = atlas_of(
        vec![],
        vec![placed("parent", "Plan"), placed_under("leaf", "note")],
    );

    let ids: Vec<String> = h
        .view()
        .atlas_links
        .iter()
        .map(|l| l.item_id.clone())
        .collect();
    assert_eq!(ids, ["leaf"]);
}
