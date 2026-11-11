//! The outline of an atlas page. The server writes the whole outline of `index_atlas`: a card
//! for each page of its directory, the link cards, the groups of cards and the callouts. Each
//! is an item; what kind of item it is, where it lies and how its title or bubble sits are
//! properties of the item.
//!
//! The outline goes in this order:
//! 1. the groups, by their titles, the untitled ones last, each with its cards under it
//! 2. the cards in no group
//! 3. the parent of the callouts, with the callouts under it
//!
//! Cards go folders first, then by the name they show.

use std::collections::{BTreeMap, BTreeSet};

use hmtid::Hmtid;

use super::model::{Item, Page};

pub use peta_atlas::{
    card_label, link_body, link_target, Kind, BOUNDS_KEY, CALLOUTS_BODY, PAGE_KEY, SIDES, SIDE_KEY,
    TYPE_KEY,
};
use peta_atlas::{AtlasItem, CALLOUTS_TYPE, CALLOUT_TYPE, GROUP_TYPE};

/// A group needs at least this many cards; one with fewer is no group.
const MIN_GROUP_CARDS: usize = 2;

/// One item of the atlas outline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub id: Hmtid,
    pub body: String,
    pub kind: Kind,
    pub side: Option<String>,
    pub bounds: Option<String>,
}

impl Node {
    fn new(id: Hmtid, body: String, kind: Kind) -> Node {
        Node {
            id,
            body,
            kind,
            side: None,
            bounds: None,
        }
    }

    /// The properties the item of the node has: its kind, its side and its bounds.
    pub fn properties(&self) -> BTreeMap<&'static str, String> {
        let mut props = BTreeMap::new();
        match &self.kind {
            Kind::Page(page_id) => {
                props.insert(PAGE_KEY, page_id.clone());
            }
            Kind::Link => {}
            Kind::Group => {
                props.insert(TYPE_KEY, GROUP_TYPE.to_string());
            }
            Kind::Callouts => {
                props.insert(TYPE_KEY, CALLOUTS_TYPE.to_string());
            }
            Kind::Callout => {
                props.insert(TYPE_KEY, CALLOUT_TYPE.to_string());
            }
        }
        if let Some(side) = &self.side {
            props.insert(SIDE_KEY, side.clone());
        }
        if let Some(bounds) = &self.bounds {
            props.insert(BOUNDS_KEY, bounds.clone());
        }
        props
    }
}

/// A group with its cards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub node: Node,
    pub cards: Vec<Node>,
}

/// One item as the outline writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub level: i32,
    pub node: Node,
}

/// The atlas outline, read from the items of the page and written back as rows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outline {
    pub groups: Vec<Group>,
    /// The cards in no group
    pub cards: Vec<Node>,
    pub callouts_parent: Option<Node>,
    pub callouts: Vec<Node>,
}

/// An id that the atlas and a name of what it is for make, the same on every device. It takes
/// the time of the atlas id and a hash of both for the random part.
pub fn derived_id(atlas: &Hmtid, name: &str) -> Hmtid {
    let time = atlas.timestamp_digits();
    let hash = format!("{}\0{name}", atlas.canonical())
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ byte as u64).wrapping_mul(0x0000_0100_0000_01b3)
        });
    let random: String = (0..7)
        .map(|i| hmtid::ENCODING[((hash >> (i * 5)) & 31) as usize] as char)
        .collect();
    Hmtid::parse(&format!(
        "{}-{}-{}-{}-{}-{}-{random}",
        &time[0..4],
        &time[4..6],
        &time[6..8],
        &time[8..10],
        &time[10..12],
        &time[12..14]
    ))
    .expect("the time of a valid id and the digits of the encoding make a valid id")
}

/// The id of the card of a page on an atlas.
pub fn page_card_id(atlas: &Hmtid, page_id: &str) -> Hmtid {
    derived_id(atlas, &format!("card {page_id}"))
}

