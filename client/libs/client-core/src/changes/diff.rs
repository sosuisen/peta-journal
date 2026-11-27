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

impl<T> ListChange<T> {
    /// The same change, with each item made into another form by `to`.
    pub fn map<U>(self, mut to: impl FnMut(T) -> U) -> ListChange<U> {
        match self {
            ListChange::Insert { at, item } => ListChange::Insert { at, item: to(item) },
            ListChange::Remove { at } => ListChange::Remove { at },
            ListChange::Move { from, to: place } => ListChange::Move { from, to: place },
            ListChange::Update { at, item } => ListChange::Update { at, item: to(item) },
            ListChange::Reset { items } => ListChange::Reset {
                items: items.into_iter().map(to).collect(),
            },
        }
    }
}

/// Makes the type of the changes of one list, of the same shape as [`ListChange`], to give to
/// the shells (ADR 088), with a `From` of the `ListChange` of its items. The types the shells
/// take cannot use one generic type with items of two kinds, so each list has a type of its
/// own; the core works with `ListChange` and moves the changes into it at the border.
macro_rules! list_change {
    ($(#[$doc:meta])* $name:ident<$b:ident>, $item:ty) => {
        $(#[$doc])*
        #[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
        #[derive(Debug, Clone, PartialEq, serde::Serialize)]
        pub enum $name<$b> {
            Insert { at: u32, item: $item },
            Remove { at: u32 },
            Move { from: u32, to: u32 },
            Update { at: u32, item: $item },
            Reset { items: Vec<$item> },
        }

        impl<$b> From<$crate::changes::ListChange<$item>> for $name<$b> {
            fn from(change: $crate::changes::ListChange<$item>) -> $name<$b> {
                use $crate::changes::ListChange;
                match change {
                    ListChange::Insert { at, item } => $name::Insert { at, item },
                    ListChange::Remove { at } => $name::Remove { at },
                    ListChange::Move { from, to } => $name::Move { from, to },
                    ListChange::Update { at, item } => $name::Update { at, item },
                    ListChange::Reset { items } => $name::Reset { items },
                }
            }
        }
    };
}

pub(crate) use list_change;

/// The changes that make `before` into `after`. `id` gives the id of an item: no two items
/// of one list have the same id.
pub fn list_changes<T: Clone + PartialEq>(
    before: &[T],
    after: &[T],
    id: impl Fn(&T) -> &str,
) -> Vec<ListChange<T>> {
    if before.is_empty() && after.is_empty() {
        return Vec::new();
    }
    let kept: HashSet<&str> = after.iter().map(&id).collect();
    if !before.iter().any(|old| kept.contains(id(old))) {
        return vec![ListChange::Reset {
            items: after.to_vec(),
        }];
    }
    let mut shown = Shown {
        items: before.iter().collect(),
        changes: Vec::new(),
    };
    shown.remove_gone(|item| kept.contains(id(item)));
    for (at, new) in after.iter().enumerate() {
        shown.place(at, new, &id);
    }
    shown.changes
}

/// The list as the shell has it after the changes so far: list C of ADR 076.
struct Shown<'a, T> {
    items: Vec<&'a T>,
    changes: Vec<ListChange<T>>,
}

impl<'a, T: Clone + PartialEq> Shown<'a, T> {
    /// Removes the items that are not kept, from the last place on: a place before them still
    /// counts as it did.
    fn remove_gone(&mut self, kept: impl Fn(&T) -> bool) {
        for at in (0..self.items.len()).rev() {
            if !kept(self.items[at]) {
                self.items.remove(at);
                self.changes.push(ListChange::Remove { at: place(at) });
            }
        }
    }

    /// Puts `new` at `at`. The places before it hold the items of the list after already, and
    /// an id is in a list once: the item with the id of `new` is at `at` or later, or nowhere.
    fn place(&mut self, at: usize, new: &'a T, id: impl Fn(&T) -> &str) {
        let found = self.items[at..].iter().position(|old| id(old) == id(new));
        let Some(later) = found else {
            self.items.insert(at, new);
            self.changes.push(ListChange::Insert {
                at: place(at),
                item: new.clone(),
            });
            return;
        };
        if later > 0 {
            let moved = self.items.remove(at + later);
            self.items.insert(at, moved);
            self.changes.push(ListChange::Move {
                from: place(at + later),
                to: place(at),
            });
        }
        if self.items[at] != new {
            self.items[at] = new;
            self.changes.push(ListChange::Update {
                at: place(at),
                item: new.clone(),
            });
        }
    }
}

/// A place in a list, as it crosses the border.
fn place(at: usize) -> u32 {
    u32::try_from(at).expect("a list has fewer items than u32::MAX")
}
