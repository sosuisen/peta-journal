//! The history of the pages that were opened, for a shell without a history of its own
//! (ADR 035).

use crate::app::{done, Cmd, Model, Route};
use crate::screens::route;
use crate::view::HistoryView;

/// The URLs that were opened, oldest first, and the one on screen.
#[derive(Debug, Default)]
pub struct History {
    urls: Vec<String>,
    /// The place of the URL on screen in `urls`. Nothing is on screen while `urls` is empty.
    at: usize,
}

impl History {
    fn push(&mut self, url: &str) {
        if !self.urls.is_empty() {
            self.urls.truncate(self.at + 1);
        }
        self.urls.push(url.to_string());
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

/// Opens the URL before the one on screen.
pub fn back(model: &mut Model, today: String) -> Cmd {
    let url = model.history.back().map(str::to_string);
    show(model, url, today)
}

/// Opens the URL that was left by going back.
pub fn forward(model: &mut Model, today: String) -> Cmd {
    let url = model.history.forward().map(str::to_string);
    show(model, url, today)
}

fn show(model: &mut Model, url: Option<String>, today: String) -> Cmd {
    match url.as_deref().and_then(Route::parse_url) {
        Some(to) => route::route(model, to, today),
        None => done(),
    }
}
