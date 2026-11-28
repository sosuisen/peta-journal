//! The JSON that crosses between wasm and the shell. It is the JSON of facet, whose names are
//! those of the Rust fields, as the generated TypeScript has them (ADR 080). The names that
//! serde gives for REST do not cross here.

use crate::body::HtmlBody;
use client_core::changes::Changes;
use facet::Facet;

/// Reads a value the shell wrote. The error says why the JSON is not one.
pub fn read<T: Facet<'static>>(json: &str) -> Result<T, String> {
    facet_json::from_str(json).map_err(|error| error.to_string())
}

/// Writes the changes of the screen for the shell.
pub fn write(changes: &Changes<HtmlBody>) -> String {
    facet_json::to_string(changes).expect("the changes are always valid JSON")
}

#[cfg(test)]
mod tests {
    use super::*;
    use client_core::drag::DragMove;
    use client_core::effects::UiOutput;
    use client_core::keys::{Chord, EditorState, Key, KeyPress};
    use client_core::typing::Typing;
    use client_core::Event;

    // シェルが書いたイベント、キー、入力、ドラッグ、答えは、facetのJSONで読める。
    #[test]
    fn what_the_shell_writes_is_read_as_the_json_of_facet() {
        let event: Event = read(r#"{"EditFinished":{"item_id":"i1"}}"#).unwrap();
        assert!(matches!(event, Event::EditFinished { item_id } if item_id == "i1"));

        let key: KeyPress = read(
            r#"{"InEditor":{"item_id":"i1",
                "chord":{"key":{"Letter":"a"},"shift":false,"ctrl":true,"alt":false,"cmd":false},
                "editor":{"text":"ab","selection_start":1,"selection_end":1,
                          "on_first_line":true,"on_last_line":false}}}"#,
        )
        .unwrap();
        assert_eq!(
            key,
            KeyPress::InEditor {
                item_id: "i1".into(),
                chord: Chord {
                    key: Key::Letter("a".into()),
                    shift: false,
                    ctrl: true,
                    alt: false,
                    cmd: false,
                },
                editor: EditorState {
                    text: "ab".into(),
                    selection_start: 1,
                    selection_end: 1,
                    on_first_line: true,
                    on_last_line: false,
                },
            }
        );

        let typing: Typing =
            read(r#"{"Typed":{"item_id":"i1","body":"日本語","cursor":3}}"#).unwrap();
        assert_eq!(
            typing,
            Typing::Typed {
                item_id: "i1".into(),
                body: "日本語".into(),
                cursor: 3,
            }
        );

        let moved: DragMove = read(r#"{"Moved":{"y":10.5,"item_under":null}}"#).unwrap();
        assert_eq!(
            moved,
            DragMove::Moved {
                y: 10.5,
                item_under: None,
            }
        );
        assert_eq!(
            read::<DragMove>(r#""Released""#).unwrap(),
            DragMove::Released
        );

        assert_eq!(
            read::<UiOutput>(r#"{"Bool":true}"#).unwrap(),
            UiOutput::Bool(true)
        );
        assert_eq!(read::<UiOutput>(r#""Done""#).unwrap(), UiOutput::Done);
        assert_eq!(
            read::<UiOutput>(r#"{"Text":null}"#).unwrap(),
            UiOutput::Text(None)
        );
    }

    // 画面の差分は、facetのJSONで書かれ、項目の名前はRustのフィールド名である。
    #[test]
    fn the_changes_are_written_as_the_json_of_facet_with_the_names_of_the_rust_fields() {
        use client_core::changes::Shown;
        use client_core::testing::plain_item;
        use client_core::{Model, PetaApp};
        use crux_core::App;
        let mut model = Model::default();
        model.state.set_items(vec![plain_item("i1", 1, "text")]);
        let view = PetaApp::<HtmlBody>::default().view(&model);

        let json = write(&Shown::default().changes(view));

        assert!(
            json.starts_with(r#"{"rows":[{"Reset":{"items":[{"item":{"id":"i1","#),
            "{json}"
        );
        assert!(json.contains(r#""page_id":"page1""#), "{json}");
        assert!(json.contains(r#""item_type":"doc""#), "{json}");
        assert!(!json.contains("pageId"), "{json}");
    }

    // 小数のない数で書かれた小数も読める。JSON.rawJSONのないブラウザは、0.0を0と書くためである。
    #[test]
    fn a_float_written_without_a_fraction_is_read() {
        let pressed: DragMove =
            read(r#"{"Pressed":{"item_id":"i1","top":0,"bottom":20}}"#).unwrap();

        assert_eq!(
            pressed,
            DragMove::Pressed {
                item_id: "i1".into(),
                top: 0.0,
                bottom: 20.0,
            }
        );
    }

    // facetのJSONでないものは、読めない理由とともに断られる。
    #[test]
    fn what_is_not_the_json_of_facet_is_refused() {
        // The name REST gives, not that of the Rust field
        let refused = read::<Event>(r#"{"EditFinished":{"itemId":"i1"}}"#);

        assert!(refused.is_err());
    }

    // 1つの値だけを持つ型は、持つ値のままで境界を渡る。生成されたTypeScriptの型が、そう書くためである。
    #[test]
    fn a_type_of_one_value_crosses_as_that_value() {
        let text = r#"{"Start":{"client_id":"c1","opens_new_day":true}}"#;

        let event: Event = read(text).unwrap();

        assert_eq!(facet_json::to_string(&event).unwrap(), text);
    }
}
