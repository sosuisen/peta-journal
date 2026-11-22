//! The changes of the screen, for a shell of any body model (ADR 078).

use crate::changes::{ListChange, Panel, Shown};
use crate::harness::Harness;
use crate::PetaApp;
use crux_core::App;

// 差分は、どの本文の形の画面からも作れる。本文の形を持たない画面でも、行の入れ替えと、変わったパネルが作られる。
#[test]
fn the_changes_are_made_from_a_screen_of_any_body_model() {
    let h = Harness::new();
    let app = PetaApp::<()>::default();
    let mut shown = Shown::default();
    let first = shown.changes(app.view(&h.model));
    assert!(matches!(first.rows[..], [ListChange::Reset { .. }]));

    let mut view = app.view(&h.model);
    view.title = "notes".into();
    let changes = shown.changes(view);

    assert!(changes.rows.is_empty());
    assert_eq!(changes.panels, vec![Panel::Title("notes".into())]);
}
