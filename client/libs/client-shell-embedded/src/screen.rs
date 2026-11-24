//! What screen opens, and what the screen tells of what did not open (ADR 056). The shells of
//! Android and iOS show [`PetaScreen`] as it is.

use std::path::PathBuf;

use client_cloud_sync::find_without_case;
use percent_encoding::percent_decode_str;

use crate::PetaCore;

/// The URL of the journal of today, whatever the date is.
const TODAY: &str = "/today";

/// What the screen tells besides the view model of the core. The web outliner has none of it,
/// so it is not in the view model.
#[boltffi::data]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PetaScreen {
    /// The page that a link named and that is not there, until another page opens. The screen
    /// offers to make it: a touch by mistake must not put an empty page into Dropbox (ADR 031).
    pub missing: Option<String>,
    /// The file of the assets folder that a link named and that is not here, until the next
    /// link is touched or a page opens. No download has brought it yet.
    pub missing_file: Option<String>,
    /// True when no screen opened and the journal of today was not to be made. A new app
    /// waits for the sign-in: a journal made before the first download would be a second file
    /// for a day that Dropbox may hold already.
    pub unopened: bool,
}

#[boltffi::export]
impl PetaCore {
    /// What the screen tells now.
    pub fn screen(&self) -> PetaScreen {
        self.screen.lock().unwrap().clone()
    }

    /// The user touched a link to a page, such as `/pages/notes`. A page that is not there is
    /// not made: nothing opens, and `missing` names the page.
    pub fn follow(&self, url: String) -> PetaScreen {
        self.go(&url, false)
    }

    /// The user wants the page that `missing` names: it is made and opens.
    pub fn make(&self) -> PetaScreen {
        let missing = self.screen.lock().unwrap().missing.clone();
        match missing {
            Some(page) => self.go(&journal_core::page_urls::page_url(&page), true),
            None => self.screen(),
        }
    }

    /// The user wants the journal of today: it is made when it is not there, and opens.
    pub fn today(&self) -> PetaScreen {
        self.go(TODAY, true)
    }

    /// Opens the screen that the app showed when it was used last. The journal of today opens
    /// when there is none, as in a new app, and when its page has gone since. Without `make`
    /// the journal of today is not made, no screen opens, and the screen is `unopened`.
    pub fn restore(&self, make: bool) -> PetaScreen {
        let before = self.last_screen();
        if let Some(url) = &before {
            self.go(url, false);
        }
        let none = before.is_none() || self.screen.lock().unwrap().missing.is_some();
        if none && make {
            self.today();
        }
        self.screen.lock().unwrap().unopened = none && !make;
        self.screen()
    }

    /// Opens the first screen of an app that waited for the sign-in, after the first download
    /// was taken in: it may have brought the journal of today. Nothing happens when a screen
    /// is open.
    pub fn open_first_screen(&self) -> PetaScreen {
        if self.screen.lock().unwrap().unopened {
            self.restore(true)
        } else {
            self.screen()
        }
    }

    /// Back to the page that was open before, as the core keeps the history (ADR 035).
    pub fn back(&self) -> PetaScreen {
        self.turned();
        self.shell.back();
        self.keep_what_is_on_screen();
        self.screen()
    }

    /// Forward to the page that was open before `back`.
    pub fn forward(&self) -> PetaScreen {
        self.turned();
        self.shell.forward();
        self.keep_what_is_on_screen();
        self.screen()
    }

    /// The core asked for the URL, as after a search result was chosen. The user asked for
    /// it, so a page that is not there is made.
    pub fn navigated(&self, url: String) -> PetaScreen {
        self.go(&url, true)
    }

    /// The file of an image of a page. `path` is its path under the Markdown folder, such as
    /// `assets/picture.png`, however its name is spelled there. None when no download has
    /// brought it yet.
    pub fn image(&self, path: String) -> Option<String> {
        self.file(&path)
            .map(|file| file.to_string_lossy().into_owned())
    }

    /// The user touched a link to a file of the assets folder. Answers with the file to open,
    /// as `image` does. When it is not here, `missing_file` names it.
    pub fn attached(&self, path: String) -> Option<String> {
        let file = self.image(path.clone());
        self.screen.lock().unwrap().missing_file = file.is_none().then_some(path);
        file
    }

    /// The core ordered the browser to open an address of the journal server, such as
    /// `/assets/report%20a.pdf` (ADR 083). The app has no server, so the file of the assets
    /// folder opens instead. Answers with the file, as `attached` does. When it is not here,
    /// `missing_file` names it.
    pub fn file_at(&self, address: String) -> Option<String> {
        let path = percent_decode_str(address.trim_start_matches('/')).decode_utf8_lossy();
        self.attached(path.into_owned())
    }
}

impl PetaCore {
    /// Opens the URL, and makes its page when `make` says so.
    fn go(&self, url: &str, make: bool) -> PetaScreen {
        let opened = if make {
            self.open_or_make(url.to_string())
        } else {
            self.open(url.to_string())
        };
        let mut screen = self.screen.lock().unwrap();
        screen.missing_file = None;
        screen.missing = opened.err();
        if screen.missing.is_none() {
            screen.unopened = false;
        }
        screen.clone()
    }

    /// Keeps the URL on screen as the last screen, for the next start. The shell does not know
    /// it after back and forward: the core opened it from its history.
    fn keep_what_is_on_screen(&self) {
        if let Some(url) = self.shell.url() {
            self.keep_last(&url);
        }
    }

    /// The pages of the history opened before, so what the last one told is gone.
    fn turned(&self) {
        let mut screen = self.screen.lock().unwrap();
        screen.missing = None;
        screen.missing_file = None;
    }

    fn file(&self, path: &str) -> Option<PathBuf> {
        find_without_case(&self.markdown, path).filter(|file| file.is_file())
    }
}
