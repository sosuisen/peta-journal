//! Who does each operation of the screen in the web app.

#[cfg(test)]
mod tests {
    use super::*;
    use client_core::effects::UiOp;

    // 時計とwindowのAPIで済む画面の操作は、wasmがこなす。
    #[test]
    fn wasm_does_the_operations_that_the_clock_and_window_do() {
        let ops = [
            UiOp::LocalNow,
            UiOp::SetTitle("notes".into()),
            UiOp::SaveAtlasDirs(vec!["maps/".into()]),
            UiOp::CopyFailed,
            UiOp::ReadClipboardText,
            UiOp::Alert("done".into()),
            UiOp::Confirm("sure?".into()),
            UiOp::Prompt("name?".into()),
            UiOp::OpenWindow {
                url: "/pages/notes".into(),
                new_window: true,
            },
        ];

        for op in ops {
            assert_eq!(lane_of(&op), Lane::Wasm, "{op:?}");
        }
    }

    // 破壊的な確認は、質問としてシェルへ渡す。
    #[test]
    fn a_destructive_confirm_goes_to_the_shell_as_a_question() {
        let op = UiOp::ConfirmDestructive("delete?".into());

        assert_eq!(lane_of(&op), Lane::Question);
    }

    // 編集欄へのフォーカスと項目までのスクロールは、シェルへ渡す。
    #[test]
    fn the_focus_of_an_editor_and_a_scroll_to_an_item_go_to_the_shell() {
        let ops = [
            UiOp::FocusEditor {
                item_id: Some("i1".into()),
                replay_keys: vec![],
            },
            UiOp::ScrollToItem {
                item_id: "i1".into(),
                smooth: true,
            },
        ];

        for op in ops {
            assert_eq!(lane_of(&op), Lane::Shell, "{op:?}");
        }
    }
}
