//! The history of the pages that were opened, for a shell without a history of its own
//! (ADR 035).

use crate::app::{done, log_error, run, Cmd, Event, Model, Route};
use crate::screens::{navigate, route};
use crate::view::HistoryView;

/// How many URLs the history keeps. The oldest goes first.
const LIMIT: usize = 100;

/// The URLs that were opened, oldest first, and the one on screen.
#[derive(Debug, Default)]
pub struct History {
    urls: Vec<String>,
    /// The place of the URL on screen in `urls`. Nothing is on screen while `urls` is empty.
    at: usize,
    /// The URL that the core asked to open in place of the one on screen, until it opens.
    in_place: Option<String>,
}

impl History {
    fn push(&mut self, url: &str) {
        let asked_in_place = self.in_place.take_if(|asked| asked == url).is_some();
        if let Some(on_screen) = self.urls.get_mut(self.at).filter(|_| asked_in_place) {
            *on_screen = url.to_string();
            return;
        }
        if self.urls.get(self.at).map(String::as_str) == Some(url) {
            return;
        }
        self.urls.truncate(self.at + 1);
        self.urls.push(url.to_string());
        if self.urls.len() > LIMIT {
            self.urls.remove(0);
        }
        self.at = self.urls.len() - 1;
    }

    fn back(&mut self) -> Option<&str> {
        if !self.can_go_back() {
            return None;
        }
        self.at -= 1;
        Some(&self.urls[self.at])
    }

    fn forward(&mut self) -> Option<&str> {
        if !self.can_go_forward() {
            return None;
        }
        self.at += 1;
        Some(&self.urls[self.at])
    }

    /// Drops the URL on screen in the history, whose page is gone. The URL left on screen is
    /// the one it was reached from, so that going on in the same way skips the URL dropped.
    fn drop_gone(&mut self, forward: bool) {
        self.urls.remove(self.at);
        if forward {
            self.at -= 1;
        }
    }

    fn can_go_back(&self) -> bool {
        self.at > 0
    }

    fn can_go_forward(&self) -> bool {
        self.at + 1 < self.urls.len()
    }

    pub fn view(&self) -> HistoryView {
        HistoryView {
            can_go_back: self.can_go_back(),
            can_go_forward: self.can_go_forward(),
            url: self.urls.get(self.at).cloned(),
        }
    }
}

/// Opens the URL, and keeps it in the history.
pub fn open(model: &mut Model, url: &str, today: String) -> Cmd {
    match Route::parse_url(url) {
        Some(to) => {
            model.history.push(url);
            route::route(model, to, today)
        }
        None => done(),
    }
}

/// Asks the shell to open the page in place of the one on screen, so that going back skips
/// the one on screen.
pub fn replace_page(model: &mut Model, path: &str) -> Cmd {
    model.history.in_place = Some(navigate::page_url(path, None));
    navigate::replace_page(path)
}

/// Opens the URL before the one on screen.
pub fn back(model: &mut Model, today: String) -> Cmd {
    let url = model.history.back().map(str::to_string);
    look(model, url, false, today)
}

/// Opens the URL that was left by going back.
pub fn forward(model: &mut Model, today: String) -> Cmd {
    let url = model.history.forward().map(str::to_string);
    look(model, url, true, today)
}

/// Looks the page of the URL up before it opens, as opening it makes a page that is not
/// there: a page that sync deleted is not made again by going back to it (#763).
fn look(model: &mut Model, url: Option<String>, forward: bool, today: String) -> Cmd {
    let Some(url) = url else {
        return done();
    };
    match Route::parse_url(&url) {
        Some(Route::Page { name, .. }) if name != "today" => run(move |api| async move {
            let found = match api.find_page(&name).await {
                Ok(page) => page.is_some(),
                // The page opens as it did before, when the server cannot tell
                Err(e) => {
                    log_error(format!("Failed to find page {name}: {e}"));
                    true
                }
            };
            api.send_event(Event::HistoryPageFound {
                url,
                found,
                forward,
                today,
            });
        }),
        Some(to) => route::route(model, to, today),
        None => done(),
    }
}

/// The server told whether the page of the URL is there. A page that is there opens; a page
/// that is gone leaves the history, and the URL beyond it is looked up. An answer for a URL
/// the history has left is not shown.
pub fn found(model: &mut Model, url: &str, found: bool, forward: bool, today: String) -> Cmd {
    if model.history.view().url.as_deref() != Some(url) {
        return done();
    }
    if found {
        return match Route::parse_url(url) {
            Some(to) => route::route(model, to, today),
            None => done(),
        };
    }
    model.history.drop_gone(forward);
    let next = if forward {
        model.history.forward()
    } else {
        model.history.back()
    };
    let next = next.map(str::to_string);
    look(model, next, forward, today)
}
