//! The bodies of items at the border with the server (ADR 070). The shell and the core see a
//! blank line in a body as a blank line. In the file, a blank line between two blocks would end
//! the item, so the server has a line of one backslash there. Every body that comes from the
//! server is read with `flatmarkdown::body_from_file`, and every body that goes to it is
//! written with `flatmarkdown::body_to_file`.

use crate::effects::{ApiOp, ApiOutput};
use crate::model::{BatchOpInput, Item};

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
/// same body. The two have the same lines. Only a blank line or a line of backslashes differs.
/// A cursor on a blank line, or on the backslash in its place, goes to the start of the line.
fn moved_cursor(from: &str, to: &str, cursor: usize) -> usize {
    let head: String = from.chars().take(cursor).collect();
    let row = head.matches('\n').count();
    let column = head
        .rsplit('\n')
        .next()
        .map_or(0, |line| line.chars().count());
    let from_line = from.split('\n').nth(row).unwrap_or_default();
    let to_line = to.split('\n').nth(row).unwrap_or_default();
    let blank = from_line.trim().is_empty() || to_line.trim().is_empty();
    let column = if from_line == to_line {
        column
    } else if blank || column == 0 {
        0
    } else {
        // A line of backslashes has one more in the file. The added one goes before the
        // cursor, so the text before the cursor keeps all its backslashes.
        column + to_line.chars().count() - from_line.chars().count()
    };
    let before: usize = to
        .split('\n')
        .take(row)
        .map(|l| l.chars().count() + 1)
        .sum();
    before + column
}

/// The answer of the server, with the bodies as the shell sees them.
pub fn output_to_shell(mut output: ApiOutput) -> ApiOutput {
    match &mut output {
        ApiOutput::Page(page) => {
            items_to_shell(&mut page.items);
            trees_to_shell(page.transcluded_trees.values_mut());
        }
        ApiOutput::Journal(journal) => {
            items_to_shell(&mut journal.items);
            trees_to_shell(journal.transcluded_trees.values_mut());
        }
        ApiOutput::JournalBefore(journal) => {
            items_to_shell(&mut journal.items);
            trees_to_shell(journal.transcluded_trees.values_mut());
        }
        ApiOutput::DocPage(page) => items_to_shell(&mut page.items),
        ApiOutput::MovedToJournal(moved) => items_to_shell(&mut moved.items),
        ApiOutput::Item(item) => item_to_shell(item),
        ApiOutput::Items(items) => items_to_shell(items),
        ApiOutput::Backlinks(entries) => entries
            .iter_mut()
            .filter_map(|entry| entry.source_item.as_mut())
            .for_each(item_to_shell),
        ApiOutput::Pasted(pasted) => {
            let cursor = &mut pasted.cursor;
            for item in &mut pasted.items {
                let shell = flatmarkdown::body_from_file(&item.body);
                if item.id == cursor.item_id {
                    cursor.position = moved_cursor(&item.body, &shell, cursor.position);
                }
                item.body = shell;
            }
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
        // The answers without items. They are listed, so that a new answer with items is a
        // build error here and not a body that the shell shows with its backslash lines.
        ApiOutput::FoundPage(_)
        | ApiOutput::PageInfo(_)
        | ApiOutput::Pages(_)
        | ApiOutput::SearchResults(_)
        | ApiOutput::Config(_)
        | ApiOutput::PropertyDefinitions(_)
        | ApiOutput::Folder(_)
        | ApiOutput::TreeExport(_)
        | ApiOutput::ItemIds(_)
        | ApiOutput::Uploaded(_)
        | ApiOutput::Launched(_)
        | ApiOutput::PageNameTaken(_)
        | ApiOutput::Done => {}
    }
    output
}

/// The items of the transcluded trees, one list for each source item.
fn trees_to_shell<'a>(trees: impl Iterator<Item = &'a mut Vec<Item>>) {
    trees.for_each(|items| items_to_shell(items));
}

fn items_to_shell(items: &mut [Item]) {
    items.iter_mut().for_each(item_to_shell);
}

fn item_to_shell(item: &mut Item) {
    item.body = flatmarkdown::body_from_file(&item.body);
}
