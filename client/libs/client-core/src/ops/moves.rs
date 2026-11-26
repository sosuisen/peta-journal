//! Moves an item with its children past the sibling tree above or below it, ported from
//! `item-move.service.ts` and `journal-day-move.service.ts`. Inside a transclusion the move
//! stays within the transcluded tree; at the edge of a journal day the item goes to the
//! neighbouring day instead.
//!
//! Moves run one after another. A move computes its target from the item list, and the
//! list changes only when the server has answered. A second key press that did not wait
//! would compute the same target again and the press would be lost.
//!
//! Every move is one request, which carries the text that was typed into the item and is
//! not saved yet (ADR 011). The screen changes when its answer is there.

use crate::app::{done, log_error, run, Cmd, Event, Model, MoveDirection};
use crate::model::ItemExt;
use crate::model::{Item, ItemPosition, ItemRef, Page};
use crate::ops::edit::{auto_save_again, take_unsaved};
use crate::ops::file_reload::{leave_the_edit, read_again_if_out_of_place, replace_page, AskedAt};
use crate::ops::split::host_page_of_transclusion;
use crate::ops::undo_replay::replay_held_undos;
use crate::outline::tree::{
    find_cross_page_placement, find_move_down_target, find_move_up_target, get_subtree,
    selected_root_items, transclusion_context_of, transclusion_scoped_items, ScanDirection,
    TransclusionContext,
};
use crate::screens;
use crate::util::date::shift_iso_date;
use crux_core::Command;

/// What a move to another day's page did; the journal reloads from it.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CrossDayMove {
    /// The date of the page the item went to.
    pub target_date: String,
    /// True when the journal should navigate to that page.
    pub navigate: bool,
    pub item_id: String,
    /// The page the move made, when the target day had no page yet.
    pub created_page: Option<Page>,
}

/// A move of a single item; the selection moves are queued the same way with an empty id.
pub fn enqueue_move(model: &mut Model, item_id: &str, direction: MoveDirection) -> Cmd {
    model.move_queue.push_back((item_id.to_string(), direction));
    start_next_move(model)
}

/// Starts the next queued move unless one is running. A move that has nowhere to go is
/// skipped.
pub fn start_next_move(model: &mut Model) -> Cmd {
    if model.move_in_progress {
        return done();
    }
    while let Some((item_id, direction)) = model.move_queue.pop_front() {
        let started = if item_id.is_empty() {
            move_selection(model, direction)
        } else {
            move_item(model, &item_id, direction)
        };
        if let Some(cmd) = started {
            model.move_in_progress = true;
            return cmd;
        }
    }
    done()
}

/// Ends a move, and starts the next one. `handover` says that the move kept the blur of
/// the editor from ending the edit, which it does not do any longer.
pub fn move_done(model: &mut Model, handover: bool) -> Cmd {
    if handover {
        model.edits.edit_handovers = model.edits.edit_handovers.saturating_sub(1);
    }
    model.move_in_progress = false;
    let next = start_next_move(model);
    let undos = replay_held_undos(model);
    Command::all([next, undos])
}

/// Moves an item and its children up or down past the neighbouring sibling tree. None when
/// there is nowhere to move it.
fn move_item(model: &mut Model, item_id: &str, direction: MoveDirection) -> Option<Cmd> {
    let item = model.cloned_item(item_id)?;
    match plan_move(model, &item, direction) {
        MovePlan::Nothing => None,
        MovePlan::WithinTransclusion {
            group_ids,
            position,
        } => Some(move_in_transclusion(model, item, group_ids, position)),
        MovePlan::AcrossDays(plan) => Some(move_to_day(model, item, plan)),
        MovePlan::WithinPage {
            position,
            state_position,
        } => Some(move_on_page(model, item, position, state_position)),
    }
}

/// Moves an item with its children to a position on its own page.
fn move_on_page(
    model: &mut Model,
    item: Item,
    position: ItemPosition,
    state_position: ItemPosition,
) -> Cmd {
    let group_ids: Vec<String> = model
        .state
        .get_item_with_children(&item.id)
        .iter()
        .map(|b| b.id.clone())
        .collect();
    request_move(model, item, group_ids, position, 0, |moved, item_id| {
        Event::Moved {
            moved,
            position: state_position,
            edited: Some(item_id),
            label: MoveLabel::Move,
        }
    })
}

/// Moves an item that a transclusion shows, within the tree of the source item.
fn move_in_transclusion(
    model: &mut Model,
    item: Item,
    group_ids: Vec<String>,
    position: ItemPosition,
) -> Cmd {
    request_move(model, item, group_ids, position, 0, |moved, item_id| {
        Event::MovedInTransclusion { item_id, moved }
    })
}

