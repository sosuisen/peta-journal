//! Opening an app or a folder through the OS.

use crate::app::Event;
use crate::effects::UiOp;
use crate::harness::Harness;

// 拡張の URL は OS に渡されて開かれる。開けなくても、書き出しフォルダを開けなくても、画面には何も出ない。
#[test]
fn a_url_and_the_export_folder_are_opened_by_the_os() {
    let mut h = Harness::new();
    h.send(Event::LaunchUrl {
        url: "peta://x".into(),
    });
    h.api
        .answer_next("LaunchUrl", Ok(crate::effects::ApiOutput::Launched(false)));
    h.send(Event::LaunchUrl {
        url: "peta://y".into(),
    });
    h.send(Event::OpenExportFolder {
        page_id: "page1".into(),
    });
    h.api.fail_next("OpenExportFolder", "no folder");
    h.send(Event::OpenExportFolder {
        page_id: "page1".into(),
    });

    assert_eq!(h.api.calls_named("LaunchUrl").len(), 2);
    assert_eq!(h.api.calls_named("OpenExportFolder").len(), 2);
    assert!(!h.ui.iter().any(|u| matches!(u, UiOp::Alert(_))));
}
