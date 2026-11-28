//! What the canvas of an atlas in the web shell draws: the cards and the callouts as the JSON
//! that `atlas.js` reads, made by the scene of peta-atlas as the Leptos shell made them.

use client_core::rich_text::{RichLink, RichRun};
use client_core::screens::atlas::{self as core_atlas, AtlasGroup, Callout, CardTone, LinkCard};
use client_core::view::AtlasCard;
use peta_atlas::scene::{self, GroupLook, GroupRef};

/// The groups a card or a callout is in, as the scene takes them, each with the place of its
/// item counted from the group above it.
fn group_refs(groups: &[AtlasGroup]) -> Vec<GroupRef> {
    groups
        .iter()
        .map(|group| GroupRef {
            id: group.id.clone(),
            title: group
                .title
                .as_ref()
                .map(|t| (t.text.clone(), t.side.clone())),
            look: match &group.look {
                core_atlas::GroupLook::Text => GroupLook::Text,
                core_atlas::GroupLook::Page {
                    path, icon_tags, ..
                } => GroupLook::Page {
                    path: path.clone(),
                    icon_tags: icon_tags.clone(),
                },
                core_atlas::GroupLook::Callout => GroupLook::Callout,
            },
            x: group.x,
            y: group.y,
        })
        .collect()
}

/// The placed cards, the link cards among them, as `setCards` of `atlas.js` takes them. The
/// cards and the links are given as the JSON of the border.
pub fn placed_json(cards: &str, links: &str) -> Result<String, String> {
    let cards: Vec<AtlasCard> = crate::border::read(cards)?;
    let links: Vec<LinkCard> = crate::border::read(links)?;
    let mut placed: Vec<scene::Card> = cards
        .iter()
        .filter_map(|c| {
            let page = scene::Page {
                id: &c.page_id,
                label: &c.label,
                path: &c.path,
                is_dir: c.is_dir,
                icon_tags: &c.icon_tags,
                published: c.published,
            };
            let groups = group_refs(&c.groups);
            c.bounds.map(|b| scene::Card::page(&page, b, &groups))
        })
        .collect();
    placed.extend(links.iter().map(|l| {
        let groups = group_refs(&l.groups);
        match &l.target {
            Some(target) => scene::Card {
                label: l.label.clone(),
                ..scene::Card::link(&l.item_id, target, tone(l.tone), l.bounds, &groups)
            },
            None => {
                let runs = l.runs.iter().map(run).collect();
                scene::Card::plain(&l.item_id, &l.label, runs, l.bounds, &groups)
            }
        }
    }));
    Ok(scene::cards_json(&placed))
}

/// What a card links to, as the scene takes it.
fn tone(tone: CardTone) -> scene::Tone {
    match tone {
        CardTone::Page => scene::Tone::Page,
        CardTone::Link => scene::Tone::Link,
        CardTone::Missing => scene::Tone::Missing,
        CardTone::Plain => scene::Tone::Plain,
    }
}

/// A part of the first line of a card as the scene takes it.
fn run(run: &RichRun) -> scene::Run {
    scene::Run {
        text: run.text.clone(),
        link: run.link.as_ref().map(|link| match link {
            RichLink::Page { url } => scene::RunLink {
                kind: "Page".to_string(),
                url: url.clone(),
            },
            RichLink::Web { url } => scene::RunLink {
                kind: "Web".to_string(),
                url: url.clone(),
            },
            RichLink::File { path } => scene::RunLink {
                kind: "File".to_string(),
                url: path.clone(),
            },
        }),
    }
}

/// The callouts, each with its character drawn from its text and the groups it is in, as
/// `setCallouts` of `atlas.js` takes them. The callouts are given as the JSON of the border.
pub fn callouts_json(callouts: &str) -> Result<String, String> {
    let callouts: Vec<Callout> = crate::border::read(callouts)?;
    let all: Vec<scene::Callout> = callouts
        .iter()
        .map(|c| {
            scene::Callout::new(&c.key, c.x, c.y, c.scale, &c.side, &c.text)
                .under(&group_refs(&c.groups))
        })
        .collect();
    Ok(scene::callouts_json(&all))
}

