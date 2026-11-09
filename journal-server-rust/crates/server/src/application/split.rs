//! What Enter makes of an item: which text stays in it, which text goes into the new item, and
//! where the new item goes. Nothing here reads or writes the database.

use super::paste::{has_unclosed_code_fence, split_at_char};
use crate::domain::model::{Item, ItemPosition};
use hmtid::Hmtid;

/// What to write for a split.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitPlan {
    /// The body the item has after the split.
    pub body: String,
    /// The body of the new item.
    pub new_body: String,
    /// The page the new item goes on.
    pub page_id: Hmtid,
    pub position: ItemPosition,
    pub level: i32,
}

/// A transclusion as its page shows it: the item that transcludes, the item it shows, and
/// the items of the page of each, in their order.
pub struct Transclusion {
    pub root: Item,
    pub root_page: Vec<Item>,
    pub source: Item,
    pub source_page: Vec<Item>,
}

/// Plans the split of `item` at the character `cursor` of `body`, the text an editor has now.
/// `on_page` holds the items of its page in their order.
pub fn plan(on_page: &[Item], item: &Item, body: &str, cursor: usize) -> SplitPlan {
    closed(cut(on_page, item, body, cursor))
}

/// Plans the split of a transclusion at the character `cursor` of `body`, the text its
/// editor has now. The text is the text of the source item: what is cut off stays on the
/// page of the source, also where the page of the transclusion does not show it then.
pub fn plan_of_transclusion(shown: &Transclusion, body: &str, cursor: usize) -> SplitPlan {
    if at_start(body, cursor) {
        // The empty item holds no text of the source: it goes above the transclusion
        return plan(&shown.root_page, &shown.root, body, cursor);
    }
    if let Some(below) = plan_below(shown, &shown.root, body, cursor) {
        return below;
    }
    let (before, after) = split_at_char(body, cursor);
    let (position, level) = shown.place_of_the_rest();
    closed(SplitPlan {
        body: before.to_string(),
        new_body: after.to_string(),
        page_id: shown.source.page_id.clone(),
        position,
        level,
    })
}

/// Plans the empty item for a split at the end of the last item a transclusion shows,
/// `item`: it goes below the transclusion, on its page. An item below `item` on the page of
/// the source would not be shown where the edit goes on, or would stay inside the
/// transclusion. None for any other split.
pub fn plan_below(
    shown: &Transclusion,
    item: &Item,
    body: &str,
    cursor: usize,
) -> Option<SplitPlan> {
    let (_, after) = split_at_char(body, cursor);
    let leaves = after.is_empty() && shown.last_shown().id == item.id;
    leaves.then(|| plan(&shown.root_page, &shown.root, body, cursor))
}

impl Transclusion {
    /// True for an item of the page of the source that is below the source item.
    pub fn has_below(&self, item: &Item) -> bool {
        descendants(&self.source_page, &self.source).any(|below| below.id == item.id)
    }

    /// The last item the transclusion shows: itself when it shows no children.
    fn last_shown(&self) -> &Item {
        let mut last = &self.root;
        // The level of a collapsed item above: what is deeper below it is hidden
        let mut hides_below = self.root.collapsed.then_some(self.source.level);
        for item in descendants(&self.source_page, &self.source) {
            if hides_below.is_some_and(|level| item.level > level) {
                continue;
            }
            hides_below = item.collapsed.then_some(item.level);
            last = item;
        }
        last
    }

    /// Where the text cut off the source item goes, on the page of the source.
    fn place_of_the_rest(&self) -> (ItemPosition, i32) {
        let source = &self.source;
        let children_of_source = last_descendant(&self.source_page, source);
        if self.root.collapsed {
            // Not among the children of the source, which the transclusion hides
            let above = children_of_source.unwrap_or(source);
            return (ItemPosition::After(above.id.clone()), source.level);
        }
        // The children of the source and the own children of the transclusion are both shown
        // below it: what is cut off goes right below it too, as the first child
        let shows_children =
            children_of_source.is_some() || last_descendant(&self.root_page, &self.root).is_some();
        let level = source.level + i32::from(shows_children);
        (ItemPosition::After(source.id.clone()), level)
    }
}

/// A stored body never ends inside a code fence.
fn closed(mut plan: SplitPlan) -> SplitPlan {
    if has_unclosed_code_fence(&plan.body) {
        plan.body.push_str("\n```");
    }
    plan
}

/// The items below `item` on its page, in their order.
fn descendants<'a>(on_page: &'a [Item], item: &'a Item) -> impl Iterator<Item = &'a Item> {
    on_page
        .iter()
        .skip_while(|other| other.id != item.id)
        .skip(1)
        .take_while(|other| other.level > item.level)
}

/// The last of the items below `item` on its page, if it has any.
fn last_descendant<'a>(on_page: &'a [Item], item: &'a Item) -> Option<&'a Item> {
    descendants(on_page, item).last()
}

/// True when the whole text would move down. An empty item above does the same and keeps
/// the id of the item.
fn at_start(body: &str, cursor: usize) -> bool {
    cursor == 0 && !body.is_empty()
}

/// An empty item right above `item` on its page, which keeps the whole body.
fn empty_above(on_page: &[Item], item: &Item, body: &str) -> SplitPlan {
    let above = on_page
        .iter()
        .take_while(|other| other.id != item.id)
        .last();
    SplitPlan {
        body: body.to_string(),
        new_body: String::new(),
        page_id: item.page_id.clone(),
        position: above.map_or(ItemPosition::First, |a| ItemPosition::After(a.id.clone())),
        level: item.level,
    }
}

fn cut(on_page: &[Item], item: &Item, body: &str, cursor: usize) -> SplitPlan {
    if at_start(body, cursor) {
        return empty_above(on_page, item, body);
    }
    let (before, after) = split_at_char(body, cursor);
    let (above, level) = match last_descendant(on_page, item) {
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
        page_id: item.page_id.clone(),
        position: ItemPosition::After(above.id.clone()),
        level,
    }
}
