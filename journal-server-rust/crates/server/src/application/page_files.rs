//! Writes the markdown file of every page again. Run by hand, with the server stopped.
//!
//! Why: a file is written when its page changes, so a file keeps the shape of the server
//! that wrote it last. The files from before the server wrote the modified date of each item
//! have none, and a database built from them gets other dates.

use hmtid::Hmtid;

use crate::config::Settings;
use crate::state::{AppState, BuildOptions};

/// How many page files were written, of how many pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Written {
    pub files: usize,
    pub pages: usize,
}

/// Opens the database and the search index as the server does, writes the files and returns
/// what to tell the person who ran it. The server must not run: it holds the search index.
pub fn run(settings: Settings) -> anyhow::Result<String> {
    let state = AppState::build(settings, BuildOptions { interactive: false })?;
    let written = write_all(&state)?;
    state.shutdown();
    Ok(format!(
        "{} of {} page files written",
        written.files, written.pages
    ))
}

/// Writes the file of every page. A file that already says what the page says is left alone.
pub fn write_all(state: &AppState) -> anyhow::Result<Written> {
    // First what the folder has and the database has not: a file that waits to be taken in
    // is not written over
    state.reconcile.scan();
    let pages = state.pages.find_all()?;
    let mut files = 0;
    for page in &pages {
        let before = written_content(state, &page.id)?;
        state.export.export_page(&page.id)?;
        if written_content(state, &page.id)? != before {
            files += 1;
        }
    }
    Ok(Written {
        files,
        pages: pages.len(),
    })
}

/// The hashes of what the files of the page were last written with.
fn written_content(state: &AppState, page_id: &Hmtid) -> anyhow::Result<Vec<String>> {
    Ok(state
        .file_sync_state
        .find_by_page_id(page_id)?
        .into_iter()
        .map(|file| file.content_hash)
        .collect())
}
