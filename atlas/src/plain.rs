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

/// The items of a plain atlas page that the screen can show, in their order. A card below
/// level 1 belongs to the group of level 1 above it, however deep it is: a blob inside a blob
/// is not drawn yet.
pub fn entries<'a, T: AtlasItem + 'a>(items: impl IntoIterator<Item = &'a T>) -> Vec<Entry<'a, T>> {
    let mut open_group: Option<&'a T> = None;
    let mut entries = Vec::new();
    let mut items = items.into_iter().peekable();
    while let Some(item) = items.next() {
        let is_parent = items.peek().is_some_and(|next| next.level() > item.level());
        if item.level() <= 1 {
            open_group = None;
        }
        let kind = if item.prop(TYPE_KEY) == Some(CALLOUT_TYPE) {
            // A callout says its text; without one there is nothing to draw
            if item.body().trim().is_empty() {
                continue;
            }
            Kind::Callout
        } else if item.level() <= 1 && is_parent {
            open_group = Some(item);
            Kind::Group
        } else if link_target(item.body()).is_some() {
            Kind::Card
        } else {
            continue;
        };
        let group = (kind == Kind::Card).then_some(open_group).flatten();
        entries.push(Entry { item, kind, group });
    }
    entries
}

/// The title of a group item; none while its body is empty. A group whose body is one wiki
/// link is titled as the card of that link is named, as its brackets are no title.
pub fn group_title<T: AtlasItem>(group: &T) -> Option<String> {
    let text = group.body().trim();
    match link_target(text) {
        Some(target) => Some(card_label(target)),
        None => (!text.is_empty()).then(|| text.to_string()),
    }
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

    // 今の画面で表せない項目は、読み飛ばされる。子のないテキストの項目、テキストのないコールアウト、wiki linkのほかにも文字のある項目である。
    #[test]
    fn an_item_the_screen_cannot_show_is_left_out() {
        let items = [
            item(1, "plain text", &[]),
            item(1, " ", &[(TYPE_KEY, "callout")]),
            item(1, "[[d/a]] and more", &[]),
        ];

        assert_eq!(read(&items), []);
    }

    // 知らないatlas_typeの項目は、種類のない項目と同じに読まれる。移行の前のgroupも同じである。
    #[test]
    fn an_item_of_an_unknown_type_is_read_as_one_of_no_type() {
        let items = [
            item(1, "G", &[(TYPE_KEY, "group")]),
            item(2, "[[d/a]]", &[(TYPE_KEY, "other")]),
        ];

        assert_eq!(
            read(&items),
            [("G", Kind::Group, None), ("[[d/a]]", Kind::Card, Some("G"))]
        );
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
