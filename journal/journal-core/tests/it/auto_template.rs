//! The auto-template extension of `ext-functions/` (see ADR 009): its `main.js` puts the
//! items of a template into a new page, or into a page that gets a tag. The `main.js` is the
//! real one; the templates are the ones of these tests.

use std::path::Path;

use serde_json::Value;

use crate::common::items::outline;
use crate::common::Core;

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
---
tag: #book
---
Title

<!-- @@ -->
Author
---
tag: movie
---
Director
";

/// A core with auto-template installed. Its weeks are numbered as ISO numbers them, as the
/// server of the tests numbered them.
fn template_core() -> Core {
    let notes = Core::with_extensions(|config_dir| {
        let source =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ext-functions/auto-template");
        let dir = config_dir.join("functions").join("auto-template");
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["config.json", "main.js"] {
            std::fs::copy(source.join(name), dir.join(name)).unwrap();
        }
        std::fs::write(dir.join("template.md"), TEMPLATES).unwrap();
    });
    notes.core.config.set_week_numbering("iso");
    notes
}

fn outlines(items: &Value) -> Vec<String> {
    outline(items.as_array().unwrap())
}

// 曜日に合うテンプレートの項目が、新しいジャーナルに入る。空の1項目は残らない。
#[test]
fn a_new_journal_gets_the_template_of_its_weekday() {
    let notes = template_core();

    let monday = notes.open_journal("2031-03-03");

    assert_eq!(
        outlines(&monday["items"]),
        vec!["1:### AM".to_string(), "2:Check e-mails".to_string()]
    );
    assert_eq!(monday["items"][1]["types"], serde_json::json!(["task"]));
}

// テンプレートは、名前のパターンに合うページにだけ入る。合うものがいくつもあれば、ファイルの順に続けて入る。
#[test]
fn templates_go_only_into_pages_whose_names_match_their_patterns() {
    let notes = template_core();

    let tuesday = notes.open_journal("2031-03-04");
    let week = notes.open_journal("2031-W10");
    let plan = notes.open_doc("plan-");

    assert_eq!(outlines(&tuesday["items"]), vec!["1:".to_string()]);
    // A weekly journal starts with links to its days, so the template goes after them
    assert_eq!(
        outlines(&week["items"]).last().map(String::as_str),
        Some("1:Week goals")
    );
    assert_eq!(
        outlines(&plan["items"]),
        vec!["1:Plan A".to_string(), "1:Plan B".to_string()]
    );
}

// ない週の名前（W60）のページは、ジャーナルではないので、週のテンプレートが入らない。名前は、サーバーと同じ決まりで読まれる。
#[test]
fn a_page_named_after_a_week_that_does_not_exist_gets_no_weekly_template() {
    let notes = template_core();

    let no_week = notes.open_doc("2031-W60");

    assert_eq!(outlines(&no_week["items"]), vec!["1:".to_string()]);
}

/// Adds `tag` to a page that has an item, and answers the question of auto-template with `ok`.
/// Returns the page and the question.
fn add_tag_and_answer(notes: &Core, tag: &str, ok: bool) -> (String, Value) {
    let page = notes.new_doc_page("tagged");
    notes.fill(&page, &["existing"]);

    notes.set_tags(&page, tag);
    let question = notes.question_with(&format!("#{tag}"));
    notes.answer(&question, ok);
    notes.wait_for_the_extensions(&page);
    (page, question)
}

// タグを足すと、そのタグのテンプレートを入れるかを尋ね、「はい」ならページの最後に入る。タグの前の#は無視する。
#[test]
fn a_tag_template_goes_to_the_end_of_the_page_after_a_yes() {
    let notes = template_core();

    let (page, question) = add_tag_and_answer(&notes, "book", true);

    assert_eq!(
        question["message"],
        "Insert the template of the tag \"#book\" at the end of this page?"
    );
    assert_eq!(
        notes.outline_of(&page),
        vec!["1:existing", "1:Title", "2:Author"]
    );
}

// 「いいえ」と答えると、タグのテンプレートは入らない。
#[test]
fn a_tag_template_stays_out_after_a_no() {
    let notes = template_core();

    let (page, _) = add_tag_and_answer(&notes, "movie", false);

    assert_eq!(notes.outline_of(&page), vec!["1:existing"]);
}
