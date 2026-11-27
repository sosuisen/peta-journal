//! The scene of an atlas as `atlas.js` reads it: the cards, with the link cards among them, and
//! the callouts. Each card carries its marks as SVG, so `atlas.js` draws no shape of its own
//! from a name.

use serde::Serialize;

use crate::marks::{
    callout_face_svg, group_color, hidden_blob_svg, link_blob_svg, tag_blob_svg, tag_icon_svg,
};
use crate::{card_label, Bounds};

/// The size of the icon of a card, in pixels of the canvas.
pub const ICON_SIZE: i32 = 48;
/// The size the icon is drawn at before it is scaled down to the card. The hatching of
/// picturesque-marks has a fixed size made for an icon of about 100px; drawn small, a shape
/// would hold only a few of its dots or lines.
pub const ICON_DRAW_SIZE: i32 = 100;
/// The size the character of a callout is drawn at, as the icons are
pub const CALLOUT_DRAW_SIZE: i32 = 100;

/// A group a card is in: the id of its item, its title, if it has one, and how the title is
/// drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupRef {
    pub id: String,
    /// The text of the title and the side of the outline it lies along
    pub title: Option<(String, String)>,
    pub look: GroupLook,
}

/// How the title of a group is drawn along its outline.
#[derive(Debug, Clone, PartialEq)]
pub enum GroupLook {
    /// The text of the title, character by character along the outline
    Text,
    /// The label of a page card: a click on it opens `path`, none for a page that is not
    /// there. The icon of the tags stands before it.
    Page {
        path: Option<String>,
        icon_tags: Vec<String>,
    },
    /// The character of a callout beside the outline, with the title in its bubble
    Callout,
}

/// A group of a card as `atlas.js` reads it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    /// The id of the item of the group
    pub id: String,
    pub color: String,
    pub title: Option<String>,
    pub title_side: Option<String>,
    /// The page a click on the title opens; none for a title that opens nothing
    pub path: Option<String>,
    /// The icon of the page before the title, as SVG
    pub icon: Option<String>,
    /// The character of a callout that stands in place of the title, as SVG of `draw_size`
    pub character: Option<String>,
    pub draw_size: i32,
}

impl Group {
    fn of(group: &GroupRef) -> Group {
        let title = group.title.as_ref();
        let (path, icon) = match &group.look {
            GroupLook::Page { path, icon_tags } => (
                path.clone(),
                (!icon_tags.is_empty()).then(|| tag_icon_svg(icon_tags, ICON_DRAW_SIZE)),
            ),
            _ => (None, None),
        };
        Group {
            id: group.id.clone(),
            color: group_color(&group.id),
            title: title.map(|(text, _)| text.clone()),
            title_side: title.map(|(_, side)| side.clone()),
            path,
            icon,
            character: (group.look == GroupLook::Callout)
                .then(|| title.map(|(text, _)| callout_face_svg(text, CALLOUT_DRAW_SIZE)))
                .flatten(),
            draw_size: CALLOUT_DRAW_SIZE,
        }
    }
}

/// A page as its card shows it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Page<'a> {
    pub id: &'a str,
    pub label: &'a str,
    /// The path that a click on the label opens
    pub path: &'a str,
    pub is_dir: bool,
    /// The tags the icon of the card is drawn from; none for no icon
    pub icon_tags: &'a [String],
    pub published: bool,
}

/// A card as `atlas.js` reads it. `x` and `y` are its center; `width` and `height` are its
/// size as scaled, and a width of 0 leaves it to the label.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    /// The id of the page of a page card, or of the item of a link card
    pub id: String,
    pub label: String,
    /// The page that a click on the label opens; none for a card that opens nothing
    pub path: Option<String>,
    pub is_dir: bool,
    pub is_link: bool,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub scale: f64,
    /// The id of the group the card is right under, the last of `groups`
    pub group: Option<String>,
    /// The groups the card is in, the outermost first
    pub groups: Vec<Group>,
    pub icon: Option<String>,
    pub blob: String,
    pub icon_size: i32,
    pub icon_draw_size: i32,
    /// The label is hidden: the card shows its place and its blob only
    pub masked: bool,
}

impl Card {
    /// The card of a page. The icon is drawn from the icon tags of the page, and the blob from
    /// the first of them, or from the label when there are none, so every card has a blob. The
    /// blob of a published page has a line around it.
    pub fn page(page: &Page, bounds: Bounds, groups: &[GroupRef]) -> Card {
        let blob_name = page
            .icon_tags
            .first()
            .map(String::as_str)
            .unwrap_or(page.label);
        Card {
            icon: (!page.icon_tags.is_empty())
                .then(|| tag_icon_svg(page.icon_tags, ICON_DRAW_SIZE)),
            blob: tag_blob_svg(blob_name, ICON_DRAW_SIZE, page.published),
            ..Card::new(
                page.id,
                page.label,
                page.path,
                bounds,
                groups,
                page.is_dir,
                false,
            )
        }
    }

