//! The screens around the outline: routing, the journal and page headers, the sidebar,
//! search, settings and the server connection.

pub mod atlas;
pub mod calendar;
pub mod day;
pub mod history;
pub mod journal;
pub mod layout;
pub mod navigate;
pub mod page;
pub mod search;
pub mod settings;
pub mod sidebar;
pub mod sse;

pub mod start {
    use crate::app::{Cmd, Model};
    use crate::screens::{day, settings, sidebar, sse};
    use crux_core::Command;

    /// The shell is up: connect to the server, load the settings and the sidebar, and watch
    /// the date.
    pub fn start(model: &mut Model) -> Cmd {
        Command::all([
            sse::connect(model),
            day::watch(model),
            settings::load_server_config(),
            settings::load_property_definitions(),
            sidebar::refresh_recent_pages(),
        ])
    }
}

pub mod route {
    use crate::app::{done, Cmd, Model, Route, Screen};
    use crate::screens::{journal, navigate, page, settings};
    use crate::util::filename::is_journal_file_name;
    use crux_core::Command;

    /// The URL changed: shows the journal, a doc page or the settings.
    pub fn route(model: &mut Model, route: Route, today: String) -> Cmd {
        model.today = today;
        match route {
            Route::Settings => {
                model.screen = Screen::Settings;
                Command::all([
                    navigate::set_title(model, "Settings"),
                    settings::load_server_config(),
                ])
            }
            Route::Page { name, fragment } => {
                let same_page = match &model.screen {
                    Screen::Journal(j) => j.page_name == name,
                    Screen::Page(p) => p.page_name == name,
                    _ => false,
                };
                if same_page && fragment.is_none() {
                    return done();
                }
                open(model, &name, fragment)
            }
        }
    }

    /// Opens the page of the name, also when it is the one shown.
    pub fn open(model: &mut Model, name: &str, fragment: Option<String>) -> Cmd {
        // A new page starts with clean undo stacks and no editor open
        model.undo.clear();
        model.state.remove_all_editing_items();
        model.selection.exit(&mut model.state);
        let _ = crate::ops::wiki::close(model);
        if name == "today" || is_journal_file_name(name) {
            journal::open(model, name, fragment)
        } else {
            page::open(model, name, fragment)
        }
    }
}
