//! Opening a folder through the OS.

use crate::app::Event;
use crate::effects::UiOp;
use crate::harness::Harness;

// 書き出しフォルダは OS に開かれる。開けなくても、画面には何も出ない。
#[test]
fn the_export_folder_is_opened_by_the_os() {
    let mut h = Harness::new();
    h.send(Event::OpenExportFolder {
        page_id: "page1".into(),
    });
    h.api.fail_next("OpenExportFolder", "no folder");
    h.send(Event::OpenExportFolder {
        page_id: "page1".into(),
    });

    assert_eq!(h.api.calls_named("OpenExportFolder").len(), 2);
    assert!(!h.ui.iter().any(|u| matches!(u, UiOp::Alert(_))));
}