/// Whether a page shows as a card on the atlas of the directory: a page of the directory, or
/// the index page of a folder right in it.
pub fn is_in_dir(page: &Page, dir: &str) -> bool {
    if page.page_type != "doc" || page.is_atlas() {
        return false;
    }
    if page.name != "index" {
        return page.dir_path == dir;
    }
    let inner = page.dir_path.trim_end_matches('/');
    !inner.is_empty()
        && match inner.rfind('/') {
            Some(slash) => &page.dir_path[..=slash] == dir,
            None => dir.is_empty(),
        }
}

/// How a card sorts: folders first, then by the name it shows, then by the whole path.
fn card_order(card: &Node) -> (bool, String, String) {
    let path = link_target(&card.body).unwrap_or(&card.body).to_string();
    let label = card_label(&path);
    (!label.ends_with('/'), label, path)
}

/// A side as stored: one of [`SIDES`], or the first.
pub fn side_or_default(side: &str) -> String {
    peta_atlas::side_or_default(side).to_string()
}

impl Outline {
    /// Reads the outline from the items of the atlas page, in their order, with their scalar
    /// properties by item id. A card at level 2 or deeper belongs to the group above it.
    /// An item of no kind is left out, and so is a second card of the same page.
    pub fn read(items: &[Item], props: &BTreeMap<String, BTreeMap<String, String>>) -> Outline {
        let empty = BTreeMap::new();
        let items: Vec<WithProps> = items
            .iter()
            .map(|item| WithProps {
                item,
                props: props.get(item.id.as_str()).unwrap_or(&empty),
            })
            .collect();
        let mut outline = Outline::default();
        let mut pages = BTreeSet::new();
        // The group of the outline that each group item became
        let mut group_of: Vec<(*const WithProps, usize)> = Vec::new();
        for entry in peta_atlas::entries(&items) {
            let node = node_of(entry.item, entry.kind);
            if let Kind::Page(page_id) = &node.kind {
                if !pages.insert(page_id.clone()) {
                    continue;
                }
            }
            match node.kind {
                Kind::Group if entry.item.level() == 1 => {
                    outline.groups.push(Group {
                        node,
                        cards: Vec::new(),
                    });
                    group_of.push((entry.item, outline.groups.len() - 1));
                }
                Kind::Group => {}
                Kind::Callouts => {
                    if outline.callouts_parent.is_none() {
                        outline.callouts_parent = Some(node);
                    }
                }
                Kind::Callout => outline.callouts.push(node),
                Kind::Page(_) | Kind::Link => {
                    let group = entry.group.and_then(|g| {
                        group_of
                            .iter()
                            .find(|(item, _)| std::ptr::eq(*item, g))
                            .map(|(_, index)| *index)
                    });
                    match group {
                        Some(group) => outline.groups[group].cards.push(node),
                        None => outline.cards.push(node),
                    }
                }
            }
        }
        outline
    }

    /// Makes the outline fit the pages of its directory: each page has one card, which links
    /// to the page by its current path. A group with fewer than two cards goes, and its card
    /// goes out of it; the callouts have a parent while there is a callout.
    ///
    /// The card of a page that is not in the directory stays, unless `moved_away` says the
    /// page is in another one. Another device may have sent the atlas before the page, and
    /// the place of the card would be lost. A deleted page takes its card with it through
    /// [`Outline::forget`].
    ///
    /// The items made here take ids from the atlas and the page, as [`derived_id`] makes
    /// them. Two devices then write the same outline from the same pages.
    pub fn fit(&mut self, atlas: &Hmtid, pages: &[Page], moved_away: impl Fn(&str) -> bool) {
        let by_id: BTreeMap<String, &Page> = pages.iter().map(|p| (p.id.to_string(), p)).collect();
        let mut carded = BTreeSet::new();
        let mut fit_card = |card: &mut Node| -> bool {
            let Kind::Page(page_id) = &card.kind else {
                return true;
            };
            let page_id = page_id.clone();
            card.id = page_card_id(atlas, &page_id);
            match by_id.get(&page_id) {
                Some(page) => {
                    card.body = link_body(&page.title());
                    carded.insert(page_id);
                    true
                }
                None => !moved_away(&page_id),
            }
        };
        for group in &mut self.groups {
            group.cards.retain_mut(&mut fit_card);
        }
        self.cards.retain_mut(&mut fit_card);
        for page in pages {
            let page_id = page.id.to_string();
            if !carded.contains(&page_id) {
                self.cards.push(Node::new(
                    page_card_id(atlas, &page_id),
                    link_body(&page.title()),
                    Kind::Page(page_id),
                ));
            }
        }
        self.drop_small_groups();
        if self.callouts.is_empty() {
            self.callouts_parent = None;
        } else {
            let id = derived_id(atlas, CALLOUTS_TYPE);
            match &mut self.callouts_parent {
                Some(parent) => parent.id = id,
                None => {
                    self.callouts_parent =
                        Some(Node::new(id, CALLOUTS_BODY.to_string(), Kind::Callouts))
                }
            }
        }
    }

