//! The auto-template extension of `ext-functions/` on the server (see ADR 009): its `main.js`
//! puts the items of a template into a new page, or into a page that gets a tag.
//!
//! These tests are a binary of their own, as the extension acts on every page its templates
//! match. The `main.js` is the real one; the templates are the ones of these tests.

#[path = "../common/mod.rs"]
mod common;

use std::path::Path;

use common::*;
use serde_json::{json, Value};

/// The templates of these tests.
const TEMPLATES: &str = "---
pattern: Monday
---
### AM

<!-- @@ -->
- [ ] Check e-mails
---
pattern: weekly
---
Week goals
---
pattern: /^plan-/
---
Plan A
---
pattern: Friday, /^plan-/
---
Plan B
";

/// The server with auto-template installed.
fn template_server() -> &'static TestServer {
    server_with(|config_dir| {
        let source =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../ext-functions/auto-template");
        let dir = config_dir.join("functions").join("auto-template");
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["config.json", "main.js"] {
            std::fs::copy(source.join(name), dir.join(name)).unwrap();
        }
        std::fs::write(dir.join("template.md"), TEMPLATES).unwrap();
    })
}

fn outline(items: &Value) -> Vec<String> {
    items
        .as_array()
        .unwrap()
        .iter()
        .map(|item| ItemRow::of(item).outline())
        .collect()
}

// 曜日に合うテンプレートの項目が、新しいジャーナルに入る。空の1項目は残らない。
#[tokio::test]
async fn a_new_journal_gets_the_template_of_its_weekday() {
    template_server();

    let (status, monday) = post_empty("/api/journals/2031-03-03").await;

    assert_eq!(status, 200, "{monday}");
    assert_eq!(
        outline(&monday["items"]),
        vec!["1:### AM".to_string(), "2:Check e-mails".to_string()]
    );
    assert_eq!(
        ItemRow::of(&monday["items"][1]).types,
        vec!["task".to_string()]
    );
}

// テンプレートは、名前のパターンに合うページにだけ入る。合うものがいくつもあれば、ファイルの順に続けて入る。
#[tokio::test]
async fn templates_go_only_into_pages_whose_names_match_their_patterns() {
    template_server();

    let (_, tuesday) = post_empty("/api/journals/2031-03-04").await;
    let (_, week) = post_empty("/api/journals/2031-W10").await;
    let (_, plan) = post(
        "/api/pages/doc",
        json!({ "title": unique_name("plan-"), "dirPath": "" }),
    )
    .await;

    assert_eq!(outline(&tuesday["items"]), vec!["1:".to_string()]);
    // A weekly journal starts with links to its days, so the template goes after them
    assert_eq!(
        outline(&week["items"]).last().map(String::as_str),
        Some("1:Week goals")
    );
    assert_eq!(
        outline(&plan["items"]),
        vec!["1:Plan A".to_string(), "1:Plan B".to_string()]
    );
}
