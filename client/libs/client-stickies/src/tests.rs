use super::*;
use tempfile::TempDir;

/// A file in a new folder of its own, which is not made yet.
fn new_file() -> (TempDir, PathBuf) {
    let folder = TempDir::new().unwrap();
    let file = folder.path().join("PetaStickies").join("stickies.json");
    (folder, file)
}

fn sticky(id: &str, url: Option<&str>) -> StickySettings {
    StickySettings {
        id: id.to_string(),
        url: url.map(str::to_string),
        x: 10.0,
        y: 20.0,
        width: 300.0,
        height: 400.0,
        color: YELLOW.to_string(),
        always_on_top: false,
        zoom: 1.0,
        rolled_up: false,
    }
}

// ファイルがまだないときは、付箋紙は1つもない。
#[test]
fn there_are_no_stickies_before_anything_is_kept() {
    let (_folder, file) = new_file();
    assert_eq!(StickyStore::open(&file).stickies(), []);
}

// 足した付箋紙は、読み直しても残る。順番も同じである。
#[test]
fn stickies_that_were_put_are_there_after_reading() {
    let (_folder, file) = new_file();
    let mut store = StickyStore::open(&file);
    store.put(sticky("a", Some("/pages/notes")));
    store.put(sticky("b", None));

    let read = StickyStore::open(&file);
    assert_eq!(
        read.stickies(),
        [sticky("a", Some("/pages/notes")), sticky("b", None)]
    );
}

// 同じ付箋紙をもう一度入れると、増えずに、中身が変わる。
#[test]
fn a_sticky_that_is_put_again_is_changed_and_not_added() {
    let (_folder, file) = new_file();
    let mut store = StickyStore::open(&file);
    store.put(sticky("a", None));
    store.put(sticky("b", None));
    let moved = StickySettings {
        x: 99.0,
        ..sticky("a", Some("/today"))
    };

    store.put(moved.clone());

    assert_eq!(
        StickyStore::open(&file).stickies(),
        [moved, sticky("b", None)]
    );
}

// 変わっていない付箋紙を入れても、ファイルは書き直さない。シェルは、ウィンドウが動くたびに付箋紙を入れるためである。
#[test]
fn a_sticky_that_did_not_change_is_not_written_again() {
    let (_folder, file) = new_file();
    let mut store = StickyStore::open(&file);
    store.put(sticky("a", None));
    fs::write(&file, "written by someone else").unwrap();

    store.put(sticky("a", None));

    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        "written by someone else"
    );
}

// 消した付箋紙は、読み直しても出てこない。ほかの付箋紙は残る。
#[test]
fn a_sticky_that_is_removed_is_not_there_after_reading() {
    let (_folder, file) = new_file();
    let mut store = StickyStore::open(&file);
    store.put(sticky("a", None));
    store.put(sticky("b", None));

    store.remove("a");

    assert_eq!(StickyStore::open(&file).stickies(), [sticky("b", None)]);
}

// 読めないファイルは、付箋紙がないものとして扱う。次に入れた付箋紙は、保存される。
#[test]
fn a_file_that_cannot_be_read_counts_as_no_stickies() {
    let (_folder, file) = new_file();
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, "not json").unwrap();
    let mut store = StickyStore::open(&file);
    assert_eq!(store.stickies(), []);

    store.put(sticky("a", None));

    assert_eq!(StickyStore::open(&file).stickies(), [sticky("a", None)]);
}

// 色、常に手前、ズーム、巻き上げは、保存される。
#[test]
fn the_look_of_a_sticky_is_kept() {
    let (_folder, file) = new_file();
    let changed = StickySettings {
        color: "#FFD1DC".into(),
        always_on_top: true,
        zoom: 1.5,
        rolled_up: true,
        ..sticky("a", None)
    };
    StickyStore::open(&file).put(changed.clone());
    assert_eq!(StickyStore::open(&file).stickies(), [changed]);
}

// 古いファイルの付箋紙には、あとから足した設定がない。黄色で、手前に表示せず、等倍で、巻き上げていない。Windowsの古いファイルの、整数の位置も読める。
#[test]
fn a_sticky_of_an_older_file_has_the_look_of_a_new_one() {
    let (_folder, file) = new_file();
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(
        &file,
        r#"[{"id": "a", "x": 10, "y": 20, "width": 300, "height": 400}]"#,
    )
    .unwrap();
    assert_eq!(StickyStore::open(&file).stickies(), [sticky("a", None)]);
}

// 新しい付箋紙は黄色で、選べる色はPetastiの付箋紙の9色である。
#[test]
fn a_new_sticky_is_yellow_among_the_nine_colors_of_petasti() {
    assert_eq!(COLORS[0], ("Yellow", "#FFF8D0"));
    assert_eq!(COLORS.len(), 9);
}

// 透明は、Petastiの透明な付箋紙と同じく、アルファが0の白である。9色には入らない。
#[test]
fn transparent_is_white_with_no_alpha_and_not_among_the_nine_colors() {
    assert_eq!(TRANSPARENT, ("Transparent", "#FFFFFF00"));
    assert!(!COLORS.contains(&TRANSPARENT));
}

// ズームは、1割ずつ変わる。0.1を足し続けても、端数は出ない。
#[test]
fn the_zoom_changes_by_whole_tenths() {
    let mut zoom = 1.0;
    for _ in 0..7 {
        zoom = zoomed(zoom, 1);
    }
    assert_eq!(zoom, 1.7);
    assert_eq!(zoomed(1.7, -1), 1.6);
}

// ズームは、30%から300%の間に収まる。保存されていた値も同じである。
#[test]
fn the_zoom_stays_between_three_tenths_and_three() {
    assert_eq!(zoomed(0.3, -1), 0.3);
    assert_eq!(zoomed(3.0, 1), 3.0);
    assert_eq!(zoomed(9.0, 0), 3.0);
    assert_eq!(zoomed(0.0, 0), 0.3);
    assert_eq!(zoomed(1.25, 0), 1.3);
}

// `#RRGGBB`の色が読める。読めない色は、新しい付箋紙の黄色になる。
#[test]
fn a_color_is_read_from_its_hex_and_anything_else_is_yellow() {
    assert_eq!(rgb("#D1E1ff"), (0xD1, 0xE1, 0xFF));
    for unreadable in ["", "D1E1FF", "#D1E1F", "#D1E1FFF", "#GGGGGG", "#+1E1FF"] {
        assert_eq!(rgb(unreadable), (0xFF, 0xF8, 0xD0), "{unreadable:?}");
    }
}

// 付箋紙の一番小さいウィンドウは280×200、新しい付箋紙のウィンドウは400×520である。MacもWindowsも同じである。
#[test]
fn the_windows_of_stickies_have_the_sizes_of_the_windows_app() {
    assert_eq!((SMALLEST.width, SMALLEST.height), (280.0, 200.0));
    assert_eq!((USUAL.width, USUAL.height), (400.0, 520.0));
}
