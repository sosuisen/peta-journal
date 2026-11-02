//! What Enter does in an editor, ported from `item-split.service.ts`: splits the item at
//! the cursor, or inserts an empty item before or after it. An empty last child is
//! outdented instead.

use crate::app::{apply, done, log_error, run, Cmd, Event, Model};
use crate::effects::Api;
use crate::model::{Item, ItemPosition};
use crate::ops::edit::{
    close_editor, discard_pending_edit, focus_editor, save_body, save_target_id, Handover,
};
use crate::ops::file_reload::reload_shown_page;
use crate::ops::level;
use crate::outline::tree::{
    edit_context_of, get_subtree, has_sibling_below, level_for_new_item_after,
    position_after_subtree, position_before, transclusion_root_ending_at, EditContext,
};
use crate::outline::CursorState;
use crate::util::split_at_char;
use crux_core::Command;
use futures::future::join;

/// Handles Enter at a cursor position in an item.
pub fn handle_enter(model: &mut Model, item_id: &str, cursor: usize) -> Cmd {
    let Some(item) = model.cloned_item(item_id) else {
        return done();
    };
    // The next editor is focused after the next render; keys pressed before that are kept for it
    model.key_relay.hold_until_focus();
    let discard = discard_pending_edit(model, &item);

    if item.body.is_empty()
        && item.level > 1
        && !has_sibling_below(model.state.visible_items(), &item.id)
    {
        // Empty last child: outdent instead of creating a new item
        let outdent = level::outdent(model, &item.id);
        let id = item.id.clone();
        let focus = run(move |api| async move { focus_editor(&api, Some(id)).await });
        return Command::all([discard, outdent, focus]);
    }

    let ctx = edit_context_of(
        &item,
        model.state.own_item_ids(),
        model.state.transcluded_trees(),
    );
    let body_len = crate::util::char_len(&item.body);
    let cursor = cursor.min(body_len);

    let cmd = if cursor == 0 && body_len > 0 {
        // The root is an item of this page: the empty item goes before it here, not on the source page
        let (ctx, position) = if ctx.in_transclusion && model.state.is_own_item(&item.id) {
            let host = EditContext {
                page_id: item.page_id.clone(),
                level_offset: 0,
                in_transclusion: false,
            };
            (host, position_before(model.state.items(), &item.id))
        } else if ctx.in_transclusion {
            (ctx, ItemPosition::First)
        } else {
            (ctx, position_before(model.state.items(), &item.id))
        };
        run(move |api| async move {
            let _handover = Handover::begin(&api);
            if let Err(e) = insert_empty_item_before(&api, item, cursor, ctx, position).await {
                log_error(format!("Failed to insert item: {e}"));
                release_keys(&api);
            }
        })
    } else if cursor == body_len {
        match transclusion_root_ending_at(
            &item,
            model.state.items(),
            model.state.visible_items(),
            model.state.transcluded_trees(),
        ) {
            Some(root) => append_empty_item_after_transclusion(model, item, cursor, &root),
            None => {
                let new_level = level_for_new(model, &item, &ctx);
                let position =
                    position_after_subtree(model.state.items(), &item, &save_target_id(&item));
                append_empty_item_after(model, item, cursor, ctx, position, new_level, None)
            }
        }
    } else {
        let new_level = level_for_new(model, &item, &ctx);
        let position = position_after_subtree(model.state.items(), &item, &save_target_id(&item));
        run(move |api| async move {
            let _handover = Handover::begin(&api);
            if let Err(e) = split_item_at(&api, item, cursor, ctx, new_level, position).await {
                log_error(format!("Failed to split item: {e}"));
                release_keys(&api);
            }
        })
    };
    Command::all([discard, cmd])
}

fn release_keys(api: &Api<Event>) {
    apply(api, "enter failed: drop held keys", |m| {
        m.key_relay.release();
        done()
    });
}

/// Enter at the beginning: a new empty item goes before the current one, which keeps its text.
async fn insert_empty_item_before(
    api: &Api<Event>,
    item: Item,
    cursor: usize,
    ctx: EditContext,
    position: ItemPosition,
) -> Result<(), crate::effects::ApiError> {
    let (new_item, saved) = join(
        api.create_item(
            &ctx.page_id,
            position.clone(),
            item.level - ctx.level_offset,
            "",
            None,
        ),
        save_body(api, &save_target_id(&item), &item.body),
    )
    .await;
    let new_item = new_item?;
    saved?;
    show_created_item(api, &new_item, &position, &ctx).await;

    let (old_id, new_clone) = (item.id.clone(), new_item.clone());
    apply(api, "enter before: record", move |m| {
        m.undo
            .begin("enter", Some(CursorState::new(&old_id, cursor)));
        m.undo.push_create(&new_clone);
        m.undo
            .commit(Some(CursorState::new(&new_clone.id, 0)), m.clock_ms);
        m.state.add_editing_item(&new_clone.id);
        done()
    });
    focus_editor(api, Some(new_item.id.clone())).await;
    close_editor(api, "enter before: close old editor", item.id.clone());
    Ok(())
}