/// Asks the server to move an edited item with the items below it, `level_delta` levels
/// deeper. The request carries the text that was typed into the item and is not saved yet
/// (ADR 011). `answered` makes the event for the moved items the server answers with.
pub fn request_move(
    model: &mut Model,
    item: Item,
    group_ids: Vec<String>,
    position: ItemPosition,
    level_delta: i32,
    answered: impl FnOnce(Vec<Item>, String) -> Event + Send + 'static,
) -> Cmd {
    let (body, discard) = take_unsaved(model, &item);
    // The editor is focused again when the item is at its new place; keys pressed before
    // that are kept for it
    model.key_relay.hold_until_focus();
    let request = run(move |api| async move {
        match api
            .move_edited_items(
                group_ids,
                &item.page_id,
                position,
                level_delta,
                body.as_deref(),
            )
            .await
        {
            Ok(moved) => api.send_event(answered(moved, item.id)),
            Err(e) => {
                log_error(format!("Failed to move item: {e}"));
                api.send_event(Event::MoveFailed {
                    unsaved: body.map(|_| item.id),
                });
            }
        }
    });
    Command::all([discard, request])
}

/// Asks the server to move an item with the items below it to the journal of the
/// neighbouring day. The request carries the text that was typed into the item and is not
/// saved yet (ADR 011); the journal is made when there is none.
fn move_to_day(model: &mut Model, item: Item, plan: DayMovePlan) -> Cmd {
    let (body, discard) = take_unsaved(model, &item);
    let request = run(move |api| async move {
        match api
            .move_items_to_journal(
                &plan.target_date,
                plan.group_ids,
                plan.position,
                body.as_deref(),
            )
            .await
        {
            Ok(journal) => api.send_event(Event::MovedToDay {
                moved: CrossDayMove {
                    target_date: plan.target_date,
                    navigate: plan.navigate,
                    item_id: item.id,
                    created_page: journal.created.then_some(journal.page),
                },
            }),
            Err(e) => {
                log_error(format!("Failed to move item to another day: {e}"));
                api.send_event(Event::MoveFailed {
                    unsaved: body.map(|_| item.id),
                });
            }
        }
    });
    Command::all([discard, request])
}

/// Shows what a move to another day did, and ends the move.
pub fn show_moved_to_day(model: &mut Model, moved: CrossDayMove) -> Cmd {
    let shown = screens::journal::on_cross_day_moved(model, moved);
    Command::all([shown, move_done(model, false)])
}

/// Records the move inside a transclusion for undo, and reads the page again: the server's
/// version of the page says how the transclusion shows its items now.
pub fn show_moved_in_transclusion(model: &mut Model, item_id: String, moved: Vec<Item>) -> Cmd {
    // A journal shows the pages of many days: the page to read is the one of the transclusion
    let Some(page_id) = host_page_of_transclusion(model, &item_id) else {
        // The screen went on to another page while the server was asked
        model.key_relay.release();
        return move_done(model, false);
    };
    let before: Vec<Item> = moved
        .iter()
        .filter_map(|item| model.state.get_item_by_id(&item.id))
        .map(|item| (*item).clone())
        .collect();
    if before.len() == moved.len() {
        model.undo.record_updates(
            MoveLabel::Move.as_str(),
            before.iter().zip(&moved),
            model.clock_ms,
        );
    }
    // Reading the page again blurs the editor of the item: the edit goes on in it
    model.edits.edit_handovers += 1;
    let asked_at = AskedAt::now(model);
    run(move |api| async move {
        replace_page(&api, &page_id, asked_at).await;
        api.send_event(Event::MovePageReloaded { item_id, asked_at });
    })
}

/// Focuses the editor of the moved item again, with the keys that were pressed while the
/// move was on its way.
pub fn focus_moved(model: &mut Model, item_id: String) -> Cmd {
    focus_moved_at(model, item_id, AskedAt::now(model))
}

/// [`focus_moved`] after the page was read again. When a change made here, such as an undo,
/// overtook the reading, the edit stays where that change left it, and the move is over.
pub fn focus_moved_at(model: &mut Model, item_id: String, asked_at: AskedAt) -> Cmd {
    if asked_at.is_overtaken(model) {
        let keys = leave_the_edit(model);
        let done = run(|api| async move { api.send_event(Event::MoveDone { handover: true }) });
        return Command::all([keys, done]);
    }
    model.state.order_focus(Some(&item_id));
    let keys = model.key_relay.hand_over();
    run(move |api| async move {
        api.focus_editor(Some(item_id), keys).await;
        api.send_event(Event::MoveDone { handover: true });
    })
}

