use crate::app::{Event, Screen};
use crate::effects::{ApiOp, UiOp};
use crate::harness::Harness;
use crate::model::{PropertyDefinition, PropertyValue};
use crate::view::ScreenView;

fn page_properties(h: &Harness) -> Vec<(String, String)> {
    match &h.model.screen {
        Screen::Page(p) => p
            .page
            .as_ref()
            .map(|pg| {
                pg.properties
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_json::to_string(v).unwrap()))
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn shows_properties(h: &Harness) -> bool {
    match &h.view().screen {
        ScreenView::Page(Some(p)) => p.show_properties,
        _ => false,
    }
}

/// The rows as the shell gets them: key, value, removable, editable.
fn rows(h: &Harness) -> Vec<(String, String, bool, bool)> {
    match &h.view().screen {
        ScreenView::Page(Some(p)) => p
            .properties
            .iter()
            .map(|r| (r.key.clone(), r.value.clone(), r.removable, r.editable))
            .collect(),
        _ => Vec::new(),
    }
}

// ページの作成日時と更新日時は削除できない行として示され、プロパティ行を開かない。日付は保存されるプロパティではなく、サーバーのページとともにだけ変わる。
#[test]
fn the_dates_of_the_page_are_rows_that_cannot_be_removed_and_do_not_open_the_rows() {
    let mut h = Harness::with_items(vec![]);
    assert!(!shows_properties(&h));
    assert_eq!(
        rows(&h),
        vec![
            (
                "created".to_string(),
                "1970-01-01T00:00:00Z".to_string(),
                false,
                true
            ),
            (
                "modified".to_string(),
                "1970-01-01T00:00:00Z".to_string(),
                false,
                false
            ),
        ]
    );
    h.send(Event::SetPageProperty {
        key: "created".into(),
        value_type: "datetime".into(),
        value: "2020-01-02T03:04:05Z".into(),
    });
    // The date is not a stored property: it changes only with the server's page
    assert!(page_properties(&h).is_empty());
    assert_eq!(rows(&h)[0].1, "1970-01-01T00:00:00.042Z");
}

// ページプロパティの行は、キーの名前を、型の接尾辞を外した本体で示す。
#[test]
fn a_row_of_a_page_property_shows_the_body_of_its_key() {
    let mut h = Harness::with_items(vec![]);
    h.send(Event::SetPageProperty {
        key: "rating_n".into(),
        value_type: "number".into(),
        value: "3".into(),
    });
    let ScreenView::Page(Some(p)) = &h.view().screen else {
        panic!("no page")
    };
    let row = p.properties.iter().find(|r| r.key == "rating_n").unwrap();
    assert_eq!(row.name, "rating");
}

// ページプロパティの行は、本体がほかの定義と重なるキーの名前に、型を添えて示す。
#[test]
fn a_row_of_a_page_property_adds_the_type_to_a_body_another_key_shares() {
    let mut h = Harness::with_items(vec![]);
    h.model.config.property_definitions = ["rating_n", "rating_d"]
        .map(|key| PropertyDefinition {
            key: key.into(),
            page: true,
            ..PropertyDefinition::default()
        })
        .to_vec();
    h.send(Event::SetPageProperty {
        key: "rating_n".into(),
        value_type: "number".into(),
        value: "3".into(),
    });
    let ScreenView::Page(Some(p)) = &h.view().screen else {
        panic!("no page")
    };
    let row = p.properties.iter().find(|r| r.key == "rating_n").unwrap();
    assert_eq!(row.name, "rating (number)");
}

// ページプロパティを設定すると、それが表示され、サーバーが返したページの値が取り込まれる。
#[test]
fn setting_a_page_property_shows_it_and_takes_the_server_value() {
    let mut h = Harness::with_items(vec![]);
    assert!(!shows_properties(&h));
    h.send(Event::SetPageProperty {
        key: "owner".into(),
        value_type: "text".into(),
        value: "me".into(),
    });
    assert!(h.api.calls.iter().any(|c| matches!(c, ApiOp::UpsertPageProperty { page_id, key, value_type, value } if page_id == "page1" && key == "owner" && value_type == "text" && value == "me")));
    assert_eq!(
        page_properties(&h),
        vec![("owner".to_string(), "\"me\"".to_string())]
    );
    assert!(shows_properties(&h));
    // The server's page replaced the shown one
    if let Screen::Page(p) = &h.model.screen {
        assert_eq!(p.page.as_ref().unwrap().modified_date, 42);
    }
}

// タグ値の設定をサーバーが拒否すると、元の空のプロパティに戻り、理由が利用者に知らされる。
#[test]
fn a_refused_tag_value_puts_the_old_properties_back_and_tells_the_user() {
    let mut h = Harness::with_items(vec![]);
    h.api.fail_next("UpsertPageProperty", "offline");
    h.send(Event::SetPageProperty {
        key: "tags".into(),
        value_type: "tags".into(),
        value: "a, b,".into(),
    });
    // The refusal put the old (empty) properties back and told the user
    assert!(page_properties(&h).is_empty());
    assert!(h
        .ui
        .iter()
        .any(|u| matches!(u, UiOp::Alert(m) if m == "offline")));
}

// ページプロパティを削除すると、それは取り除かれる。最後のプロパティが消えても行は閉じたままで、メニューから開くと示される。
#[test]
fn deleting_a_page_property_removes_it() {
    let mut h = Harness::with_items(vec![]);
    h.send(Event::SetPageProperty {
        key: "owner".into(),
        value_type: "text".into(),
        value: "me".into(),
    });
    h.send(Event::DeletePageProperty {
        key: "owner".into(),
    });
    assert!(h
        .api
        .calls
        .iter()
        .any(|c| matches!(c, ApiOp::DeletePageProperty { key, .. } if key == "owner")));
    assert!(page_properties(&h).is_empty());
    // The rows stay off once the last property is gone, unless the menu opens them
    assert!(!shows_properties(&h));
    h.send(Event::EditPageProperties);
    assert!(shows_properties(&h));
}

// タグの前に打たれた # は取り除かれ、同じタグは一つだけ残される。
#[test]
fn a_hash_typed_before_a_tag_is_dropped_and_a_tag_is_kept_once() {
    let mut h = Harness::with_items(vec![]);
    h.send(Event::SetPageProperty {
        key: "tags".into(),
        value_type: "tags".into(),
        value: "#abc, ##def, abc,".into(),
    });
    assert!(h
        .api
        .calls
        .iter()
        .any(|c| matches!(c, ApiOp::UpsertPageProperty { value, .. } if value == "abc, def")));
}

// リストのプロパティは、値をコンマで並べた1行として示される。
#[test]
fn a_list_property_is_shown_as_one_comma_separated_line() {
    let mut h = Harness::with_items(vec![]);
    if let Screen::Page(p) = &mut h.model.screen {
        p.page
            .as_mut()
            .unwrap()
            .properties
            .insert("tags".into(), ["a", "b"].into());
    }
    assert!(rows(&h)
        .iter()
        .any(|r| r == &("tags".to_string(), "a, b".to_string(), true, true)));
    let ScreenView::Page(Some(p)) = &h.view().screen else {
        panic!("no page")
    };
    let row = p.properties.iter().find(|r| r.key == "tags").unwrap();
    assert_eq!(row.values, ["a", "b"]);
}

/// A shown page whose list property `key` holds `tags`.
fn with_list(key: &str, tags: &[&str]) -> Harness {
    let mut h = Harness::with_items(vec![]);
    if let Screen::Page(p) = &mut h.model.screen {
        p.page.as_mut().unwrap().properties.insert(
            key.into(),
            PropertyValue::List(tags.iter().map(|t| t.to_string()).collect()),
        );
    }
    h
}

/// The values the server was asked to store, in order.
fn upserted(h: &Harness) -> Vec<(String, String, String)> {
    h.api
        .calls
        .iter()
        .filter_map(|c| match c {
            ApiOp::UpsertPageProperty {
                key,
                value_type,
                value,
                ..
            } => Some((key.clone(), value_type.clone(), value.clone())),
            _ => None,
        })
        .collect()
}

fn triple(key: &str, value_type: &str, value: &str) -> (String, String, String) {
    (key.into(), value_type.into(), value.into())
}

// タグの × は、そのタグだけをリストから取り除く。
#[test]
fn the_cross_of_a_tag_takes_only_that_tag_out() {
    let mut h = with_list("tags", &["a", "b", "c"]);
    h.send(Event::RemovePageTag {
        key: "tags".into(),
        tag: "b".into(),
    });
    assert_eq!(upserted(&h), [triple("tags", "tags", "a, c")]);
}

// 最後のタグを取り除くと、プロパティごと消える。タグのない行は残らない。
#[test]
fn removing_the_last_tag_deletes_the_property() {
    let mut h = with_list("books_list", &["a"]);
    h.send(Event::RemovePageTag {
        key: "books_list".into(),
        tag: "a".into(),
    });
    assert!(upserted(&h).is_empty());
    assert!(h
        .api
        .calls
        .iter()
        .any(|c| matches!(c, ApiOp::DeletePageProperty { key, .. } if key == "books_list")));
}

// ドラッグしたタグは、落とした先のタグの前か後ろに移る。元の場所に落としても何も送らない。
#[test]
fn a_dragged_tag_moves_before_or_after_the_target() {
    let mut h = with_list("tags", &["a", "b", "c"]);
    h.send(Event::MovePageTag {
        key: "tags".into(),
        tag: "a".into(),
        target: "c".into(),
        after: true,
    });
    assert_eq!(upserted(&h), [triple("tags", "tags", "b, c, a")]);

    let mut h = with_list("tags", &["a", "b", "c"]);
    h.send(Event::MovePageTag {
        key: "tags".into(),
        tag: "a".into(),
        target: "b".into(),
        after: false,
    });
    assert!(upserted(&h).is_empty());
}

// 打ったタグは、リストの後ろに足される。# と、すでにあるタグは落とされる。
#[test]
fn typed_tags_are_added_after_the_list() {
    let mut h = with_list("tags", &["a", "b"]);
    h.send(Event::AddPageTags {
        key: "tags".into(),
        typed: "#d, a".into(),
    });
    assert_eq!(upserted(&h), [triple("tags", "tags", "a, b, d")]);
}

// まだないリストのプロパティにタグを打つと、そのタグでプロパティができる。空のときは何も送らない。
#[test]
fn typed_tags_make_a_list_the_page_does_not_have_yet() {
    let mut h = Harness::with_items(vec![]);
    h.send(Event::AddPageTags {
        key: "books_list".into(),
        typed: " ".into(),
    });
    assert!(upserted(&h).is_empty());
    h.send(Event::AddPageTags {
        key: "books_list".into(),
        typed: "x".into(),
    });
    assert_eq!(upserted(&h), [triple("books_list", "list", "x")]);
}

fn definition(key: &str, value_type: &str, page: bool) -> PropertyDefinition {
    PropertyDefinition {
        key: key.into(),
        value_type: value_type.into(),
        page,
        ..PropertyDefinition::default()
    }
}

/// The rows as the shell sends their values: key and the type of a new value.
fn value_types(h: &Harness) -> Vec<(String, String)> {
    match &h.view().screen {
        ScreenView::Page(Some(p)) => p
            .properties
            .iter()
            .map(|r| (r.key.clone(), r.value_type.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

// 行の新しい値は、定義の型で送られる。定義のないキーはテキストで、リストは定義がなくてもリストのまま送られる。
#[test]
fn a_row_sends_its_value_with_the_type_of_its_definition() {
    let mut h = Harness::with_items(vec![]);
    h.model.config.property_definitions = vec![
        definition("created", "datetime", false),
        definition("rating_n", "number", true),
    ];
    if let Screen::Page(p) = &mut h.model.screen {
        let properties = &mut p.page.as_mut().unwrap().properties;
        properties.insert("rating_n".into(), "3".into());
        properties.insert("note".into(), "x".into());
        properties.insert("tags".into(), ["a"].into());
        properties.insert("books_list".into(), ["b"].into());
    }
    let types = value_types(&h);
    let type_of = |key: &str| {
        types
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, t)| t.as_str())
            .unwrap()
    };
    assert_eq!(type_of("created"), "datetime");
    assert_eq!(type_of("modified"), "text");
    assert_eq!(type_of("rating_n"), "number");
    assert_eq!(type_of("note"), "text");
    assert_eq!(type_of("tags"), "tags");
    assert_eq!(type_of("books_list"), "list");
}

// 新しい行が取れるキーは、ページに使える定義のうち、まだページにないものである。キーは表示の名前と型を持つ。
#[test]
fn a_new_row_may_take_the_page_keys_the_page_does_not_have_yet() {
    let mut h = Harness::with_items(vec![]);
    h.model.config.property_definitions = vec![
        definition("rating_n", "number", true),
        definition("due_d", "date", true),
        definition("secret", "text", false),
    ];
    if let Screen::Page(p) = &mut h.model.screen {
        let properties = &mut p.page.as_mut().unwrap().properties;
        properties.insert("rating_n".into(), "3".into());
    }
    let ScreenView::Page(Some(p)) = &h.view().screen else {
        panic!("no page")
    };
    let free: Vec<(&str, &str, &str)> = p
        .free_keys
        .iter()
        .map(|k| (k.key.as_str(), k.name.as_str(), k.value_type.as_str()))
        .collect();
    assert_eq!(free, [("due_d", "due", "date")]);
}