/// Enter at the end of the last item a transclusion shows. The new item goes on the page
/// that shows the transclusion, right below it. Creating it on the source page would put
/// it outside the transcluded tree, where this page does not show it.
fn append_empty_item_after_transclusion(
    model: &mut Model,
    item: Item,
    cursor: usize,
    root: &Item,
) -> Cmd {
    let ctx = EditContext {
        page_id: root.page_id.clone(),
        level_offset: 0,
        in_transclusion: false,
    };
    let subtree = get_subtree(model.state.items(), &root.id);
    let (position, new_level) = if model.state.is_collapsed(&root.id) || subtree.len() == 1 {
        let last = &subtree[subtree.len() - 1];
        (ItemPosition::after(last.id.clone()), root.level)
    } else {
        // Own children of the root follow the transcluded items, so the new item joins them
        (ItemPosition::after(root.id.clone()), root.level + 1)
    };
    append_empty_item_after(model, item, cursor, ctx, position, new_level, Some(root))
}

/// Enter at the end: the new item is shown at once with a client-side ID and created on
/// the server in the background. It takes its page fields from `page_item`, or from the item.
fn append_empty_item_after(
    model: &mut Model,
    item: Item,
    cursor: usize,
    ctx: EditContext,
    position: ItemPosition,
    new_level: i32,
    page_item: Option<&Item>,
) -> Cmd {
    let page_item = page_item.unwrap_or(&item);
    let new_id = model.new_item_id(model.clock_ms);
    let now = model.clock_ms as i64;
    let optimistic = Item {
        id: new_id.clone(),
        page_id: ctx.page_id.clone(),
        dir_path: page_item.dir_path.clone(),
        name: page_item.name.clone(),
        item_type: page_item.item_type.clone(),
        created_date: now,
        modified_date: now,
        idx: String::new(),
        level: new_level,
        body: String::new(),
        ast: String::new(),
        collapsed: false,
        types: Vec::new(),
        properties: Default::default(),
        transclude_item_id: None,
    };
    model.state.add_item(optimistic.clone(), &position);
    model
        .undo
        .begin("enter", Some(CursorState::new(&item.id, cursor)));
    model.undo.push_create(&optimistic);
    model
        .undo
        .commit(Some(CursorState::new(&new_id, 0)), model.clock_ms);
    model.state.add_editing_item(&new_id);
    model.state.set_focused_item(&new_id, None);

    let in_transclusion = ctx.in_transclusion;
    let create = {
        let (page_id, position, new_id) = (ctx.page_id.clone(), position.clone(), new_id.clone());
        run(move |api| async move {
            match api
                .create_item(
                    &page_id,
                    position,
                    new_level - ctx.level_offset,
                    "",
                    Some(new_id.clone()),
                )
                .await
            {
                Ok(server_item) => {
                    if in_transclusion {
                        reload_shown_page(&api).await;
                    } else {
                        apply(&api, "enter after: server idx", move |m| {
                            m.state.update_item(&new_id, |i| i.idx = server_item.idx);
                            done()
                        });
                    }
                }
                Err(e) => {
                    // The item shown is not on the server; show the page as the server has it
                    log_error(format!("Failed to create item: {e}"));
                    reload_shown_page(&api).await;
                }
            }
        })
    };
    let finish = run(move |api| async move {
        let _handover = Handover::begin(&api);
        focus_editor(&api, Some(new_id)).await;
        let (before, _) = split_at_char(&item.body, cursor);
        if let Err(e) = save_body(&api, &save_target_id(&item), before).await {
            log_error(format!("Failed to save item: {e}"));
        }
        close_editor(&api, "enter after: close old editor", item.id.clone());
    });
    Command::all([create, finish])
}

/// Enter in the middle: the text after the cursor moves to a new item below.
async fn split_item_at(
    api: &Api<Event>,
    item: Item,
    cursor: usize,
    ctx: EditContext,
    new_level: i32,
    position: ItemPosition,
) -> Result<(), crate::effects::ApiError> {
    let (before_text, after_text) = split_at_char(&item.body, cursor);
    let (updated_original, new_item) = join(
        save_body(api, &save_target_id(&item), before_text),
        api.create_item(
            &ctx.page_id,
            position.clone(),
            new_level - ctx.level_offset,
            after_text,
            None,
        ),
    )
    .await;
    let updated_original = updated_original?;
    let new_item = new_item?;
    show_created_item(api, &new_item, &position, &ctx).await;

    let (original, new_clone) = (item.clone(), new_item.clone());
    apply(api, "enter split: record", move |m| {
        m.undo
            .begin("enter", Some(CursorState::new(&original.id, cursor)));
        m.undo.push_update(&original, &updated_original);
        m.undo.push_create(&new_clone);
        m.undo
            .commit(Some(CursorState::new(&new_clone.id, 0)), m.clock_ms);
        m.state.add_editing_item(&new_clone.id);
        m.state.set_focused_item(&new_clone.id, Some(0)); // the caret stays where the text was cut
        done()
    });
    focus_editor(api, Some(new_item.id.clone())).await;
    close_editor(api, "enter split: close old editor", item.id.clone());
    Ok(())
}

/// Puts a newly created item into the state. Inside a transclusion the page is reloaded so the trees are fresh.
async fn show_created_item(
    api: &Api<Event>,
    new_item: &Item,
    position: &ItemPosition,
    ctx: &EditContext,
) {
    if ctx.in_transclusion {
        reload_shown_page(api).await;
    } else {
        let (item, position) = (new_item.clone(), position.clone());
        apply(api, "show created item", move |m| {
            m.state.add_item(item, &position);
            done()
        });
    }
}

/// The shown level of a new item after this one; a transclusion reads the neighbour from the visible list.
fn level_for_new(model: &Model, item: &Item, ctx: &EditContext) -> i32 {
    let items = if ctx.in_transclusion {
        model.state.visible_items()
    } else {
        model.state.items()
    };
    level_for_new_item_after(items, item)
}
