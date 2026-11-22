//! The changes that make one list with ids into another (ADR 076). A shell applies them in
//! order to the list it shows.

use serde::Serialize;
use std::collections::HashSet;

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
    let kept: HashSet<&str> = after.iter().map(&id).collect();
    if before.is_empty() && after.is_empty() {
        return changes;
    }
    if !before.iter().any(|old| kept.contains(id(old))) {
        return vec![ListChange::Reset {
            items: after.to_vec(),
        }];
    }
    for at in (0..shown.len()).rev() {
        if !kept.contains(id(&shown[at])) {
            shown.remove(at);
            changes.push(ListChange::Remove { at: at as u32 });
        }
    }
    for (at, new) in after.iter().enumerate() {
        let found = shown[at..].iter().position(|old| id(old) == id(new));
        match found {
            None => {
                shown.insert(at, new.clone());
                changes.push(ListChange::Insert {
                    at: at as u32,
                    item: new.clone(),
                });
                continue;
            }
            Some(0) => {}
            Some(later) => {
                let moved = shown.remove(at + later);
                shown.insert(at, moved);
                changes.push(ListChange::Move {
                    from: (at + later) as u32,
                    to: at as u32,
                });
            }
        }
        if shown[at] != *new {
            shown[at] = new.clone();
            changes.push(ListChange::Update {
                at: at as u32,
                item: new.clone(),
            });
        }
    }
    changes
}