/// The folder above the folder `dir_path`, as the up button of an atlas names it.
pub fn parent_dir_label(dir_path: &str) -> String {
    client_core::screens::atlas::parent_dir_label(dir_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn ids_of(json: &str) -> Vec<(String, bool)> {
        let cards: Vec<Value> = serde_json::from_str(json).unwrap();
        cards
            .iter()
            .map(|c| {
                (
                    c["id"].as_str().unwrap().to_string(),
                    c["isLink"].as_bool().unwrap(),
                )
            })
            .collect()
    }

    // 置いたカードだけを、リンクのカードとともに、グループとその題名と位置をつけて渡す。置いていないカードは渡さない。
    #[test]
    fn the_placed_cards_go_with_the_link_cards_and_their_groups() {
        let cards = r#"[
            {"page_id":"a","label":"A","path":"m/a","is_dir":false,
             "bounds":{"x":10.0,"y":20.0,"width":0.0,"height":0.0},
             "icon_tags":["sun"],
             "groups":[{"id":"g","title":{"side":"top","text":"Plan"},"look":"Text","x":100.0,"y":200.0}],"published":false,"text":""},
            {"page_id":"b","label":"B","path":"m/b","is_dir":true,"bounds":null,
             "icon_tags":[],"groups":[],"published":false,"text":""}
        ]"#;
        let links = r#"[
            {"item_id":"l","target":"x/Foo","label":"Shown","tone":"Link","runs":[],"text":"[[x/Foo|Shown]]",
             "bounds":{"x":1.0,"y":2.0,"width":0.0,"height":0.0},
             "groups":[]}
        ]"#;

        let json = placed_json(cards, links).unwrap();
        let placed: Vec<Value> = serde_json::from_str(&json).unwrap();

        assert_eq!(
            ids_of(&json),
            [("a".to_string(), false), ("l".to_string(), true)]
        );
        assert_eq!(
            (
                &placed[0]["group"],
                &placed[0]["groups"][0]["title"],
                &placed[0]["groups"][0]["titleSide"]
            ),
            (&Value::from("g"), &Value::from("Plan"), &Value::from("top"))
        );
        // The card is counted from its group, which carries its own place
        assert_eq!(
            (&placed[0]["x"], &placed[0]["groups"][0]["x"]),
            (&Value::from(10.0), &Value::from(100.0))
        );
        assert!(placed[0]["icon"].is_string());
        assert_eq!(placed[1]["label"], "Shown");
    }

    // 吹き出しは、文字から描いた顔と、属するグループとともに渡す。
    #[test]
    fn each_callout_goes_with_the_character_drawn_from_its_text() {
        let callouts = r#"[{"key":"k","x":1.0,"y":2.0,"scale":1.5,"side":"left","text":"hello",
            "groups":[{"id":"g","title":null,"look":"Text","x":100.0,"y":200.0}]}]"#;

        let json = callouts_json(callouts).unwrap();
        let shown: Vec<Value> = serde_json::from_str(&json).unwrap();

        assert_eq!(
            (&shown[0]["key"], &shown[0]["scale"], &shown[0]["text"]),
            (&Value::from("k"), &Value::from(1.5), &Value::from("hello"))
        );
        assert!(shown[0]["character"].as_str().unwrap().contains("<svg"));
        assert_eq!(
            (&shown[0]["groups"][0]["id"], &shown[0]["groups"][0]["y"]),
            (&Value::from("g"), &Value::from(200.0))
        );
    }

    // 読めないカードや吹き出しは、理由とともに断る。
    #[test]
    fn unreadable_cards_and_callouts_are_refused() {
        assert!(placed_json("[{", "[]").is_err());
        assert!(placed_json("[]", "[{").is_err());
        assert!(callouts_json("[{").is_err());
    }

    // 上のボタンは、ひとつ上のフォルダを名乗る。一番上なら(root)/である。
    #[test]
    fn the_up_button_names_the_folder_above() {
        assert_eq!(parent_dir_label("a/b/"), "a/");
        assert_eq!(parent_dir_label("a/"), "(root)/");
    }
}
