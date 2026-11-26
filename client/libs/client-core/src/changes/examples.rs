//! Examples of the changes to a list, which every shell applies in its tests as the core does
//! (ADR 089, ADR 090 D1).

use super::{list_changes, ListChange};
use crate::testing::plain_item;
use crate::view::Row;
use crate::BodyModel;
use std::sync::Arc;

/// One example: a list, the changes that `list_changes` makes from it to another, and that
/// other list.
#[derive(Debug, Clone, PartialEq)]
pub struct ListExample<T> {
    /// What the shells name the example by, unique among the examples.
    pub id: &'static str,
    pub before: Vec<T>,
    pub changes: Vec<ListChange<T>>,
    pub after: Vec<T>,
}

/// The examples of the rows of the outline.
pub fn row_examples<B: BodyModel>() -> Vec<ListExample<Row<B>>> {
    let rows = |ids: &[&str]| -> Vec<Row<B>> { ids.iter().map(|id| row(id, id)).collect() };
    let example = |id, before: Vec<Row<B>>, after: Vec<Row<B>>| ListExample {
        id,
        changes: list_changes(&before, &after, |row| &row.item.id),
        before,
        after,
    };
    vec![
        example("insert-into-empty", rows(&[]), rows(&["a"])),
        example("insert-first", rows(&["b", "c"]), rows(&["a", "b", "c"])),
        example("insert-last", rows(&["a", "b"]), rows(&["a", "b", "c"])),
        example("remove-first", rows(&["a", "b", "c"]), rows(&["b", "c"])),
        example("remove-last", rows(&["a", "b", "c"]), rows(&["a", "b"])),
        example(
            "move-last-to-first",
            rows(&["a", "b", "c"]),
            rows(&["c", "a", "b"]),
        ),
        example(
            "move-first-to-last",
            rows(&["a", "b", "c"]),
            rows(&["b", "c", "a"]),
        ),
        example(
            "update",
            rows(&["a", "b"]),
            vec![row("a", "a"), row("b", "b changed")],
        ),
        example("reset-to-others", rows(&["a", "b"]), rows(&["c", "d"])),
        example("reset-to-empty", rows(&["a"]), rows(&[])),
        example("empty-to-empty", rows(&[]), rows(&[])),
        example(
            "every-kind-at-once",
            rows(&["a", "b", "c", "d"]),
            vec![
                row("e", "e"),
                row("d", "d changed"),
                row("b", "b"),
                row("c", "c"),
            ],
        ),
    ]
}

/// The row of the item `id` whose body is `body`.
fn row<B: BodyModel>(id: &str, body: &str) -> Row<B> {
    Row {
        item: Arc::new(plain_item(id, 1, body)),
        body_model: B::of_markdown(body, "tags"),
        position: 0,
        indent_px: 0,
        is_editing: false,
        is_selected: false,
        is_collapsed: false,
        has_children: false,
        heading_level: 0,
        is_heading: false,
        heading_line: false,
        first_line_scale: 1.0,
        task: None,
        has_own_zoom: false,
        is_transclusion: false,
        is_broken_transclusion: false,
        show_bullet: true,
        effective_zoom: 1.0,
        date_header: None,
        suggestions_visible: false,
    }
}
