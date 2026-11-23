//! The JSON that crosses between wasm and the shell. It is the JSON of facet, whose names are
//! those of the Rust fields, as the generated TypeScript has them (ADR 080). The names that
//! serde gives for REST do not cross here.

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

    // facetのJSONでないものは、読めない理由とともに断られる。
    #[test]
    fn what_is_not_the_json_of_facet_is_refused() {
        // The name REST gives, not that of the Rust field
        let refused = read::<Event>(r#"{"EditFinished":{"itemId":"i1"}}"#);

        assert!(refused.is_err());
    }
}