    /// A link card: named by the last part of the path it links to, its blob drawn from that
    /// path in a color of its own.
    pub fn link(item_id: &str, target: &str, bounds: Bounds, groups: &[GroupRef]) -> Card {
        Card {
            blob: link_blob_svg(target, ICON_DRAW_SIZE),
            ..Card::new(
                item_id,
                &card_label(target),
                target,
                bounds,
                groups,
                false,
                true,
            )
        }
    }

    /// The card with its label hidden, as a site shows a page it does not publish. The label,
    /// the page it opens and the icon go, so none of them reaches the scene. The card keeps its
    /// place, its group and the shape of its blob, which turns white. `label_width` is the width
    /// of the card at scale 1 that the label gave it, as [`crate::measure`] measures it, so the
    /// card keeps its size.
    pub fn hide_label(self, label_width: f64) -> Card {
        Card {
            label: String::new(),
            path: None,
            icon: None,
            width: label_width * self.scale,
            blob: hidden_blob_svg(&self.blob),
            masked: true,
            ..self
        }
    }

    fn new(
        id: &str,
        label: &str,
        path: &str,
        bounds: Bounds,
        groups: &[GroupRef],
        is_dir: bool,
        is_link: bool,
    ) -> Card {
        Card {
            id: id.to_string(),
            label: label.to_string(),
            path: Some(path.to_string()),
            is_dir,
            is_link,
            x: bounds.x,
            y: bounds.y,
            width: bounds.width,
            height: bounds.height,
            scale: bounds.scale(),
            group: groups.last().map(|g| g.id.clone()),
            groups: groups.iter().map(Group::of).collect(),
            icon: None,
            blob: String::new(),
            icon_size: ICON_SIZE,
            icon_draw_size: ICON_DRAW_SIZE,
            masked: false,
        }
    }
}

/// A callout as `atlas.js` reads it: a character at a point, with a speech bubble on one side.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Callout {
    /// The id of the item of the callout
    pub key: String,
    /// The center of the character, in the coordinates of the canvas
    pub x: f64,
    pub y: f64,
    pub scale: f64,
    /// The side of the character the bubble is on
    pub side: String,
    pub text: String,
    pub character: String,
    pub draw_size: i32,
}

impl Callout {
    pub fn new(key: &str, x: f64, y: f64, scale: f64, side: &str, text: &str) -> Callout {
        Callout {
            key: key.to_string(),
            x,
            y,
            scale,
            side: side.to_string(),
            text: text.to_string(),
            character: callout_face_svg(text, CALLOUT_DRAW_SIZE),
            draw_size: CALLOUT_DRAW_SIZE,
        }
    }
}

/// The cards as the JSON that `setCards` of `atlas.js` takes.
pub fn cards_json(cards: &[Card]) -> String {
    serde_json::to_string(cards).expect("a card has only strings, numbers and booleans")
}

