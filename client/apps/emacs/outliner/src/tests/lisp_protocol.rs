use crate::lisp_protocol::lisp_of;
use facet::Facet;
use facet_generate::reflection::RegistryBuilder;
use facet_generate::Registry;

fn registry_of<'a, T: Facet<'a>>() -> Registry {
    RegistryBuilder::new()
        .add_type::<T>()
        .unwrap()
        .build()
        .unwrap()
}

/// A line of a test, with a field of each kind.
#[derive(Facet)]
struct PagePlace {
    /// The page.
    page_id: String,
    at: u32,
}

// 構造体は、フィールドごとに、Lispの名前で読む関数になる。
#[test]
fn a_struct_gives_a_reader_for_each_field_by_its_lisp_name() {
    let lisp = lisp_of(&registry_of::<PagePlace>());

    assert!(
        lisp.contains(
            "(defsubst peta-journal-protocol-page-place-page-id (value)\n  \"The page.\"\n  (plist-get value :page_id))\n"
        ),
        "{lisp}"
    );
    assert!(
        lisp.contains("(defsubst peta-journal-protocol-page-place-at (value)\n"),
        "{lisp}"
    );
}
