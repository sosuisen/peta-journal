//! The entrance to the stickies for a shell in another language, called here from Rust. The
//! rules themselves are tested in `client-stickies`; these tests check what crosses the border.
//! It only exists with the feature `ffi`: `cargo test -p client-shell-http --features ffi`.
#![cfg(feature = "ffi")]

use client_shell_http::stickies::{
    smallest_sticky, sticky_colors, sticky_rgb, sticky_transparent, sticky_yellow, sticky_zoom,
    usual_sticky, PetaRgb, PetaSize, PetaSticky, PetaStickyColor, PetaStickyStore,
};

fn sticky(id: &str) -> PetaSticky {
    PetaSticky {
        id: id.to_string(),
        url: Some("/pages/notes".into()),
        x: 10.0,
        y: 20.0,
        width: 300.0,
        height: 400.0,
        color: "#D1E1FF".into(),
        always_on_top: true,
        zoom: 1.5,
        rolled_up: true,
    }
}

// 入口から入れた付箋紙は、すべての値がそのまま読み直せる。消した付箋紙は、読み直しても出てこない。
#[test]
fn a_sticky_put_through_the_entrance_is_read_back_whole() {
    let folder = tempfile::tempdir().unwrap();
    let file = folder.path().join("stickies.json").display().to_string();
    let store = PetaStickyStore::new(file.clone());
    store.put(sticky("a"));
    store.put(sticky("b"));

    store.remove("b".into());

    assert_eq!(PetaStickyStore::new(file).stickies(), [sticky("a")]);
}

// 入口から、新しい付箋紙の色、選べる色、透明、ズーム、色の読み方、ウィンドウの大きさが得られる。
#[test]
fn the_look_of_a_sticky_comes_through_the_entrance() {
    assert_eq!(sticky_yellow(), "#FFF8D0");
    let colors = sticky_colors();
    assert_eq!(colors.len(), 9);
    assert_eq!(
        colors[3],
        PetaStickyColor {
            name: "Blue".into(),
            hex: "#D1E1FF".into(),
        }
    );
    assert_eq!(
        sticky_transparent(),
        PetaStickyColor {
            name: "Transparent".into(),
            hex: "#FFFFFF00".into(),
        }
    );
    assert_eq!(sticky_zoom(1.0, 1), 1.1);
    assert_eq!(sticky_zoom(1.0, -1), 0.9);
    assert_eq!(
        sticky_rgb("#D1E1FF".into()),
        PetaRgb {
            red: 0xD1,
            green: 0xE1,
            blue: 0xFF,
        }
    );
    assert_eq!(
        smallest_sticky(),
        PetaSize {
            width: 280.0,
            height: 200.0,
        }
    );
    assert_eq!(
        usual_sticky(),
        PetaSize {
            width: 400.0,
            height: 520.0,
        }
    );
}