/// The callouts as the JSON that `setCallouts` of `atlas.js` takes.
pub fn callouts_json(callouts: &[Callout]) -> String {
    serde_json::to_string(callouts).expect("a callout has only strings and numbers")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(x: f64, y: f64, width: f64, height: f64) -> Bounds {
        Bounds {
            x,
            y,
            width,
            height,
        }
    }

    fn page<'a>(id: &'a str, label: &'a str, tags: &'a [String], published: bool) -> Page<'a> {
        Page {
            id,
            label,
            path: "d/One",
            is_dir: false,
            icon_tags: tags,
            published,
        }
    }

    fn group() -> GroupRef {
        GroupRef {
            id: "G1".to_string(),
            title: Some(("Tools".to_string(), "left".to_string())),
            look: GroupLook::Text,
        }
    }

    // ページのカードは、場所・倍率・グループ・アイコンをatlas.jsの名前で持つ
    #[test]
    fn a_page_card_has_its_place_group_and_icon() {
        let tags = vec!["JavaFX".to_string()];
        let card = Card::page(
            &page("P1", "One", &tags, false),
            bounds(10.0, -20.0, 0.0, 60.0),
            &[group()],
        );
        let json: serde_json::Value = serde_json::from_str(&cards_json(&[card])).unwrap();
        let card = &json[0];
        assert_eq!(card["id"], "P1");
        assert_eq!(card["path"], "d/One");
        assert_eq!(card["isLink"], false);
        assert_eq!(card["x"], 10.0);
        assert_eq!(card["scale"], 1.5);
        assert_eq!(card["group"], "G1");
        assert_eq!(card["icon"], tag_icon_svg(&tags, ICON_DRAW_SIZE));
        assert_eq!(card["blob"], tag_blob_svg("JavaFX", ICON_DRAW_SIZE, false));
        assert_eq!(card["iconSize"], ICON_SIZE);
        assert_eq!(card["masked"], false);
    }

    // カードは、その上のグループをすべて、外から順に、色とタイトルとともに持つ。いちばん内のグループが、カードのグループである。
    #[test]
    fn a_card_carries_every_group_it_is_under_from_the_outermost() {
        let inner = GroupRef {
            id: "G2".to_string(),
            title: None,
            look: GroupLook::Text,
        };
        let card = Card::page(
            &page("P1", "One", &[], false),
            bounds(0.0, 0.0, 0.0, 40.0),
            &[group(), inner],
        );

        let json: serde_json::Value = serde_json::from_str(&cards_json(&[card])).unwrap();
        let groups = &json[0]["groups"];
        assert_eq!(json[0]["group"], "G2");
        assert_eq!(
            (
                &groups[0]["id"],
                &groups[0]["title"],
                &groups[0]["titleSide"]
            ),
            (
                &serde_json::Value::from("G1"),
                &serde_json::Value::from("Tools"),
                &serde_json::Value::from("left")
            )
        );
        assert_eq!(groups[0]["color"], group_color("G1"));
        assert_eq!(
            (&groups[1]["id"], &groups[1]["title"]),
            (&serde_json::Value::from("G2"), &serde_json::Value::Null)
        );
    }

    // ページカードのグループは、タイトルで開くページとアイコンを持つ。コールアウトのグループは、タイトルの文字から描いたキャラクターを持つ。
    #[test]
    fn a_group_of_a_page_has_its_page_and_icon_and_one_of_a_callout_its_character() {
        let tags = vec!["sun".to_string()];
        let of_page = GroupRef {
            id: "G1".to_string(),
            title: Some(("One".to_string(), "top".to_string())),
            look: GroupLook::Page {
                path: Some("d/One".to_string()),
                icon_tags: tags.clone(),
            },
        };
        let of_callout = GroupRef {
            id: "G2".to_string(),
            title: Some(("Hi".to_string(), "right".to_string())),
            look: GroupLook::Callout,
        };
        let card = Card::page(
            &page("P1", "Two", &[], false),
            bounds(0.0, 0.0, 0.0, 40.0),
            &[of_page, of_callout],
        );

        let [page_group, callout_group] = &card.groups[..] else {
            panic!("two groups: {:?}", card.groups);
        };
        assert_eq!(
            (&page_group.path, &page_group.icon, &page_group.character),
            (
                &Some("d/One".to_string()),
                &Some(tag_icon_svg(&tags, ICON_DRAW_SIZE)),
                &None
            )
        );
        assert_eq!(
            (
                &callout_group.path,
                &callout_group.icon,
                &callout_group.character
            ),
            (
                &None,
                &None,
                &Some(callout_face_svg("Hi", CALLOUT_DRAW_SIZE))
            )
        );
    }

    // タグのないページのカードは、アイコンがなく、blobをラベルから描く
    #[test]
    fn a_page_without_tags_has_no_icon_and_a_blob_of_its_label() {
        let card = Card::page(
            &page("P1", "One", &[], true),
            bounds(0.0, 0.0, 0.0, 40.0),
            &[],
        );
        assert_eq!(card.icon, None);
        assert_eq!(card.blob, tag_blob_svg("One", ICON_DRAW_SIZE, true));
        assert!(card.groups.is_empty());
    }

    // リンクカードは、リンク先のパスの最後をラベルにし、オレンジのblobを持つ
    #[test]
    fn a_link_card_is_named_by_its_target() {
        let card = Card::link("I1", "a/b/index", bounds(0.0, 0.0, 0.0, 40.0), &[group()]);
        assert_eq!(card.label, "b/");
        assert_eq!(card.path.as_deref(), Some("a/b/index"));
        assert!(card.is_link);
        assert_eq!(card.blob, link_blob_svg("a/b/index", ICON_DRAW_SIZE));
        assert_eq!(card.group.as_deref(), Some("G1"));
    }

    // ラベルを伏せたカードは、ラベル・リンク先・アイコンを持たず、元の幅と形のblobを白で保つ
    #[test]
    fn a_card_with_a_hidden_label_keeps_its_size_and_a_white_blob_only() {
        let tags = vec!["JavaFX".to_string()];
        let shown = Card::page(
            &page("P1", "Draft", &tags, false),
            bounds(5.0, 6.0, 0.0, 60.0),
            &[group()],
        );
        let hidden = shown.clone().hide_label(100.0);
        assert_eq!(hidden.label, "");
        assert_eq!(hidden.path, None);
        assert_eq!(hidden.icon, None);
        assert!(hidden.masked);
        // The width of the label at scale 1, scaled as the card is
        assert_eq!(hidden.width, 150.0);
        // The same shape, filled white in place of the tag color
        assert_eq!(hidden.blob, shown.blob.replace("#CFE3FF", "#FFFFFF"));
        assert_eq!(
            (hidden.x, hidden.y, hidden.group),
            (shown.x, shown.y, shown.group)
        );
    }

    // コールアウトは、テキストの顔をキャラクターとして持つ
    #[test]
    fn a_callout_has_the_face_of_its_text() {
        let callout = Callout::new("C1", 1.0, 2.0, 1.5, "top", "Hi");
        let json: serde_json::Value = serde_json::from_str(&callouts_json(&[callout])).unwrap();
        assert_eq!(json[0]["key"], "C1");
        assert_eq!(
            json[0]["character"],
            callout_face_svg("Hi", CALLOUT_DRAW_SIZE)
        );
        assert_eq!(json[0]["drawSize"], CALLOUT_DRAW_SIZE);
    }
}
