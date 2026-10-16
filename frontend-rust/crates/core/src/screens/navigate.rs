//! Router navigation, ported from `navigation.service.ts`.

use crate::app::Cmd;
use crate::effects::{NavigateOp, UiOp};
use crux_core::Command;

/// Navigates to a page by its path (optionally with a dir path prefix like `cat/mypage`).
pub fn to_page(
    path: &str,
    new_tab: bool,
    new_window: bool,
    replace: bool,
    fragment: Option<&str>,
) -> Cmd {
    let mut url = format!("/pages/{path}");
    if let Some(fragment) = fragment {
        url.push('#');
        url.push_str(fragment);
    }
    if new_tab || new_window {
        Command::notify_shell(UiOp::OpenWindow { url, new_window }).into()
    } else {
        to_url(url, replace)
    }
}

pub fn to_url(url: impl Into<String>, replace: bool) -> Cmd {
    Command::notify_shell(NavigateOp {
        url: url.into(),
        replace,
    })
    .into()
}

pub fn set_title(title: impl Into<String>) -> Cmd {
    Command::notify_shell(UiOp::SetTitle(title.into())).into()
}
