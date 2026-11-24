//! Who does each operation of the screen in the web app.
//!
//! The native bridge has a table of its own, as who does what differs there: a native shell
//! shows every dialog itself (ADR 080).

use client_core::effects::UiOp;

/// Who does an operation of the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lane {
    /// Wasm, with the clock and the APIs of `window`.
    Wasm,
    /// The shell, one question at a time (`client_core::policy::Questions`).
    Question,
    /// The shell, which has the elements on the screen.
    Shell,
}

/// Who does `op`. The dialogs of the browser (`alert`, `confirm`, `prompt`) are of `window`,
/// so wasm shows them. The confirm of a destructive change is a dialog of the shell instead:
/// the dialog of the browser answers OK to Enter.
pub fn lane_of(op: &UiOp) -> Lane {
    match op {
        UiOp::FocusEditor { .. } | UiOp::ScrollToItem { .. } => Lane::Shell,
        UiOp::ConfirmDestructive(_) => Lane::Question,
        UiOp::LocalNow
        | UiOp::SetTitle(_)
        | UiOp::SaveAtlasDirs(_)
        | UiOp::CopyMarkdown { .. }
        | UiOp::CopyFailed
        | UiOp::ReadClipboardText
        | UiOp::Alert(_)
        | UiOp::Confirm(_)
        | UiOp::Prompt(_)
        | UiOp::OpenNewView { .. }
        | UiOp::OpenInBrowser { .. } => Lane::Wasm,
    }
}

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
            UiOp::OpenNewView {
                url: "/pages/notes".into(),
            },
            UiOp::OpenInBrowser {
                address: "/assets/a.png".into(),
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
