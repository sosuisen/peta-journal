//! Examples of the changes to a list, which every shell applies in its tests as the core does
//! (ADR 089, ADR 090 D1).

use super::ListChange;
use crate::view::Row;
use crate::BodyModel;

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
    Vec::new()
}
