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
        ApiOp::SplitItem {
            item_id,
            root_id,
            body,
            cursor,
        } => {
            let file = flatmarkdown::body_to_file(&body);
            let cursor = cursor_to_file(&body, &file, cursor);
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

/// The character `cursor` of `body`, moved to the same place in `file`, its form in the file.
/// The two have the same lines. Only a blank line or a line of backslashes differs, and a
/// cursor on a blank line goes to the start of its backslash.
fn cursor_to_file(body: &str, file: &str, cursor: usize) -> usize {
    let mut before = 0;
    let mut rest = cursor;
    for (line, file_line) in body.split('\n').zip(file.split('\n')) {
        let length = line.chars().count();
        let file_length = file_line.chars().count();
        if rest <= length {
            let column = if line != file_line && line.trim().is_empty() {
                0
            } else {
                rest
            };
            return before + column.min(file_length);
        }
        rest -= length + 1;
        before += file_length + 1;
    }
    file.chars().count()
}

/// The answer of the server, with the bodies as the shell sees them.
pub fn output_to_shell(output: ApiOutput) -> ApiOutput {
    match output {
        ApiOutput::Page(mut page) => {
            items_to_shell(&mut page.items);
            ApiOutput::Page(page)
        }
        other => other,
    }
}

fn items_to_shell(items: &mut [Item]) {
    for item in items {
        item.body = flatmarkdown::body_from_file(&item.body);
    }
}
