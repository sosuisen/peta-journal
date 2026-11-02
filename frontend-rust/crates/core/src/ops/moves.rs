//! Moves an item with its children past the sibling tree above or below it, ported from
//! `item-move.service.ts` and `journal-day-move.service.ts`. Inside a transclusion the move
//! stays within the transcluded tree; at the edge of a journal day the item goes to the
//! neighbouring day instead.
//!
//! Moves run one after another. A move computes its target from the item list, and the
//! list changes only when the server has answered. A second key press that did not wait
//! would compute the same target again and the press would be lost.

use crate::app::{apply, done, log_error, run, Cmd, Event, Model, MoveDirection};
use crate::effects::{Api, ApiError, ApiResult};
use crate::model::{Item, ItemPosition, ItemRef};
use crate::ops::edit::{flush_pending_edit, focus_editor, read, Handover};
use crate::ops::file_reload::reload_shown_page;
use crate::outline::tree::{
    find_cross_page_placement, find_move_down_target, find_move_up_target, get_subtree,
    selected_root_items, transclusion_context_of, transclusion_scoped_items, ScanDirection,
    TransclusionContext,
};
use crate::screens;
use crate::util::date::shift_iso_date;

/// What a move to another day's page did; the journal reloads from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossDayMove {
    /// The date of the page the item went to.
    pub target_date: String,
    /// True when the journal should navigate to that page.
    pub navigate: bool,
    pub item_id: String,
}

/// A move of a single item; the selection moves are queued the same way with an empty id.
pub fn enqueue_move(model: &mut Model, item_id: &str, direction: MoveDirection) -> Cmd {
    model.move_queue.push_back((item_id.to_string(), direction));
    start_next_move(model)
}

/// Starts the next queued move unless one is running.
pub fn start_next_move(model: &mut Model) -> Cmd {
    if model.move_in_progress {
        return done();
    }
    let Some((item_id, direction)) = model.move_queue.pop_front() else {
        return done();
    };
    model.move_in_progress = true;
    let body = if item_id.is_empty() {
        move_selection(model, direction)
    } else {
        move_item(model, &item_id, direction)
    };
    crux_core::Command::all([body, done()]).then(crux_core::Command::event(crate::app::cont(
        "move done",
        |m| {
            m.move_in_progress = false;
            start_next_move(m)
        },
    )))
}

/// Moves an item and its children up or down past the neighbouring sibling tree.
fn move_item(model: &mut Model, item_id: &str, direction: MoveDirection) -> Cmd {
    let Some(item) = model.cloned_item(item_id) else {
        return done();
    };
    run(move |api| async move {
        if let Err(e) = flush_pending_edit(&api, &item).await {
            log_error(format!("Failed to save before move: {e}"));
        }
        if let Err(e) = move_now(&api, item, direction).await {
            log_error(format!("Failed to move item: {e}"));
        }
    })
}

async fn move_now(api: &Api<Event>, item: Item, direction: MoveDirection) -> ApiResult<()> {
    let id = item.id.clone();
    // The item as shown now, after the flush
    let item = read(api, move |m| m.current_item(&id).map(|i| (*i).clone()))
        .await
        .unwrap_or(item);
    let plan = {
        let item = item.clone();
        read(api, move |m| plan_move(m, &item, direction)).await
    };
    match plan {
        MovePlan::Nothing => Ok(()),
        MovePlan::WithinTransclusion {
            group_ids,
            position,
        } => {
            api.move_items(group_ids, &item.page_id, position, None)
                .await?;
            let _handover = Handover::begin(api);
            reload_shown_page(api).await;
            focus_editor(api, Some(item.id.clone())).await;
            Ok(())
        }
        MovePlan::AcrossDays(plan) => {
            if let Some(crossed) = move_across_days(api, &item, plan).await? {
                apply(api, "cross-day move", move |m| {
                    screens::journal::on_cross_day_moved(m, crossed)
                });
                return Ok(());
            }
            // The item stayed on its page: move it within the page
            let item2 = item.clone();
            let target = read(api, move |m| within_page_target(m, &item2, direction)).await;
            match target {
                Some((position, state_position)) => {
                    move_group_to(api, &item, position, state_position).await
                }
                None => Ok(()),
            }
        }
        MovePlan::WithinPage {
            position,
            state_position,
        } => move_group_to(api, &item, position, state_position).await,
    }
}

