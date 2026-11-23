//! What the canvas of an atlas in the web shell draws: the cards and the callouts as the JSON
//! that `atlas.js` reads, made by the scene of peta-atlas as the Leptos shell made them.

/// The placed cards, the link cards among them, as `setCards` of `atlas.js` takes them. The
/// cards and the links are given as the JSON of the border.
pub fn placed_json(cards: &str, links: &str) -> Result<String, String> {
    Ok(String::new())
}

/// The callouts, each with its character drawn from its text, as `setCallouts` of
/// `atlas.js` takes them. The callouts are given as the JSON of the border.
pub fn callouts_json(callouts: &str) -> Result<String, String> {
    Ok(String::new())
}

/// The folder above the folder `dir_path`, as the up button of an atlas names it.
pub fn parent_dir_label(dir_path: &str) -> String {
    String::new()
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

    // 置いたカードだけを、リンクのカードとともに、グループとその題名をつけて渡す。置いていないカードは渡さない。
    #[test]
    fn the_placed_cards_go_with_the_link_cards_and_their_groups() {
        let cards = r#"[
            {"page_id":"a","label":"A","path":"m/a","is_dir":false,
             "bounds":{"x":10.0,"y":20.0,"width":0.0,"height":0.0},
             "group":"g","group_color":null,"icon_tags":["sun"],
             "group_title":{"side":"top","text":"Plan"},"published":false},
            {"page_id":"b","label":"B","path":"m/b","is_dir":true,"bounds":null,
             "group":null,"group_color":null,"icon_tags":[],"group_title":null,"published":false}
        ]"#;
        let links = r#"[
            {"item_id":"l","target":"x/Foo",
             "bounds":{"x":1.0,"y":2.0,"width":0.0,"height":0.0},
             "group":null,"group_title":null}
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
                &placed[0]["groupTitle"],
                &placed[0]["groupTitleSide"]
            ),
            (&Value::from("g"), &Value::from("Plan"), &Value::from("top"))
        );
        assert!(placed[0]["icon"].is_string());
        assert_eq!(placed[1]["label"], "Foo");
    }

    // 吹き出しは、文字から描いた顔とともに渡す。
    #[test]
    fn each_callout_goes_with_the_character_drawn_from_its_text() {
        let callouts = r#"[{"key":"k","x":1.0,"y":2.0,"scale":1.5,"side":"left","text":"hello"}]"#;

        let json = callouts_json(callouts).unwrap();
        let shown: Vec<Value> = serde_json::from_str(&json).unwrap();

        assert_eq!(
            (&shown[0]["key"], &shown[0]["scale"], &shown[0]["text"]),
            (&Value::from("k"), &Value::from(1.5), &Value::from("hello"))
        );
        assert!(shown[0]["character"].as_str().unwrap().contains("<svg"));
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
