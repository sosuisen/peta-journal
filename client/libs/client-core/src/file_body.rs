//! The bodies of items at the border with the server (ADR 070). The shell and the core see a
//! blank line in a body as a blank line. In the file, a blank line between two blocks would end
//! the item, so the server has a line of one backslash there. Every body that comes from the
//! server is read with `flatmarkdown::body_from_file`, and every body that goes to it is
//! written with `flatmarkdown::body_to_file`.

use crate::effects::{ApiOp, ApiOutput};
use crate::model::Item;

/// The request to the server, with the bodies in the form of the file.
pub fn op_to_file(op: ApiOp) -> ApiOp {
    match op {
        ApiOp::UpdateItem {
            item_id,
            body,
            level,
            collapsed,
        } => ApiOp::UpdateItem {
            item_id,
            body: body.as_deref().map(flatmarkdown::body_to_file),
            level,
            collapsed,
        },
        ApiOp::PasteText {
            item_id,
            body,
            cursor,
            text,
            mode,
        } => {
            let file = flatmarkdown::body_to_file(&body);
            let cursor = moved_cursor(&body, &file, cursor);
            ApiOp::PasteText {
                item_id,
                body: file,
                cursor,
                text,
                mode,
            }
        }
        ApiOp::SplitItem {
            item_id,
            root_id,
            body,
            cursor,
        } => {
            let file = flatmarkdown::body_to_file(&body);
            let cursor = moved_cursor(&body, &file, cursor);
            ApiOp::SplitItem {
                item_id,
                root_id,
                body: file,
                cursor,
            }
        }
        other => other,
    }
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
pub fn output_to_shell(output: ApiOutput) -> ApiOutput {
    match output {
        ApiOutput::Page(mut page) => {
            items_to_shell(&mut page.items);
            ApiOutput::Page(page)
        }
        ApiOutput::Pasted(mut pasted) => {
            if let Some(item) = pasted.items.iter().find(|i| i.id == pasted.cursor.item_id) {
                let shell = flatmarkdown::body_from_file(&item.body);
                pasted.cursor.position = moved_cursor(&item.body, &shell, pasted.cursor.position);
            }
            items_to_shell(&mut pasted.items);
            ApiOutput::Pasted(pasted)
        }
        ApiOutput::Split(mut split) => {
            item_to_shell(&mut split.item);
            item_to_shell(&mut split.created);
            ApiOutput::Split(split)
        }
        other => other,
    }
}

fn items_to_shell(items: &mut [Item]) {
    items.iter_mut().for_each(item_to_shell);
}

fn item_to_shell(item: &mut Item) {
    item.body = flatmarkdown::body_from_file(&item.body);
}