enum MovePlan {
    Nothing,
    WithinTransclusion {
        group_ids: Vec<String>,
        position: ItemPosition,
    },
    AcrossDays(DayMovePlan),
    WithinPage {
        position: ItemPosition,
        state_position: ItemPosition,
    },
}

fn plan_move(model: &Model, item: &Item, direction: MoveDirection) -> MovePlan {
    let t_ctx = transclusion_context_of(item, model.state.items(), model.state.transcluded_trees());
    if let Some(t_ctx) = t_ctx {
        if !model.state.is_own_item(&item.id) {
            return match plan_within_transclusion(model, item, &t_ctx, direction) {
                Some((group_ids, position)) => MovePlan::WithinTransclusion {
                    group_ids,
                    position,
                },
                None => MovePlan::Nothing,
            };
        }
    }
    if item.is_daily_journal_item() {
        if let Some(plan) = plan_day_move(model, item, direction) {
            return MovePlan::AcrossDays(plan);
        }
    }
    match within_page_target(model, item, direction) {
        Some((position, state_position)) => MovePlan::WithinPage {
            position,
            state_position,
        },
        None => MovePlan::Nothing,
    }
}

/// The target of a move on the item's own page, in page-scoped and full-list form.
fn within_page_target(
    model: &Model,
    item: &Item,
    direction: MoveDirection,
) -> Option<(ItemPosition, ItemPosition)> {
    // In journal mode the candidates are the item's own page, so no position refers to
    // another page, which the backend cannot resolve
    let candidates: Vec<ItemRef> = if item.is_daily_journal_item() {
        model
            .state
            .items()
            .iter()
            .filter(|b| b.page_id == item.page_id)
            .cloned()
            .collect()
    } else {
        model.state.items().to_vec()
    };
    let target = match direction {
        MoveDirection::Up => find_move_up_target(&candidates, &item.id),
        MoveDirection::Down => find_move_down_target(&candidates, &item.id),
    }?;
    let state_position = to_full_list_position(model, &target, &item.page_id);
    Some((target, state_position))
}

/// Translates a page-scoped position to a full-list position for state updates.
/// `first` becomes the position before the first item of the page in the full list.
fn to_full_list_position(model: &Model, position: &ItemPosition, page_id: &str) -> ItemPosition {
    if *position != ItemPosition::First {
        return position.clone();
    }
    let all = model.state.items();
    match all.iter().position(|b| b.page_id == page_id) {
        Some(idx) if idx > 0 => ItemPosition::after(all[idx - 1].id.clone()),
        _ => ItemPosition::First,
    }
}

/// Moves an item with its children to a position on its own page, keeping its editor open.
async fn move_group_to(
    api: &Api<Event>,
    item: &Item,
    position: ItemPosition,
    state_position: ItemPosition,
) -> ApiResult<()> {
    let id = item.id.clone();
    let group: Vec<Item> = read(api, move |m| {
        m.state
            .get_item_with_children(&id)
            .iter()
            .map(|b| (**b).clone())
            .collect()
    })
    .await;
    let group_ids: Vec<String> = group.iter().map(|b| b.id.clone()).collect();
    let moved = api
        .move_items(group_ids.clone(), &item.page_id, position, None)
        .await?;
    let before = group.clone();
    let moved_for_undo = moved.clone();
    apply(api, "move: record", move |m| {
        m.undo.begin("move", None);
        for (b, after) in before.iter().zip(moved_for_undo.iter()) {
            m.undo.push_update(b, after);
        }
        m.undo.commit(None, m.clock_ms);
        done()
    });
    // Reordering the list blurs the editor; the edit goes on in the moved item
    let _handover = Handover::begin(api);
    apply(api, "move: reorder", move |m| {
        m.state.replace_items(&group_ids, moved, &state_position);
        done()
    });
    focus_editor(api, Some(item.id.clone())).await;
    Ok(())
}

