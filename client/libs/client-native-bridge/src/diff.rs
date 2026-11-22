//! The changes that make one list with ids into another (ADR 076). A shell applies them in
//! order to the list it shows.

use serde::Serialize;

/// One change to a list. A place counts in the list as the changes before this one left it.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum ListChange<T> {
    Insert { at: u32, item: T },
    Remove { at: u32 },
    Move { from: u32, to: u32 },
    Update { at: u32, item: T },
    Reset { items: Vec<T> },
}

/// The changes that make `before` into `after`. `id` gives the id of an item: no two items
/// of one list have the same id.
pub fn list_changes<T: Clone + PartialEq>(
    before: &[T],
    after: &[T],
    id: impl Fn(&T) -> &str,
) -> Vec<ListChange<T>> {
    let mut changes = Vec::new();
    let mut shown = before.to_vec();
    for (at, new) in after.iter().enumerate() {
        match shown.get(at) {
            Some(old) if id(old) == id(new) => {
                if old != new {
                    changes.push(ListChange::Update {
                        at: at as u32,
                        item: new.clone(),
                    });
                }
            }
            _ => {
                shown.insert(at, new.clone());
                changes.push(ListChange::Insert {
                    at: at as u32,
                    item: new.clone(),
                });
            }
        }
    }
    changes
}
