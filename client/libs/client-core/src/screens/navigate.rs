//! Router navigation, ported from `navigation.service.ts`.

use crate::app::{Cmd, Model};
use crate::effects::{NavigateOp, UiOp};
use crate::screens::atlas::folder_path;
use crux_core::Command;

/// Navigates to a page by its path (optionally with a dir path prefix like `cat/mypage`).
pub fn to_page(
    path: &str,
    new_tab: bool,
    new_window: bool,
    replace: bool,
    fragment: Option<&str>,
) -> Cmd {
    let url = page_url(path, fragment);
    if new_tab || new_window {
        Command::notify_shell(UiOp::OpenWindow { url, new_window }).into()
    } else {
        to_url(url, replace)
    }
}

/// The URL of a page by its path, with the item to go to after a `#`.
pub fn page_url(path: &str, fragment: Option<&str>) -> String {
    let mut url = format!("/pages/{path}");
    if let Some(fragment) = fragment {
        url.push('#');
        url.push_str(fragment);
    }
    url
}

/// Opens a page in this tab.
pub fn open_page(path: &str) -> Cmd {
    to_page(path, false, false, false, None)
}

/// Opens a page in this tab in place of the current URL, so the back button skips it.
pub fn replace_page(path: &str) -> Cmd {
    to_page(path, false, false, true, None)
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
