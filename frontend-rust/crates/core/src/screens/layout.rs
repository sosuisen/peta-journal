//! The header's actions, ported from `main-layout.ts`: a new page and deleting the shown page.

use crate::app::{apply, done, run, Cmd, Model, Screen};
use crate::effects::UiOp;
use crate::screens::{navigate, search, sidebar};
use crate::util::date::generate_untitled_title;
use crate::util::filename::{is_journal_file_name, validate_file_name};
use crux_core::Command;

/// Creates a new document page. Uses the search query as the title when there is one,
/// otherwise asks for a title; an empty title becomes a timestamp-based one.
pub fn create_new_page(model: &mut Model, search_query: String) -> Cmd {
    let query = search_query.trim().to_string();
    let clear = if query.is_empty() { done() } else { search::clear(model) };
    let open = run(move |api| async move {
        let title = if query.is_empty() {
            match api.prompt("Page title:").await {
                Some(t) => t,
                None => return,
            }
        } else {
            query
        };
        let mut title = title.trim().to_string();
        if title.is_empty() {
            title = generate_untitled_title(api.local_now().await.0);
        }
        if !is_journal_file_name(&title) {
            if let Some(error) = validate_file_name(&title) {
                api.notify(UiOp::Alert(error));
                return;
            }
        }
        apply(&api, "new page", move |_| navigate::to_page(&title, false, false, false, None));
    });
    Command::all([clear, open])
}

/// The name in the URL of the shown page; None on the settings screen or `today`.
fn shown_file_name(model: &Model) -> Option<String> {
    match &model.screen {
        Screen::Journal(j) if j.page_name != "today" && !j.page_name.is_empty() => Some(j.page_name.clone()),
        Screen::Page(p) => Some(p.page_name.clone()),
        _ => None,
    }
}

/// Deletes the shown page after confirmation, then goes to today's journal.
pub fn delete_current_page(model: &mut Model) -> Cmd {
    let Some(file_name) = shown_file_name(model) else { return done() };
    run(move |api| async move {
        if !api.confirm(&format!("Delete \"{file_name}\"?")).await {
            return;
        }
        let page = if is_journal_file_name(&file_name) {
            api.get_or_create_journal(&file_name).await
        } else {
            let (dir, title) = super::page::split_doc_path(&file_name);
            api.get_or_create_doc(&title, &dir).await
        };
        let page = match page {
            Ok(p) => p.page,
            Err(e) => return api.notify(UiOp::Alert(e.0)),
        };
        if let Err(e) = api.delete_page(&page.id).await {
            api.notify(UiOp::Alert(if e.0.is_empty() { "Failed to delete page".into() } else { e.0 }));
            return;
        }
        apply(&api, "page deleted", |_| {
            Command::all([sidebar::refresh_recent_pages(), navigate::to_url("/today", false)])
        });
    })
}