    /// Takes away the card of a deleted page. True when the outline had one.
    pub fn forget(&mut self, page_id: &str) -> bool {
        let is_card = |card: &Node| matches!(&card.kind, Kind::Page(id) if id == page_id);
        let before = self.cards.len() + self.groups.iter().map(|g| g.cards.len()).sum::<usize>();
        self.cards.retain(|c| !is_card(c));
        for group in &mut self.groups {
            group.cards.retain(|c| !is_card(c));
        }
        let after = self.cards.len() + self.groups.iter().map(|g| g.cards.len()).sum::<usize>();
        after < before
    }

    fn drop_small_groups(&mut self) {
        let (kept, dropped): (Vec<Group>, Vec<Group>) = std::mem::take(&mut self.groups)
            .into_iter()
            .partition(|g| g.cards.len() >= MIN_GROUP_CARDS);
        self.groups = kept;
        for group in dropped {
            self.cards.extend(group.cards);
        }
    }

    /// The items of the outline in their order, each with its level.
    pub fn rows(&self) -> Vec<Row> {
        let row = |level: i32, node: &Node| Row {
            level,
            node: node.clone(),
        };
        let sorted = |cards: &[Node]| {
            let mut cards = cards.to_vec();
            cards.sort_by_cached_key(card_order);
            cards
        };
        let mut groups: Vec<&Group> = self.groups.iter().collect();
        groups.sort_by_cached_key(|g| {
            let title = g.node.body.trim().to_string();
            (title.is_empty(), title, g.node.id.to_string())
        });
        let mut rows = Vec::new();
        for group in groups {
            rows.push(row(1, &group.node));
            rows.extend(sorted(&group.cards).iter().map(|c| row(2, c)));
        }
        rows.extend(sorted(&self.cards).iter().map(|c| row(1, c)));
        if let Some(parent) = &self.callouts_parent {
            rows.push(row(1, parent));
            rows.extend(self.callouts.iter().map(|c| row(2, c)));
        }
        rows
    }

    fn take_card(&mut self, id: &Hmtid) -> Option<Node> {
        if let Some(i) = self.cards.iter().position(|c| c.id == *id) {
            return Some(self.cards.remove(i));
        }
        self.groups.iter_mut().find_map(|g| {
            let i = g.cards.iter().position(|c| c.id == *id)?;
            Some(g.cards.remove(i))
        })
    }

    fn card_mut(&mut self, id: &Hmtid) -> Option<&mut Node> {
        self.cards
            .iter_mut()
            .chain(self.groups.iter_mut().flat_map(|g| g.cards.iter_mut()))
            .find(|c| c.id == *id)
    }

    /// The id of the card of a page, or of a card by its own id.
    pub fn card_id(&self, id: &str) -> Option<Hmtid> {
        self.cards
            .iter()
            .chain(self.groups.iter().flat_map(|g| g.cards.iter()))
            .find(|c| c.id.as_str() == id || c.kind == Kind::Page(id.to_string()))
            .map(|c| c.id.clone())
    }