/// Plans a move of a transcluded descendant within the tree of its transclusion root.
fn plan_within_transclusion(
    model: &Model,
    item: &Item,
    t_ctx: &TransclusionContext,
    direction: MoveDirection,
) -> Option<(Vec<String>, ItemPosition)> {
    let scoped = transclusion_scoped_items(model.state.visible_items(), t_ctx);
    let target = match direction {
        MoveDirection::Up => find_move_up_target(&scoped, &item.id),
        MoveDirection::Down => find_move_down_target(&scoped, &item.id),
    }?;
    // The scoped list starts below the source root, so "first" means right after that root
    // on the source page, not the start of the page
    let source_root_id = model
        .state
        .get_item_by_id(&t_ctx.root_item_id)
        .and_then(|r| r.transclude_item_id.clone());
    let position = match (&target, source_root_id) {
        (ItemPosition::First, Some(source_root_id)) => ItemPosition::after(source_root_id),
        _ => target,
    };
    let group_ids = get_subtree(model.state.visible_items(), &item.id)
        .iter()
        .map(|b| b.id.clone())
        .collect();
    Some((group_ids, position))
}

/// Moves every selected subtree past the sibling tree above or below the selection.
fn move_selection(model: &mut Model, direction: MoveDirection) -> Cmd {
    let roots = selected_root_items(model.state.items(), model.state.selected_item_ids());
    if roots.is_empty() {
        return done();
    }
    let edge_root = match direction {
        MoveDirection::Up => roots.first().unwrap().clone(),
        MoveDirection::Down => roots.last().unwrap().clone(),
    };
    let target = match direction {
        MoveDirection::Up => find_move_up_target(model.state.items(), &edge_root.id),
        MoveDirection::Down => find_move_down_target(model.state.items(), &edge_root.id),
    };
    let Some(target) = target else { return done() };
    let mut all_ids: Vec<String> = Vec::new();
    for root in &roots {
        for b in model.state.get_item_with_children(&root.id) {
            all_ids.push(b.id.clone());
        }
    }
    let before: Vec<Item> = all_ids
        .iter()
        .filter_map(|id| model.state.get_item_by_id(id))
        .map(|b| (*b).clone())
        .collect();
    let page_id = edge_root.page_id.clone();
    run(move |api| async move {
        match api
            .move_items(all_ids.clone(), &page_id, target.clone(), None)
            .await
        {
            Ok(moved) => apply(&api, "move selection: reorder", move |m| {
                m.state.replace_items(&all_ids, moved.clone(), &target);
                m.undo.begin("move-selection", None);
                for (b, after) in before.iter().zip(moved.iter()) {
                    m.undo.push_update(b, after);
                }
                m.undo.commit(None, m.clock_ms);
                done()
            }),
            Err(e) => log_error(format!("Failed to move selected items: {e}")),
        }
    })
}

// ---- Journal day moves ----

#[derive(Debug, Clone)]
enum DayMovePlan {
    /// A level 1 item at the edge of its page goes to the adjacent day's page.
    Whole {
        target_date: String,
        group_ids: Vec<String>,
        position: ItemPosition,
        shown_page_id: Option<String>,
    },
    /// A level 2 item at the edge goes to the adjacent day's page when that page is loaded and has room.
    Level2 {
        target_date: String,
        target_page_id: String,
        group_ids: Vec<String>,
        placement: ItemPosition,
    },
}

