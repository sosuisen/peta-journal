//! The bodies of items at the border with the server (ADR 070). The shell and the core see a
//! blank line in a body as a blank line. In the file, a blank line between two blocks would end
//! the item, so the server has a line of one backslash there. Every body that comes from the
//! server is read with `flatmarkdown::body_from_file`, and every body that goes to it is
//! written with `flatmarkdown::body_to_file`.

use crate::effects::{ApiOp, ApiOutput};
use crate::model::{BatchOpInput, Item, TranscludedTrees};

/// The request to the server, with the bodies in the form of the file.
pub fn op_to_file(mut op: ApiOp) -> ApiOp {
    match &mut op {
        ApiOp::CreateItem { body, .. } => body_to_file(body),
        ApiOp::UpdateItem { body, .. }
        | ApiOp::MoveItems { body, .. }
        | ApiOp::MoveItemsToJournal { body, .. }
        | ApiOp::StashItem { body, .. } => body.iter_mut().for_each(body_to_file),
        ApiOp::MergeItems {
            body, into_body, ..
        } => body.iter_mut().chain(into_body).for_each(body_to_file),
        ApiOp::PasteText { body, cursor, .. } | ApiOp::SplitItem { body, cursor, .. } => {
            let file = flatmarkdown::body_to_file(body);
            *cursor = moved_cursor(body, &file, *cursor);
            *body = file;
        }
        ApiOp::ApplyBatch { ops } => ops.iter_mut().for_each(|op| match op {
            BatchOpInput::Create { body, .. } => body_to_file(body),
            BatchOpInput::Update { body, .. } => body.iter_mut().for_each(body_to_file),
            BatchOpInput::Move { .. } | BatchOpInput::Delete { .. } => {}
        }),
        _ => {}
    }
    op
}

fn body_to_file(body: &mut String) {
    *body = flatmarkdown::body_to_file(body);
}

/// The character `cursor` of `from`, moved to the same place in `to`, the other form of the
/// same body. The two have the same lines. Only a blank line or a line of backslashes differs,
/// and a cursor on a blank line, or on the backslash in its place, goes to the start of it.
fn moved_cursor(from: &str, to: &str, cursor: usize) -> usize {
    let mut before = 0;
    let mut rest = cursor;
    for (from_line, to_line) in from.split('\n').zip(to.split('\n')) {
        let from_length = from_line.chars().count();
        let to_length = to_line.chars().count();
        if rest <= from_length {
            let blank = from_line.trim().is_empty() || to_line.trim().is_empty();
            let column = if from_line != to_line && blank {
                0
            } else {
                rest
            };
            return before + column.min(to_length);
        }
        rest -= from_length + 1;
        before += to_length + 1;
    }
    to.chars().count()
}

/// The answer of the server, with the bodies as the shell sees them.
pub fn output_to_shell(mut output: ApiOutput) -> ApiOutput {
    match &mut output {
        ApiOutput::Page(page) => {
            items_to_shell(&mut page.items);
            trees_to_shell(&mut page.transcluded_trees);
        }
        ApiOutput::JournalBefore(journal) => {
            items_to_shell(&mut journal.items);
            trees_to_shell(&mut journal.transcluded_trees);
        }
        ApiOutput::Item(item) => item_to_shell(item),
        ApiOutput::Items(items) => items_to_shell(items),
        ApiOutput::Backlinks(entries) => entries
            .iter_mut()
            .filter_map(|entry| entry.source_item.as_mut())
            .for_each(item_to_shell),
        ApiOutput::Pasted(pasted) => {
            if let Some(item) = pasted.items.iter().find(|i| i.id == pasted.cursor.item_id) {
                let shell = flatmarkdown::body_from_file(&item.body);
                pasted.cursor.position = moved_cursor(&item.body, &shell, pasted.cursor.position);
            }
            items_to_shell(&mut pasted.items);
        }
        ApiOutput::Split(split) => {
            item_to_shell(&mut split.item);
            item_to_shell(&mut split.created);
        }
        ApiOutput::Stashed(stashed) => {
            item_to_shell(&mut stashed.heading);
            items_to_shell(&mut stashed.items);
        }
        ApiOutput::Batched(batched) => {
            items_to_shell(&mut batched.items);
            items_to_shell(&mut batched.created);
        }
        _ => {}
    }
    output
}

fn trees_to_shell(trees: &mut Option<TranscludedTrees>) {
    trees
        .iter_mut()
        .flat_map(|trees| trees.values_mut())
        .for_each(|items| items_to_shell(items));
}

fn items_to_shell(items: &mut [Item]) {
    items.iter_mut().for_each(item_to_shell);
}

fn item_to_shell(item: &mut Item) {
    item.body = flatmarkdown::body_from_file(&item.body);
}
