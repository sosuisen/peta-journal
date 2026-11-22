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

/// What a test tells, with its kind inside.
#[derive(Facet)]
#[facet(tag = "t", rename_all = "snake_case")]
#[repr(C)]
#[allow(dead_code)]
enum Told {
    PointAt { id: String, cursor: u32 },
    Place(PagePlace),
}

/// A change of a test, with its kind outside.
#[derive(Facet)]
#[repr(C)]
#[allow(dead_code)]
enum Change {
    Insert { at: u32, text: String },
    Remove { at: u32 },
}

// データを持つenumは、種類ごとに関数を呼び分ける関数になる。引数は、フィールドの順である。タグを中に持つenumも、外に持つenumも同じである。
#[test]
fn an_enum_with_data_gives_a_function_that_calls_one_function_for_each_kind() {
    let told = lisp_of(&registry_of::<Told>());
    let change = lisp_of(&registry_of::<Change>());

    assert!(
        told.contains(concat!(
            "(defun peta-journal-protocol-handle-told (value handlers)\n",
            "  \"Call the function in HANDLERS for the kind of VALUE, with its fields.\n",
            "HANDLERS is a plist from the keyword of each kind to its function.\n",
            "The fields come in the order of `peta-journal-protocol-told-fields'.\"\n",
            "  (pcase (plist-get value :t)\n",
            "    (\"point_at\"\n",
            "     (funcall (plist-get handlers :point-at)\n",
            "              (plist-get value :id) (plist-get value :cursor)))\n",
            "    (\"place\"\n",
            "     (funcall (plist-get handlers :place)\n",
            "              (plist-get value :page_id) (plist-get value :at)))))\n",
        )),
        "{told}"
    );
    assert!(
        told.contains(
            "(defconst peta-journal-protocol-told-fields\n  '((:point-at \"id\" \"cursor\") (:place \"page_id\" \"at\"))\n"
        ),
        "{told}"
    );
    assert!(
        change.contains(concat!(
            "  (pcase value\n",
            "    (`(:Insert ,fields)\n",
            "     (funcall (plist-get handlers :insert)\n",
            "              (plist-get fields :at) (plist-get fields :text)))\n",
            "    (`(:Remove ,fields)\n",
            "     (funcall (plist-get handlers :remove)\n",
            "              (plist-get fields :at)))))\n",
        )),
        "{change}"
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
