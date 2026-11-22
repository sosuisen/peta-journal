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

/// The look of a task in a test.
#[derive(Facet)]
#[facet(rename_all = "snake_case")]
#[repr(C)]
#[allow(dead_code)]
enum TaskLook {
    NotDone,
    Done,
}

// 値だけのenumは、送るときの名前の一覧になる。
#[test]
fn an_enum_of_names_gives_the_list_of_its_names_as_they_are_sent() {
    let lisp = lisp_of(&registry_of::<TaskLook>());

    assert!(
        lisp.contains(
            "(defconst peta-journal-protocol-task-look-names '(\"not_done\" \"done\")\n  \"The look of a task in a test.\")\n"
        ),
        "{lisp}"
    );
}
