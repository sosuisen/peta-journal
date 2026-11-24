//! The stickies of the SwiftUI and WinUI shells: what is kept of each sticky between two runs
//! of the app, and the rules of how a sticky looks. The shells keep only what their windows do.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

/// The color of the paper of a new sticky.
pub const YELLOW: &str = "#FFF8D0";

/// The colors the menu of a sticky offers, by name. They are the colors of the cards of
/// Petasti (sosuisen/petasti, `cardColors`), without its transparent one: a sticky is a paper.
pub const COLORS: [(&str, &str); 9] = [
    ("Yellow", YELLOW),
    ("Red", "#FFCAC4"),
    ("Green", "#EDFFC4"),
    ("Blue", "#D1E1FF"),
    ("Orange", "#FFDD9E"),
    ("Purple", "#F8D0FF"),
    ("White", "#FFFFFF"),
    ("Gray", "#E0E0E0"),
    ("Light Gray", "#F8F8F8"),
];

/// The color of a sticky without paper: only what is written on it is seen. It is
/// `#RRGGBBAA`, with no alpha. It is not among `COLORS`, the colors of paper: the menus of the
/// shells offer it after them.
pub const TRANSPARENT: (&str, &str) = ("Transparent", "#FFFFFF00");

/// A size of a window, in points as they are at a scale of 100%.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

/// The smallest window of a sticky that is not rolled up.
pub const SMALLEST: Size = Size {
    width: 280.0,
    height: 200.0,
};

/// The window of a new sticky.
pub const USUAL: Size = Size {
    width: 400.0,
    height: 520.0,
};

/// What is kept of one sticky between two runs of the app.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StickySettings {
    pub id: String,
    /// What the sticky shows, as the URL that opens it. `None` when it shows no page.
    #[serde(default)]
    pub url: Option<String>,
    /// Where the window is, as the shell counts: macOS from the bottom left of the main screen,
    /// Windows from the top left over all screens. The file stays on one computer.
    pub x: f64,
    pub y: f64,
    /// The size of the window. Rolled up, it is still the size of the whole sticky.
    pub width: f64,
    pub height: f64,
    /// The color of the paper, as `#RRGGBB`, or `TRANSPARENT` for no paper.
    #[serde(default = "yellow")]
    pub color: String,
    /// The window stays above the windows of other apps.
    #[serde(default)]
    pub always_on_top: bool,
    /// How large the page is shown: 1 is the usual size.
    #[serde(default = "usual_zoom")]
    pub zoom: f64,
    /// Only the title bar of the window is shown.
    #[serde(default)]
    pub rolled_up: bool,
}

// A file of an older version of the app lacks what was added since: such a value is the one
// of a new sticky
fn yellow() -> String {
    YELLOW.to_string()
}

fn usual_zoom() -> f64 {
    1.0
}

/// The stickies as a file keeps them. Every change is written at once: the app may be ended at
/// any time.
pub struct StickyStore {
    file: PathBuf,
    stickies: Vec<StickySettings>,
}

impl StickyStore {
    /// Reads the file. A file that is not there, or cannot be read, gives no stickies.
    pub fn open(file: impl Into<PathBuf>) -> Self {
        let file = file.into();
        let stickies = fs::read(&file)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        StickyStore { file, stickies }
    }

    pub fn stickies(&self) -> &[StickySettings] {
        &self.stickies
    }

    /// Adds the sticky, or changes the one with its id.
    pub fn put(&mut self, sticky: StickySettings) {
        match self.stickies.iter_mut().find(|kept| kept.id == sticky.id) {
            Some(kept) if *kept == sticky => return,
            Some(kept) => *kept = sticky,
            None => self.stickies.push(sticky),
        }
        self.write();
    }

    pub fn remove(&mut self, id: &str) {
        self.stickies.retain(|kept| kept.id != id);
        self.write();
    }

    /// A failure is not told: the stickies on screen go on, and the next change writes again.
    fn write(&self) {
        let _ = write_beside(&self.file, &self.stickies);
    }
}

/// Writes the stickies beside the file and then puts them in its place: the app may be ended
/// in the middle of the writing, and half a file would lose all the stickies.
fn write_beside(file: &Path, stickies: &[StickySettings]) -> std::io::Result<()> {
    if let Some(folder) = file.parent() {
        fs::create_dir_all(folder)?;
    }
    let mut beside = file.as_os_str().to_owned();
    beside.push(".new");
    fs::write(&beside, serde_json::to_vec_pretty(stickies)?)?;
    fs::rename(&beside, file)
}

/// The zoom `steps` tenths larger (smaller when negative), in whole tenths between three tenths
/// and three. Whole tenths, as steps of 0.1 do not add up to them. With no step, it is the
/// zoom kept in a file, put in the same bounds.
pub fn zoomed(zoom: f64, steps: i32) -> f64 {
    ((zoom * 10.0).round() + f64::from(steps)).clamp(3.0, 30.0) / 10.0
}

/// The red, green and blue of the color that `#RRGGBB` names. Anything else is the yellow of a
/// new sticky.
pub fn rgb(hex: &str) -> (u8, u8, u8) {
    let value = hex
        .strip_prefix('#')
        .filter(|digits| digits.len() == 6 && digits.bytes().all(|b| b.is_ascii_hexdigit()))
        .and_then(|digits| u32::from_str_radix(digits, 16).ok());
    match value {
        Some(value) => ((value >> 16) as u8, (value >> 8) as u8, value as u8),
        None => rgb(YELLOW),
    }
}
