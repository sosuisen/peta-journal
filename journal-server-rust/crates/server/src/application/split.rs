//! What Enter makes of an item: which text stays in it, which text goes into the new item, and
//! where the new item goes. Nothing here reads or writes the database.

use super::paste::split_at_char;
use crate::domain::model::{Item, ItemPosition};

/// What to write for a split.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitPlan {
    /// The body the item has after the split.
    pub body: String,
    /// The body of the new item.
    pub new_body: String,
    pub position: ItemPosition,
    pub level: i32,
}

/// Plans the split of `item` at the character `cursor` of `body`, the text an editor has now.
/// `on_page` holds the items of its page in their order.
pub fn plan(on_page: &[Item], item: &Item, body: &str, cursor: usize) -> SplitPlan {
    if cursor == 0 && !body.is_empty() {
        // The whole text would move down: an empty item above does the same and keeps the id
        let above = on_page
            .iter()
            .take_while(|other| other.id != item.id)
            .last();
        return SplitPlan {
            body: body.to_string(),
            new_body: String::new(),
            position: above.map_or(ItemPosition::First, |a| ItemPosition::After(a.id.clone())),
            level: item.level,
        };
    }
    let (before, after) = split_at_char(body, cursor);
    let last_descendant = on_page
        .iter()
        .skip_while(|other| other.id != item.id)
        .skip(1)
        .take_while(|other| other.level > item.level)
        .last();
    let (above, level) = match last_descendant {
        // The new item is not hidden with the children: it goes after them
        Some(last) if item.collapsed => (last, item.level),
        // Right below an open parent is the place of its first child: an item of the
        // parent's level there would take the children
        Some(_) => (item, item.level + 1),
        None => (item, item.level),
    };
    SplitPlan {
        body: before.to_string(),
        new_body: after.to_string(),
        position: ItemPosition::After(above.id.clone()),
        level,
    }
}
