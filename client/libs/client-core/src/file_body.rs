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
        other => other,
    }
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