    /// Puts cards into the group of the target card. A target in no group starts a new one
    /// with them. A card is in one group only, so the cards leave the groups they were in.
    pub fn group(&mut self, moving: &[Hmtid], target: &Hmtid, new_id: impl FnOnce() -> Hmtid) {
        let moving: Vec<&Hmtid> = moving.iter().filter(|id| *id != target).collect();
        if moving.is_empty() || self.card_mut(target).is_none() {
            return;
        }
        let taken: Vec<Node> = moving.iter().filter_map(|id| self.take_card(id)).collect();
        let index = match self
            .groups
            .iter()
            .position(|g| g.cards.iter().any(|c| c.id == *target))
        {
            Some(index) => index,
            None => {
                let target = self.take_card(target).expect("the target is a card");
                let mut node = Node::new(new_id(), String::new(), Kind::Group);
                node.side = Some(SIDES[0].to_string());
                self.groups.push(Group {
                    node,
                    cards: vec![target],
                });
                self.groups.len() - 1
            }
        };
        self.groups[index].cards.extend(taken);
        self.drop_small_groups();
    }

    /// Takes cards out of their groups.
    pub fn ungroup(&mut self, ids: &[Hmtid]) {
        for group in &mut self.groups {
            let (out, kept): (Vec<Node>, Vec<Node>) = std::mem::take(&mut group.cards)
                .into_iter()
                .partition(|c| ids.contains(&c.id));
            group.cards = kept;
            self.cards.extend(out);
        }
        self.drop_small_groups();
    }

    /// Gives a group a title along a side of its outline. An empty title leaves it untitled.
    /// False when there is no such group.
    pub fn set_group_title(&mut self, id: &Hmtid, side: &str, title: &str) -> bool {
        let Some(group) = self.groups.iter_mut().find(|g| g.node.id == *id) else {
            return false;
        };
        group.node.body = title.trim().to_string();
        group.node.side = Some(side_or_default(side));
        true
    }

    /// Adds a link card in no group.
    pub fn add_link(&mut self, id: Hmtid, target: &str, bounds: String) {
        let mut node = Node::new(id, link_body(target), Kind::Link);
        node.bounds = Some(bounds);
        self.cards.push(node);
    }

    /// Links a link card to another page. False when there is no such link card.
    pub fn set_link(&mut self, id: &Hmtid, target: &str) -> bool {
        match self.card_mut(id).filter(|c| c.kind == Kind::Link) {
            Some(card) => {
                card.body = link_body(target);
                true
            }
            None => false,
        }
    }

    /// Adds a callout.
    pub fn add_callout(&mut self, id: Hmtid, bounds: String, side: &str, text: &str) {
        let mut node = Node::new(id, text.trim().to_string(), Kind::Callout);
        node.side = Some(side_or_default(side));
        node.bounds = Some(bounds);
        self.callouts.push(node);
    }

    /// Changes the text and the side of a callout. An empty text takes the callout away.
    /// False when there is no such callout.
    pub fn set_callout(&mut self, id: &Hmtid, side: &str, text: &str) -> bool {
        let Some(index) = self.callouts.iter().position(|c| c.id == *id) else {
            return false;
        };
        let text = text.trim();
        if text.is_empty() {
            self.callouts.remove(index);
        } else {
            let callout = &mut self.callouts[index];
            callout.body = text.to_string();
            callout.side = Some(side_or_default(side));
        }
        true
    }

    /// Takes a link card or a callout away. A page card stays while its page is in the
    /// directory. False when there is no such item.
    pub fn remove(&mut self, id: &Hmtid) -> bool {
        if let Some(index) = self.callouts.iter().position(|c| c.id == *id) {
            self.callouts.remove(index);
            return true;
        }
        let is_link = self
            .card_mut(id)
            .is_some_and(|card| card.kind == Kind::Link);
        if is_link {
            self.take_card(id);
            self.drop_small_groups();
        }
        is_link
    }

    /// Places a card, as the move of an old atlas brings the place of a page card along.
    pub fn set_bounds(&mut self, id: &Hmtid, bounds: String) {
        if let Some(card) = self.card_mut(id) {
            card.bounds = Some(bounds);
        }
    }
}

/// An item with its scalar properties, as the reader of the outline takes it.
struct WithProps<'a> {
    item: &'a Item,
    props: &'a BTreeMap<String, String>,
}

impl AtlasItem for WithProps<'_> {
    fn level(&self) -> i32 {
        self.item.level
    }

    fn body(&self) -> &str {
        &self.item.body
    }

    fn prop(&self, key: &str) -> Option<&str> {
        self.props.get(key).map(String::as_str)
    }
}

