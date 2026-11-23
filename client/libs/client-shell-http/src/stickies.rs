//! The entrance to `client-stickies` for the SwiftUI and WinUI shells: what is kept of each
//! sticky, and the rules of how a sticky looks.

use client_stickies::{StickySettings, StickyStore};
use std::sync::Mutex;

/// What is kept of one sticky between two runs of the app. `client_stickies::StickySettings`
/// tells what each value is.
#[boltffi::data]
#[derive(Clone, Debug, PartialEq)]
pub struct PetaSticky {
    pub id: String,
    pub url: Option<String>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub color: String,
    pub always_on_top: bool,
    pub zoom: f64,
    pub rolled_up: bool,
}

impl From<&StickySettings> for PetaSticky {
    fn from(kept: &StickySettings) -> Self {
        PetaSticky {
            id: kept.id.clone(),
            url: kept.url.clone(),
            x: kept.x,
            y: kept.y,
            width: kept.width,
            height: kept.height,
            color: kept.color.clone(),
            always_on_top: kept.always_on_top,
            zoom: kept.zoom,
            rolled_up: kept.rolled_up,
        }
    }
}

impl From<PetaSticky> for StickySettings {
    fn from(sticky: PetaSticky) -> Self {
        StickySettings {
            id: sticky.id,
            url: sticky.url,
            x: sticky.x,
            y: sticky.y,
            width: sticky.width,
            height: sticky.height,
            color: sticky.color,
            always_on_top: sticky.always_on_top,
            zoom: sticky.zoom,
            rolled_up: sticky.rolled_up,
        }
    }
}

/// A color the menu of a sticky offers.
#[boltffi::data]
#[derive(Clone, Debug, PartialEq)]
pub struct PetaStickyColor {
    pub name: String,
    pub hex: String,
}

/// The red, green and blue of a color.
#[boltffi::data]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PetaRgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

/// A size of a window, in points as they are at a scale of 100%.
#[boltffi::data]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PetaSize {
    pub width: f64,
    pub height: f64,
}

/// The stickies as a file keeps them. Every change is written at once.
#[boltffi::export]
pub struct PetaStickyStore {
    store: Mutex<StickyStore>,
}

#[boltffi::export]
impl PetaStickyStore {
    /// Reads the file at the path. A file that is not there, or cannot be read, gives no
    /// stickies.
    pub fn new(file: String) -> Self {
        PetaStickyStore {
            store: Mutex::new(StickyStore::open(file)),
        }
    }

    pub fn stickies(&self) -> Vec<PetaSticky> {
        let store = self.store.lock().unwrap();
        store.stickies().iter().map(PetaSticky::from).collect()
    }

    /// Adds the sticky, or changes the one with its id.
    pub fn put(&self, sticky: PetaSticky) {
        self.store.lock().unwrap().put(sticky.into());
    }

    pub fn remove(&self, id: String) {
        self.store.lock().unwrap().remove(&id);
    }
}

/// The color of the paper of a new sticky, as `#RRGGBB`.
#[boltffi::export]
pub fn sticky_yellow() -> String {
    client_stickies::YELLOW.to_string()
}

/// The colors the menu of a sticky offers, in their order.
#[boltffi::export]
pub fn sticky_colors() -> Vec<PetaStickyColor> {
    let colors = client_stickies::COLORS.iter();
    colors
        .map(|(name, hex)| PetaStickyColor {
            name: name.to_string(),
            hex: hex.to_string(),
        })
        .collect()
}

/// The color of a sticky without paper. The menu offers it after the colors, where the shell
/// draws it.
#[boltffi::export]
pub fn sticky_transparent() -> PetaStickyColor {
    let (name, hex) = client_stickies::TRANSPARENT;
    PetaStickyColor {
        name: name.to_string(),
        hex: hex.to_string(),
    }
}

/// The zoom `steps` tenths larger (smaller when negative), between three tenths and three.
/// With no step, it is a kept zoom put in the same bounds.
#[boltffi::export]
pub fn sticky_zoom(zoom: f64, steps: i32) -> f64 {
    client_stickies::zoomed(zoom, steps)
}

/// The color that `#RRGGBB` names. Anything else is the yellow of a new sticky.
#[boltffi::export]
pub fn sticky_rgb(hex: String) -> PetaRgb {
    let (red, green, blue) = client_stickies::rgb(&hex);
    PetaRgb { red, green, blue }
}

/// The smallest window of a sticky that is not rolled up.
#[boltffi::export]
pub fn smallest_sticky() -> PetaSize {
    let size = client_stickies::SMALLEST;
    PetaSize {
        width: size.width,
        height: size.height,
    }
}

/// The window of a new sticky.
#[boltffi::export]
pub fn usual_sticky() -> PetaSize {
    let size = client_stickies::USUAL;
    PetaSize {
        width: size.width,
        height: size.height,
    }
}
