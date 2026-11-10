//! Writes the markdown file of every page again. Run by hand, with the server stopped.
//!
//! Why: a file is written when its page changes, so a file keeps the shape of the server
//! that wrote it last. The files from before the server wrote the modified date of each item
//! have none, and a database built from them gets other dates.

use crate::state::AppState;

/// Writes the file of every page. A file that already says what the page says is left alone.
pub fn write_all(state: &AppState) -> anyhow::Result<()> {
    for page in state.pages.find_all()? {
        state.export.export_page(&page.id)?;
    }
    Ok(())
}