/// The node of an item of a kind, with the properties that kind keeps.
fn node_of(item: &WithProps, kind: Kind) -> Node {
    let side = match kind {
        Kind::Group | Kind::Callout => Some(side_or_default(item.prop(SIDE_KEY).unwrap_or(""))),
        _ => None,
    };
    let bounds = match kind {
        Kind::Page(_) | Kind::Link | Kind::Callout => item.prop(BOUNDS_KEY).map(str::to_string),
        _ => None,
    };
    Node {
        id: item.item.id.clone(),
        body: item.item.body.clone(),
        kind,
        side,
        bounds,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u32) -> Hmtid {
        // Ids in the order of `n`, as the tests read them
        let (h, m, s) = (n / 3600, n / 60 % 60, n % 60);
        Hmtid::parse(&format!("2026-09-29-{h:02}-{m:02}-{s:02}-AAAAAAA")).unwrap()
    }

    fn page(n: u32, dir: &str, name: &str) -> Page {
        Page::new(id(n), dir, name, "doc", 0)
    }

    fn item(n: u32, level: i32, body: &str) -> Item {
        Item {
            id: id(n),
            page_id: id(9999),
            parent_item_id: None,
            dir_path: "d/".to_string(),
            name: "index_atlas".to_string(),
            item_type: "doc".to_string(),
            created_date: 0,
            modified_date: 0,
            idx: format!("{n:04}"),
            level,
            body: body.to_string(),
            ast: String::new(),
            collapsed: false,
            types: Vec::new(),
            transclude_item_id: None,
        }
    }

    type Props = BTreeMap<String, BTreeMap<String, String>>;

    fn props(entries: &[(u32, &str, &str)]) -> Props {
        let mut props = Props::new();
        for (n, key, value) in entries {
            props
                .entry(id(*n).to_string())
                .or_default()
                .insert(key.to_string(), value.to_string());
        }
        props
    }

    fn atlas() -> Hmtid {
        id(9999)
    }

    fn outline_of(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|r| format!("{}:{}", r.level, r.node.body))
            .collect()
    }

    // カードはフォルダが先に、次に名前順に並ぶ。
    #[test]
    fn the_cards_go_folders_first_and_then_by_name() {
        let pages = [
            page(1, "d/", "zeta"),
            page(2, "d/b/", "index"),
            page(3, "d/", "alpha"),
            page(4, "d/a/", "index"),
        ];
        let mut outline = Outline::default();

        outline.fit(&atlas(), &pages, |_| false);

        assert_eq!(
            outline_of(&outline.rows()),
            [
                "1:[[d/a/index]]",
                "1:[[d/b/index]]",
                "1:[[d/alpha]]",
                "1:[[d/zeta]]"
            ]
        );
    }

    // リンクカードは、表示する名前でページのカードの間に並ぶ。
    #[test]
    fn a_link_card_sorts_among_the_page_cards_by_the_name_it_shows() {
        let items = [item(10, 1, "[[other/beta]]"), item(11, 1, "[[x/y/index]]")];
        let pages = [page(1, "d/", "alpha"), page(2, "d/", "gamma")];
        let mut outline = Outline::read(&items, &Props::new());

        outline.fit(&atlas(), &pages, |_| false);

        assert_eq!(
            outline_of(&outline.rows()),
            [
                "1:[[x/y/index]]",
                "1:[[d/alpha]]",
                "1:[[other/beta]]",
                "1:[[d/gamma]]"
            ]
        );
    }

    // ページのカードはIDでページに追随し、位置を保つ。
    #[test]
    fn a_page_card_follows_its_page_by_id_and_keeps_its_place() {
        let items = [item(10, 1, "[[d/old]]")];
        let props = props(&[(10, PAGE_KEY, id(1).as_str()), (10, BOUNDS_KEY, "1,2,0,40")]);
        let mut outline = Outline::read(&items, &props);

        outline.fit(&atlas(), &[page(1, "d/", "new")], |_| false);

        let rows = outline.rows();
        assert_eq!(outline_of(&rows), ["1:[[d/new]]"]);
        assert_eq!(rows[0].node.bounds.as_deref(), Some("1,2,0,40"));
    }

    // ページのカードのIDは、アトラスとページから決まる。
    #[test]
    fn a_page_card_takes_an_id_from_the_atlas_and_the_page() {
        let items = [item(10, 1, "[[d/a]]")];
        let props = props(&[(10, PAGE_KEY, id(1).as_str())]);
        let mut kept = Outline::read(&items, &props);
        let mut made = Outline::default();

        kept.fit(&atlas(), &[page(1, "d/", "a")], |_| false);
        made.fit(&atlas(), &[page(1, "d/", "a")], |_| false);

        // Two devices make the same card, and a card of old takes the same id
        assert_eq!(kept.rows(), made.rows());
        assert_eq!(
            made.rows()[0].node.id,
            page_card_id(&atlas(), id(1).as_str())
        );
        // Another atlas gives the page another card
        assert_ne!(
            page_card_id(&id(9998), id(1).as_str()),
            made.rows()[0].node.id
        );
    }

    // コールアウトの親のIDは、アトラスから決まる。
    #[test]
    fn the_parent_of_the_callouts_takes_an_id_from_the_atlas() {
        let mut outline = Outline::default();
        outline.add_callout(id(50), "0,0,0,40".to_string(), "top", "hi");

        outline.fit(&atlas(), &[], |_| false);

        assert_eq!(
            outline.rows()[0].node.id,
            derived_id(&atlas(), CALLOUTS_TYPE)
        );
    }

    // 導出したIDは、アトラスと同じ時刻を持ち、アトラスと名前で決まる。
    #[test]
    fn a_derived_id_takes_the_time_of_the_atlas_and_is_decided_by_the_atlas_and_the_name() {
        let made = derived_id(&atlas(), "card x");

        assert_eq!(made.timestamp_digits(), atlas().timestamp_digits());
        assert_eq!(made, derived_id(&atlas(), "card x"));
        assert_ne!(made, derived_id(&atlas(), "card y"));
    }

    // 別のディレクトリへ移動したページのカードは消え、種類のない項目も消える。
    #[test]
    fn the_card_of_a_page_moved_to_another_directory_goes_and_an_item_of_no_kind_goes() {
        let items = [
            item(10, 1, "[[d/gone]]"),
            item(11, 1, "some text"),
            item(12, 1, ""),
        ];
        let props = props(&[(10, PAGE_KEY, id(1).as_str())]);
        let mut outline = Outline::read(&items, &props);

        outline.fit(&atlas(), &[], |page_id| page_id == id(1).as_str());

        assert!(outline.rows().is_empty());
    }

    // まだ知らないページのカードは、位置とともに残る。
    #[test]
    fn the_card_of_a_page_not_known_yet_stays_with_its_place() {
        // Another device sent the atlas before the page
        let items = [item(10, 1, "[[d/coming]]")];
        let props = props(&[(10, PAGE_KEY, id(1).as_str()), (10, BOUNDS_KEY, "1,2,0,40")]);
        let mut outline = Outline::read(&items, &props);

        outline.fit(&atlas(), &[], |_| false);
        assert_eq!(outline_of(&outline.rows()), ["1:[[d/coming]]"]);

        outline.fit(&atlas(), &[page(1, "d/", "coming")], |_| false);
        let rows = outline.rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].node.bounds.as_deref(), Some("1,2,0,40"));
    }

    // 削除したページのカードは消える。
    #[test]
    fn a_deleted_page_takes_its_card_with_it() {
        let items = [
            item(10, 1, "Tools"),
            item(11, 2, "[[d/a]]"),
            item(12, 2, "[[d/b]]"),
        ];
        let props = props(&[
            (10, TYPE_KEY, GROUP_TYPE),
            (11, PAGE_KEY, id(1).as_str()),
            (12, PAGE_KEY, id(2).as_str()),
        ]);
        let mut outline = Outline::read(&items, &props);

        assert!(outline.forget(id(2).as_str()));
        assert!(!outline.forget(id(3).as_str()));
        outline.fit(&atlas(), &[page(1, "d/", "a")], |_| false);

        assert_eq!(outline_of(&outline.rows()), ["1:[[d/a]]"]);
    }

    // ページがディレクトリにあるかは、パスで決まる。
    #[test]
    fn a_page_is_in_a_directory_by_its_path() {
        assert!(is_in_dir(&page(1, "d/", "a"), "d/"));
        assert!(is_in_dir(&page(1, "d/sub/", "index"), "d/"));
        assert!(is_in_dir(&page(1, "top/", "index"), ""));
        assert!(!is_in_dir(&page(1, "d/", "index"), "d/"));
        assert!(!is_in_dir(&page(1, "", "index"), ""));
        assert!(!is_in_dir(&page(1, "d/", "index_atlas"), "d/"));
        assert!(!is_in_dir(&page(1, "e/", "a"), "d/"));
    }

    // グループはタイトル順に先頭に並び、タイトルのないグループはその最後に来る。
    #[test]
    fn groups_come_first_by_title_with_the_untitled_last() {
        let items = [
            item(10, 1, ""),
            item(11, 2, "[[d/a]]"),
            item(12, 2, "[[d/b]]"),
            item(13, 1, "Tools"),
            item(14, 2, "[[d/d]]"),
            item(15, 2, "[[d/c]]"),
            item(16, 1, "[[d/e]]"),
        ];
        let mut entries = vec![(10, TYPE_KEY, GROUP_TYPE), (13, TYPE_KEY, GROUP_TYPE)];
        let page_ids: Vec<String> = (1..=5).map(|n| id(n).to_string()).collect();
        for (n, page_id) in (11..=12).chain(14..=16).zip(&page_ids) {
            entries.push((n, PAGE_KEY, page_id));
        }
        let mut outline = Outline::read(&items, &props(&entries));
        let pages: Vec<Page> = ["a", "b", "d", "c", "e"]
            .iter()
            .enumerate()
            .map(|(i, name)| page(i as u32 + 1, "d/", name))
            .collect();

        outline.fit(&atlas(), &pages, |_| false);

        assert_eq!(
            outline_of(&outline.rows()),
            [
                "1:Tools",
                "2:[[d/c]]",
                "2:[[d/d]]",
                "1:",
                "2:[[d/a]]",
                "2:[[d/b]]",
                "1:[[d/e]]"
            ]
        );
    }

    // カードが1枚だけ残ったグループは消える。
    #[test]
    fn a_group_left_with_one_card_goes() {
        let items = [
            item(10, 1, "Tools"),
            item(11, 2, "[[d/a]]"),
            item(12, 2, "[[d/gone]]"),
        ];
        let props = props(&[
            (10, TYPE_KEY, GROUP_TYPE),
            (11, PAGE_KEY, id(1).as_str()),
            (12, PAGE_KEY, id(2).as_str()),
        ]);
        let mut outline = Outline::read(&items, &props);

        outline.fit(&atlas(), &[page(1, "d/", "a")], |page_id| {
            page_id == id(2).as_str()
        });

        assert_eq!(outline_of(&outline.rows()), ["1:[[d/a]]"]);
    }

    // コールアウトは、親の下に元の順で末尾に置かれる。
    #[test]
    fn the_callouts_go_last_under_their_parent_in_their_order() {
        let items = [
            item(10, 1, CALLOUTS_BODY),
            item(11, 2, "hello"),
            item(12, 2, "bye"),
            item(13, 1, "[[x/link]]"),
        ];
        let props = props(&[
            (10, TYPE_KEY, CALLOUTS_TYPE),
            (11, TYPE_KEY, CALLOUT_TYPE),
            (12, TYPE_KEY, CALLOUT_TYPE),
        ]);
        let mut outline = Outline::read(&items, &props);

        outline.fit(&atlas(), &[], |_| false);

        assert_eq!(
            outline_of(&outline.rows()),
            ["1:[[x/link]]", "1:callout", "2:hello", "2:bye"]
        );
    }

    // 最後のコールアウトが消えると、その親も消える。
    #[test]
    fn the_parent_of_the_callouts_goes_with_the_last_callout() {
        let items = [item(10, 1, CALLOUTS_BODY), item(11, 2, "hello")];
        let props = props(&[(10, TYPE_KEY, CALLOUTS_TYPE), (11, TYPE_KEY, CALLOUT_TYPE)]);
        let mut outline = Outline::read(&items, &props);

        assert!(outline.set_callout(&id(11), "top", " "));
        outline.fit(&atlas(), &[], |_| false);

        assert!(outline.rows().is_empty());
    }

    // グループにないカードに落としたカードは、両方で1つのグループを作る。
    #[test]
    fn a_card_dropped_on_a_card_in_no_group_makes_a_group_of_both() {
        let mut outline = Outline::default();
        outline.fit(&atlas(), &[page(1, "d/", "a"), page(2, "d/", "b")], |_| {
            false
        });
        outline.add_link(id(50), "x/link", "0,0,0,40".to_string());
        let a = outline.card_id(id(1).as_str()).unwrap();

        outline.group(&[id(50)], &a, || id(60));

        let rows = outline.rows();
        assert_eq!(
            outline_of(&rows),
            ["1:", "2:[[d/a]]", "2:[[x/link]]", "1:[[d/b]]"]
        );
        assert_eq!(rows[0].node.id, id(60));
    }

    // カードは落とした先のグループに入り、元のグループを抜ける。
    #[test]
    fn a_card_joins_the_group_of_the_target_and_leaves_its_own() {
        let items = [
            item(10, 1, "One"),
            item(11, 2, "[[x/a]]"),
            item(12, 2, "[[x/b]]"),
            item(13, 1, "Two"),
            item(14, 2, "[[x/c]]"),
            item(15, 2, "[[x/d]]"),
        ];
        let props = props(&[(10, TYPE_KEY, GROUP_TYPE), (13, TYPE_KEY, GROUP_TYPE)]);
        let mut outline = Outline::read(&items, &props);

        outline.group(&[id(11)], &id(14), || id(60));

        assert_eq!(
            outline_of(&outline.rows()),
            ["1:Two", "2:[[x/a]]", "2:[[x/c]]", "2:[[x/d]]", "1:[[x/b]]"]
        );
    }

    // 2枚のグループからカードを出すと、グループは終わる。
    #[test]
    fn a_card_taken_out_of_a_group_of_two_ends_the_group() {
        let items = [
            item(10, 1, "One"),
            item(11, 2, "[[x/a]]"),
            item(12, 2, "[[x/b]]"),
        ];
        let props = props(&[(10, TYPE_KEY, GROUP_TYPE)]);
        let mut outline = Outline::read(&items, &props);

        outline.ungroup(&[id(11)]);

        assert_eq!(outline_of(&outline.rows()), ["1:[[x/a]]", "1:[[x/b]]"]);
    }

    // グループのタイトルは辺を持ち、知らない辺は上辺になる。
    #[test]
    fn a_group_title_takes_its_side_and_a_side_not_known_is_the_top() {
        let items = [
            item(10, 1, ""),
            item(11, 2, "[[x/a]]"),
            item(12, 2, "[[x/b]]"),
        ];
        let props = props(&[(10, TYPE_KEY, GROUP_TYPE)]);
        let mut outline = Outline::read(&items, &props);

        assert!(outline.set_group_title(&id(10), "left", " Tools "));
        assert_eq!(outline.rows()[0].node.body, "Tools");
        assert_eq!(outline.rows()[0].node.side.as_deref(), Some("left"));

        assert!(outline.set_group_title(&id(10), "middle", "Tools"));
        assert_eq!(outline.rows()[0].node.side.as_deref(), Some("top"));
    }

    // ページのカードは削除できないが、リンクカードは削除できる。
    #[test]
    fn a_page_card_cannot_be_removed_but_a_link_card_can() {
        let mut outline = Outline::default();
        outline.fit(&atlas(), &[page(1, "d/", "a")], |_| false);
        outline.add_link(id(50), "x/link", "0,0,0,40".to_string());
        let a = outline.card_id(id(1).as_str()).unwrap();

        assert!(!outline.remove(&a));
        assert!(outline.remove(&id(50)));
        assert_eq!(outline_of(&outline.rows()), ["1:[[d/a]]"]);
    }
}