/// Plans the move of a daily journal item to the adjacent day's page if it is at the edge
/// of its own page. Moving the topmost item up takes it to the next day, moving the
/// bottommost item down takes it to the previous day: the journal shows the days newest
/// first, so this keeps the order.
fn plan_day_move(model: &Model, item: &Item, direction: MoveDirection) -> Option<DayMovePlan> {
    let page_items: Vec<ItemRef> = model
        .state
        .items()
        .iter()
        .filter(|b| b.page_id == item.page_id)
        .cloned()
        .collect();
    let group_ids: Vec<String> = model
        .state
        .get_item_with_children(&item.id)
        .iter()
        .map(|b| b.id.clone())
        .collect();
    let adjacent = |days: i64| shift_iso_date(&item.name, days);

    match direction {
        MoveDirection::Up => {
            // The topmost root item moves to the end of the next day's page
            if item.level == 1
                && page_items
                    .iter()
                    .find(|b| b.level == 1)
                    .map(|b| b.id.as_str())
                    == Some(&item.id)
            {
                return Some(DayMovePlan::Whole {
                    target_date: adjacent(1),
                    group_ids,
                    position: ItemPosition::Last,
                    shown_page_id: model.shown_page_id(),
                });
            }
            // A level 2 item at the second position moves to the next day's page
            if item.level == 2 && page_items.len() >= 2 && page_items[1].id == item.id {
                return plan_level2(model, item, adjacent(1), ScanDirection::FromEnd, group_ids);
            }
            None
        }
        MoveDirection::Down => {
            // The bottommost item moves to the previous day's page
            if page_items.last().map(|b| b.id.as_str()) != Some(&item.id) {
                return None;
            }
            if item.level == 1 {
                return Some(DayMovePlan::Whole {
                    target_date: adjacent(-1),
                    group_ids,
                    position: ItemPosition::First,
                    shown_page_id: model.shown_page_id(),
                });
            }
            if item.level == 2 {
                return plan_level2(
                    model,
                    item,
                    adjacent(-1),
                    ScanDirection::FromStart,
                    group_ids,
                );
            }
            None
        }
    }
}

fn plan_level2(
    model: &Model,
    item: &Item,
    target_date: String,
    scan: ScanDirection,
    group_ids: Vec<String>,
) -> Option<DayMovePlan> {
    let target_items: Vec<ItemRef> = model
        .state
        .items()
        .iter()
        .filter(|b| b.item_type == "journal" && b.name == target_date)
        .cloned()
        .collect();
    if target_items.is_empty() {
        return None;
    }
    let placement = find_cross_page_placement(&target_items, item.level, scan)?;
    Some(DayMovePlan::Level2 {
        target_date,
        target_page_id: target_items[0].page_id.clone(),
        group_ids,
        placement,
    })
}

/// Carries out a planned day move. Returns None when a level 2 move found no room after all.
async fn move_across_days(
    api: &Api<Event>,
    item: &Item,
    plan: DayMovePlan,
) -> ApiResult<Option<CrossDayMove>> {
    match plan {
        DayMovePlan::Whole {
            target_date,
            group_ids,
            position,
            shown_page_id,
        } => {
            let result = api.get_or_create_journal(&target_date).await?;
            let is_next = position == ItemPosition::Last;
            api.move_items(group_ids.clone(), &result.page.id, position, None)
                .await?;
            // If the target page had only one item with an empty body, delete it
            if result.items.len() == 1 && result.items[0].body.is_empty() {
                api.delete_item(&result.items[0].id).await?;
            }
            keep_source_page_non_empty(api, &item.page_id, &group_ids).await?;
            // The journal navigates to the next day when the item left the page it shows
            let navigate = is_next && shown_page_id.as_deref() == Some(&item.page_id);
            Ok(Some(CrossDayMove {
                target_date,
                navigate,
                item_id: item.id.clone(),
            }))
        }
        DayMovePlan::Level2 {
            target_date,
            target_page_id,
            group_ids,
            placement,
        } => {
            api.move_items(group_ids.clone(), &target_page_id, placement, None)
                .await?;
            keep_source_page_non_empty(api, &item.page_id, &group_ids).await?;
            Ok(Some(CrossDayMove {
                target_date,
                navigate: false,
                item_id: item.id.clone(),
            }))
        }
    }
}

/// A page must keep at least one item: creates an empty one when the moved items were the last ones.
async fn keep_source_page_non_empty(
    api: &Api<Event>,
    source_page_id: &str,
    moved_ids: &[String],
) -> Result<(), ApiError> {
    let (page_id, moved) = (source_page_id.to_string(), moved_ids.to_vec());
    let remaining = read(api, move |m| {
        m.state
            .items()
            .iter()
            .filter(|b| b.page_id == page_id && !moved.contains(&b.id))
            .count()
    })
    .await;
    if remaining == 0 {
        api.create_item(source_page_id, ItemPosition::First, 1, "", None)
            .await?;
    }
    Ok(())
}