/// What the undo history calls a move.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MoveLabel {
    Move,
    MoveSelection,
    Outdent,
}

impl MoveLabel {
    pub fn as_str(self) -> &'static str {
        match self {
            MoveLabel::Move => "move",
            MoveLabel::MoveSelection => "move-selection",
            MoveLabel::Outdent => "outdent",
        }
    }
}

/// Puts the items the server moved at their new place, and records the move for undo under
/// `label`. The edit goes on in `edited`, the item the move was started from; a move of the
/// selection has none.
pub fn show_moved(
    model: &mut Model,
    moved: Vec<Item>,
    position: &ItemPosition,
    edited: Option<String>,
    label: MoveLabel,
) -> Cmd {
    let ids: Vec<String> = moved.iter().map(|item| item.id.clone()).collect();
    let before: Vec<Item> = ids
        .iter()
        .filter_map(|id| model.state.get_item_by_id(id))
        .map(|item| (*item).clone())
        .collect();
    if before.len() != moved.len() {
        // The screen went on to another page while the server was asked
        model.key_relay.release();
        return move_done(model, false);
    }
    model
        .undo
        .record_updates(label.as_str(), before.iter().zip(&moved), model.clock_ms);
    model.state.replace_items(&ids, moved, position);
    let reread = read_again_if_out_of_place(model, &ids);
    let Some(item_id) = edited else {
        return Command::all([move_done(model, false), reread]);
    };
    // Putting the item at its new place blurs its editor: the edit goes on in it
    model.edits.edit_handovers += 1;
    Command::all([focus_moved(model, item_id), reread])
}

/// Ends the wait for the editor after a move that did not reach the server, and schedules
/// the auto-save of the typed text the move was to carry.
pub fn move_failed(model: &mut Model, unsaved: Option<&str>) -> Cmd {
    model.key_relay.release();
    // The undos waited for this move, which did not happen
    model.held_undos = 0;
    let save = auto_save_again(model, unsaved);
    Command::all([save, move_done(model, false)])
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
fn move_selection(model: &mut Model, direction: MoveDirection) -> Option<Cmd> {
    let roots = selected_root_items(model.state.items(), model.state.selected_item_ids());
    if roots.is_empty() {
        return None;
    }
    let edge_root = match direction {
        MoveDirection::Up => roots.first().unwrap().clone(),
        MoveDirection::Down => roots.last().unwrap().clone(),
    };
    let target = match direction {
        MoveDirection::Up => find_move_up_target(model.state.items(), &edge_root.id),
        MoveDirection::Down => find_move_down_target(model.state.items(), &edge_root.id),
    };
    let target = target?;
    let mut all_ids: Vec<String> = Vec::new();
    for root in &roots {
        for b in model.state.get_item_with_children(&root.id) {
            all_ids.push(b.id.clone());
        }
    }
    let page_id = edge_root.page_id.clone();
    Some(run(move |api| async move {
        match api
            .move_items(all_ids, &page_id, target.clone(), None)
            .await
        {
            Ok(moved) => api.send_event(Event::Moved {
                moved,
                position: target,
                edited: None,
                label: MoveLabel::MoveSelection,
            }),
            Err(e) => {
                log_error(format!("Failed to move selected items: {e}"));
                api.send_event(Event::MoveDone { handover: false });
            }
        }
    }))
}

// ---- Journal day moves ----

/// A move of a journal item with its children to the page of the neighbouring day.
#[derive(Debug, Clone)]
struct DayMovePlan {
    target_date: String,
    group_ids: Vec<String>,
    position: ItemPosition,
    /// True when the journal goes on to the target day: the item left the day it shows.
    navigate: bool,
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
                return Some(DayMovePlan {
                    target_date: adjacent(1),
                    group_ids,
                    position: ItemPosition::Last,
                    // The journal follows the item to the next day when it left the day shown
                    navigate: model.shown_page_id().as_deref() == Some(&item.page_id),
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
                return Some(DayMovePlan {
                    target_date: adjacent(-1),
                    group_ids,
                    position: ItemPosition::First,
                    navigate: false,
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
    let position = find_cross_page_placement(&target_items, item.level, scan)?;
    Some(DayMovePlan {
        target_date,
        group_ids,
        position,
        navigate: false,
    })
}
