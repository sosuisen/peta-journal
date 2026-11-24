//! Router navigation, ported from `navigation.service.ts`.

use crate::app::{Cmd, Model};
use crate::effects::{NavigateOp, UiOp};
use crate::keys::{Held, OpenIn};
use crate::rich_text::RichLink;
use crate::screens::atlas::folder_path;
use client_page_links::AppLinks;
use crux_core::Command;

/// Navigates to a page by its path (optionally with a dir path prefix like `cat/mypage`).
pub fn to_page(path: &str, replace: bool, fragment: Option<&str>) -> Cmd {
    to_url(page_url(path, fragment), replace)
}

/// Opens a link in the text of an item that was clicked with the keys `held` (ADR 083). A page
/// opens where the keys say. A link out of the journal, and a file of the assets folder, open in
/// the browser whatever keys are held.
pub fn link_clicked(link: RichLink, held: Held) -> Cmd {
    match link {
        RichLink::Page { url } => open_url_in(url, held.open_in()),
        RichLink::Web { url } => in_browser(url),
        RichLink::File { path } => in_browser(
            AppLinks {
                app_url: "",
                tags_dir: "",
            }
            .asset(&path),
        ),
    }
}

fn in_browser(address: String) -> Cmd {
    Command::notify_shell(UiOp::OpenInBrowser { address }).into()
}

/// Opens the URL of a page of the app, such as `/pages/notes`, where `open_in` says (ADR 083).
pub fn open_url_in(url: String, open_in: OpenIn) -> Cmd {
    match open_in {
        OpenIn::Here => to_url(url, false),
        OpenIn::NewView => Command::notify_shell(UiOp::OpenNewView { url }).into(),
        OpenIn::Browser => in_browser(url),
    }
}

/// The URL of a page by its path, with the item to go to after a `#`, encoded.
pub fn page_url(path: &str, fragment: Option<&str>) -> String {
    match fragment {
        Some(item) => page_urls::item_url(path, item),
        None => page_urls::page_url(path),
    }
}

/// Opens a page in this tab.
pub fn open_page(path: &str) -> Cmd {
    to_page(path, false, None)
}

/// Opens a page in this tab in place of the current URL, so the back button skips it.
pub fn replace_page(path: &str) -> Cmd {
    to_page(path, true, None)
}

/// Opens a folder's page as the folder was last seen: its atlas or its index.
pub fn open_folder(model: &Model, path: &str) -> Cmd {
    open_page(&folder_path(model, path))
}

pub fn to_url(url: impl Into<String>, replace: bool) -> Cmd {
    Command::notify_shell(NavigateOp {
        url: url.into(),
        replace,
    })
    .into()
}

/// Names what is open. The shell hears of the name, and the view model keeps it for the title
/// of a window (`ViewModel::title`).
pub fn set_title(model: &mut Model, title: impl Into<String>) -> Cmd {
    model.title = title.into();
    Command::notify_shell(UiOp::SetTitle(model.title.clone())).into()
}
