//! The atlas page of a directory is a plain page (ADR 053): the server writes nothing into it
//! by itself, whatever happens to the pages of the directory.

use crate::common::Core;

impl Core {
    /// A directory with its atlas page. Gives the directory and the id of the atlas.
    fn dir_with_atlas(&self, name: &str) -> (String, String) {
        self.new_doc_page(name);
        let dir = format!("{name}/");
        let atlas_id = self.atlas_of(&dir);
        (dir, atlas_id)
    }
}

// ページを作っても、アトラスにカードは足されない。
#[test]
fn a_new_page_gets_no_card_on_the_atlas() {
    let notes = Core::new();
    let (dir, atlas_id) = notes.dir_with_atlas("plain-new");
    let before = notes.outline_of(&atlas_id);

    notes.open_doc_in("b", &dir);

    assert_eq!(notes.outline_of(&atlas_id), before);
}
