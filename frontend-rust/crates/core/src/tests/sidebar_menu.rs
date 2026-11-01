//! The context menu of a page in the sidebar.

use crate::app::Event;
use crate::effects::ApiOp;
use crate::harness::Harness;

#[test]
fn opening_the_export_folder_asks_the_server_for_the_page() {
    let mut h = Harness::new();
    h.send(Event::OpenExportFolder {
        page_id: "page1".into(),
    });
    assert_eq!(
        h.api.calls_named("OpenExportFolder"),
        vec![&ApiOp::OpenExportFolder {
            page_id: "page1".into()
        }]
    );
}

#[test]
fn a_failure_to_open_the_folder_is_only_logged() {
    let mut h = Harness::new();
    h.api
        .fail_next("OpenExportFolder", "Export folder not found");
    h.send(Event::OpenExportFolder {
        page_id: "page1".into(),
    });
    assert!(h.api.called("OpenExportFolder"));
}
