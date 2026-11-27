//! Reads the outline of an atlas page that is a plain page (ADR 053): anybody may write it,
//! in the outline or in the `.md`, so what an item is comes from its body, its place in the
//! tree and its `atlas_type`, and not from marks the server once left on it.
//!
//! Only what the screen of today can show is read. A card is an item whose body is one wiki
//! link, a group is an item of level 1 with items under it, and a callout is an item whose
//! `atlas_type` says so. Any other item is left out.

use crate::{card_label, link_target, AtlasItem, CALLOUT_TYPE, TYPE_KEY};

/// What an item of a plain atlas page is drawn as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// An item whose body is one wiki link and nothing else. [`link_target`] names its page.
    Card,
    /// An item of level 1 with items under it. Its body is the title.
    Group,
    /// An item whose `atlas_type` is `callout`, at any level. Its body is the text.
    Callout,
}

/// An item of the atlas with what it is and, for a card, the group item it is under.
#[derive(Debug)]
pub struct Entry<'a, T> {
    pub item: &'a T,
    pub kind: Kind,
    pub group: Option<&'a T>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::item;
    use crate::{BOUNDS_KEY, SIDE_KEY};

    fn read(items: &[crate::tests::Item]) -> Vec<(&str, Kind, Option<&str>)> {
        entries(items)
            .iter()
            .map(|e| (e.item.body(), e.kind, e.group.map(|g| g.body())))
            .collect()
    }

    // 本文が1つのwiki linkだけの項目は、atlas_pageがなくてもカードである。
    #[test]
    fn an_item_whose_body_is_one_wiki_link_is_a_card() {
        let items = [
            item(1, "[[d/a]]", &[(BOUNDS_KEY, "0,0,0,40")]),
            item(1, " [[x/b]] ", &[]),
        ];

        assert_eq!(
            read(&items),
            [
                ("[[d/a]]", Kind::Card, None),
                (" [[x/b]] ", Kind::Card, None)
            ]
        );
    }

    // 子を持つレベル1の項目は、atlas_typeがなくてもグループであり、その下のカードが属する。
    #[test]
    fn an_item_of_level_1_with_items_under_it_is_a_group() {
        let items = [
            item(1, "Tools", &[(SIDE_KEY, "left")]),
            item(2, "[[d/a]]", &[]),
            item(2, "[[x/b]]", &[]),
            item(1, "", &[]),
            item(2, "[[d/c]]", &[]),
        ];

        assert_eq!(
            read(&items),
            [
                ("Tools", Kind::Group, None),
                ("[[d/a]]", Kind::Card, Some("Tools")),
                ("[[x/b]]", Kind::Card, Some("Tools")),
                ("", Kind::Group, None),
                ("[[d/c]]", Kind::Card, Some("")),
            ]
        );
    }

    // レベル1のカードは、どのグループにも属さない。前のグループはそこで終わる。
    #[test]
    fn a_card_of_level_1_is_in_no_group() {
        let items = [
            item(1, "G", &[]),
            item(2, "[[d/a]]", &[]),
            item(1, "[[d/b]]", &[]),
        ];

        assert_eq!(read(&items)[2], ("[[d/b]]", Kind::Card, None));
    }

    // atlas_typeがcalloutの項目は、レベル1でもコールアウトである。子があってもグループにはならない。
    #[test]
    fn an_item_of_the_callout_type_is_a_callout_at_any_level() {
        let callout = [(TYPE_KEY, "callout")];
        let items = [
            item(1, "Hi", &callout),
            item(1, "Parent", &callout),
            item(2, "Deep", &callout),
        ];

        assert_eq!(
            read(&items),
            [
                ("Hi", Kind::Callout, None),
                ("Parent", Kind::Callout, None),
                ("Deep", Kind::Callout, None),
            ]
        );
    }

    // 今の画面で表せない項目は、読み飛ばされる。子のないテキストの項目、テキストのないコールアウト、知らない種類の項目である。
    #[test]
    fn an_item_the_screen_cannot_show_is_left_out() {
        let items = [
            item(1, "plain text", &[]),
            item(1, " ", &[(TYPE_KEY, "callout")]),
            item(1, "[[d/a]] and more", &[]),
            item(1, "[[d/b]]", &[(TYPE_KEY, "other")]),
        ];

        assert_eq!(read(&items), []);
    }

    // レベル3より深いカードは、いちばん上の親のグループに属する。入れ子のblobは、まだ描かない。
    #[test]
    fn a_deeper_card_belongs_to_the_group_of_level_1() {
        let items = [
            item(1, "G", &[]),
            item(2, "inner", &[]),
            item(3, "[[d/a]]", &[]),
        ];

        assert_eq!(
            read(&items),
            [("G", Kind::Group, None), ("[[d/a]]", Kind::Card, Some("G"))]
        );
    }

    // コールアウトの下のカードは、どのグループにも属さない。
    #[test]
    fn a_card_under_a_callout_is_in_no_group() {
        let items = [
            item(1, "G", &[]),
            item(2, "[[d/a]]", &[]),
            item(1, "Hi", &[(TYPE_KEY, "callout")]),
            item(2, "[[d/b]]", &[]),
        ];

        assert_eq!(read(&items)[3], ("[[d/b]]", Kind::Card, None));
    }

    // グループのタイトルは本文である。本文がwiki linkだけなら、そのカードのラベルである。空ならタイトルはない。
    #[test]
    fn the_title_of_a_group_is_its_body_or_the_label_of_its_link() {
        let title = |body: &str| group_title(&item(1, body, &[]));

        assert_eq!(title(" Tools ").as_deref(), Some("Tools"));
        assert_eq!(title("[[d/e/page]]").as_deref(), Some("page"));
        assert_eq!(title(" "), None);
    }
}
