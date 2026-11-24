use crate::app::SelectionKey;
use crate::keys::{
    Chord, EditorKey, EditorState, Held, Key, KeyMap, KeyPlatform, KeyPress, OpenIn,
};

const NOW: u64 = 1_000;

/// An editor of one line with the cursor at `cursor`, in UTF-16 units.
fn cursor_at(text: &str, cursor: u64) -> EditorState {
    EditorState {
        text: text.into(),
        selection_start: cursor,
        selection_end: cursor,
        on_first_line: true,
        on_last_line: true,
    }
}

/// An editor of one line with `start..end` selected.
fn selecting(text: &str, start: u64, end: u64) -> EditorState {
    EditorState {
        selection_end: end,
        ..cursor_at(text, start)
    }
}

/// An editor whose cursor is on neither the first nor the last line.
fn in_the_middle(text: &str, cursor: u64) -> EditorState {
    EditorState {
        on_first_line: false,
        on_last_line: false,
        ..cursor_at(text, cursor)
    }
}

fn chord(key: Key, held: &str) -> Chord {
    Chord {
        key,
        shift: held.contains("shift"),
        ctrl: held.contains("ctrl"),
        alt: held.contains("alt"),
        cmd: held.contains("cmd"),
    }
}

fn letter(letter: &str) -> Key {
    Key::Letter(letter.into())
}

/// The event of a key of the keyboard in the editor of item `a`, as its debug text.
fn pressed(platform: KeyPlatform, key: Key, held: &str, editor: EditorState) -> String {
    let press = KeyPress::InEditor {
        item_id: "a".into(),
        chord: chord(key, held),
        editor,
    };
    format!("{:?}", KeyMap::of(platform).event_of(&press, NOW))
}

fn on_windows(key: Key, held: &str, editor: EditorState) -> String {
    pressed(KeyPlatform::Windows, key, held, editor)
}

fn in_selection(platform: KeyPlatform, key: Key, held: &str) -> Option<SelectionKey> {
    KeyMap::of(platform).in_selection(&chord(key, held))
}

// Enterは、カーソルの位置で項目を分ける。位置は、シェルが数えたUTF-16から文字の数に直される。
#[test]
fn enter_splits_the_item_at_the_cursor_counted_in_characters() {
    // 😀 is two UTF-16 units and one character
    let event = on_windows(Key::Enter, "", cursor_at("😀ab", 3));
    assert_eq!(
        event,
        r#"Some(EnterPressed { item_id: "a", cursor: 2, now_ms: 1000 })"#
    );
}

// Shift+Enterは、項目の中で改行するので、アウトラインの操作にならない。
#[test]
fn shift_enter_is_left_to_the_text() {
    assert_eq!(on_windows(Key::Enter, "shift", cursor_at("ab", 1)), "None");
}

// ↑は、最初の行でだけ上の項目へ移る。ほかの行では、文字の中でカーソルが動く。
#[test]
fn up_goes_to_the_item_above_only_from_the_first_line() {
    assert_eq!(
        on_windows(Key::Up, "", cursor_at("ab", 1)),
        r#"Some(NavigateUp { item_id: "a", cursor: 1 })"#
    );
    assert_eq!(on_windows(Key::Up, "", in_the_middle("ab", 1)), "None");
}

// ↓は、最後の行でだけ下の項目へ移る。
#[test]
fn down_goes_to_the_item_below_only_from_the_last_line() {
    assert_eq!(
        on_windows(Key::Down, "", cursor_at("ab", 1)),
        r#"Some(NavigateDown { item_id: "a", cursor: 1 })"#
    );
    assert_eq!(on_windows(Key::Down, "", in_the_middle("ab", 1)), "None");
}

// ←は、先頭でだけ上の項目の末尾へ移る。文字を選んでいるときは、まず選択が解ける。
#[test]
fn left_goes_to_the_end_of_the_item_above_only_from_the_start_with_nothing_selected() {
    assert_eq!(
        on_windows(Key::Left, "", cursor_at("ab", 0)),
        format!(
            r#"Some(NavigateUp {{ item_id: "a", cursor: {} }})"#,
            usize::MAX / 2
        )
    );
    assert_eq!(on_windows(Key::Left, "", cursor_at("ab", 1)), "None");
    assert_eq!(on_windows(Key::Left, "", selecting("ab", 0, 2)), "None");
}

// →は、末尾でだけ下の項目の先頭へ移る。文字を選んでいるときは、まず選択が解ける。
#[test]
fn right_goes_to_the_start_of_the_item_below_only_from_the_end_with_nothing_selected() {
    assert_eq!(
        on_windows(Key::Right, "", cursor_at("😀", 2)),
        r#"Some(NavigateDown { item_id: "a", cursor: 0 })"#
    );
    assert_eq!(on_windows(Key::Right, "", cursor_at("😀", 0)), "None");
    assert_eq!(on_windows(Key::Right, "", selecting("ab", 0, 2)), "None");
}

// Backspaceは、先頭でだけ前の項目とつなぐ。文字を選んでいるときは、選んだ文字を消す。
#[test]
fn backspace_joins_the_item_above_only_at_the_start_with_nothing_selected() {
    assert_eq!(
        on_windows(Key::Backspace, "", cursor_at("ab", 0)),
        r#"Some(BackspaceAtStart { item_id: "a" })"#
    );
    assert_eq!(on_windows(Key::Backspace, "", cursor_at("ab", 1)), "None");
    assert_eq!(
        on_windows(Key::Backspace, "", selecting("ab", 0, 1)),
        "None"
    );
}

// Deleteは、末尾でだけ次の項目をつなぐ。
#[test]
fn delete_joins_the_item_below_only_at_the_end_with_nothing_selected() {
    assert_eq!(
        on_windows(Key::Delete, "", cursor_at("ab", 2)),
        r#"Some(DeleteAtEnd { item_id: "a" })"#
    );
    assert_eq!(on_windows(Key::Delete, "", cursor_at("ab", 1)), "None");
    assert_eq!(on_windows(Key::Delete, "", selecting("ab", 1, 2)), "None");
}

// Shift+↑は最初の行で、Shift+↓は最後の行で、項目の選択を始める。ほかの行では文字を選ぶ。
#[test]
fn shift_up_and_down_select_the_item_from_the_first_and_last_lines() {
    let select = r#"Some(ShiftSelect { item_id: "a" })"#;
    assert_eq!(on_windows(Key::Up, "shift", cursor_at("ab", 1)), select);
    assert_eq!(on_windows(Key::Down, "shift", cursor_at("ab", 1)), select);
    assert_eq!(on_windows(Key::Up, "shift", in_the_middle("ab", 1)), "None");
    assert_eq!(
        on_windows(Key::Down, "shift", in_the_middle("ab", 1)),
        "None"
    );
}

// TabとShift+Alt+→は字下げし、Shift+TabとShift+Alt+←は字下げを戻す。Shift+Alt+↑↓は項目を動かす。
#[test]
fn tab_and_shift_alt_arrows_indent_outdent_and_move_the_item() {
    let editor = || in_the_middle("ab", 1);
    let indent = r#"Some(Indent { item_id: "a" })"#;
    let outdent = r#"Some(Outdent { item_id: "a" })"#;
    assert_eq!(on_windows(Key::Tab, "", editor()), indent);
    assert_eq!(on_windows(Key::Right, "shift alt", editor()), indent);
    assert_eq!(on_windows(Key::Tab, "shift", editor()), outdent);
    assert_eq!(on_windows(Key::Left, "shift alt", editor()), outdent);
    assert_eq!(
        on_windows(Key::Up, "shift alt", editor()),
        r#"Some(MoveUp { item_id: "a" })"#
    );
    assert_eq!(
        on_windows(Key::Down, "shift alt", editor()),
        r#"Some(MoveDown { item_id: "a" })"#
    );
}

// Windowsでは、Ctrl+Enterがタスクを進め、Ctrl+↑↓が子を隠して戻し、Ctrl+Zが取り消し、Ctrl+Oがリンクを開く。
#[test]
fn control_keys_on_windows_step_the_task_fold_undo_and_open_the_link() {
    let editor = || in_the_middle("[[p]]", 2);
    assert_eq!(
        on_windows(Key::Enter, "ctrl", editor()),
        r#"Some(TaskCycle { item_id: "a" })"#
    );
    assert_eq!(
        on_windows(Key::Up, "ctrl", editor()),
        r#"Some(SetCollapsed { item_id: "a", collapsed: true })"#
    );
    assert_eq!(
        on_windows(Key::Down, "ctrl", editor()),
        r#"Some(SetCollapsed { item_id: "a", collapsed: false })"#
    );
    assert_eq!(on_windows(letter("z"), "ctrl", editor()), "Some(Undo)");
    assert_eq!(
        on_windows(letter("z"), "ctrl shift", editor()),
        "Some(Redo)"
    );
    assert_eq!(
        on_windows(letter("o"), "ctrl", editor()),
        r#"Some(OpenLinkAt { item_id: "a", cursor: 2 })"#
    );
}

// Macでは、Cmdが子を隠し、タスクを進め、リンクを開く。Ctrl+↑↓とCtrl+Oは、macOSに任せる。
#[test]
fn command_keys_on_a_mac_fold_step_the_task_and_open_the_link() {
    let on_mac = |key, held| pressed(KeyPlatform::Mac, key, held, in_the_middle("[[p]]", 2));
    assert_eq!(
        on_mac(Key::Up, "cmd"),
        r#"Some(SetCollapsed { item_id: "a", collapsed: true })"#
    );
    assert_eq!(
        on_mac(Key::Enter, "cmd"),
        r#"Some(TaskCycle { item_id: "a" })"#
    );
    assert_eq!(
        on_mac(Key::Enter, "ctrl"),
        r#"Some(TaskCycle { item_id: "a" })"#
    );
    assert_eq!(
        on_mac(letter("o"), "cmd"),
        r#"Some(OpenLinkAt { item_id: "a", cursor: 2 })"#
    );
    assert_eq!(on_mac(Key::Up, "ctrl"), "None");
    assert_eq!(on_mac(letter("o"), "ctrl"), "None");
}

// Macでは、Cmd+ZとCtrl+Zのどちらでも取り消せる。
#[test]
fn a_mac_undoes_with_command_or_control() {
    let on_mac = |key, held| pressed(KeyPlatform::Mac, key, held, cursor_at("", 0));
    assert_eq!(on_mac(letter("z"), "cmd"), "Some(Undo)");
    assert_eq!(on_mac(letter("z"), "cmd shift"), "Some(Redo)");
    assert_eq!(on_mac(letter("z"), "ctrl"), "Some(Undo)");
    assert_eq!(on_mac(letter("z"), "ctrl shift"), "Some(Redo)");
}

// Webでは、Windowsのキーに加えて、Macのブラウザのために、Cmd+↑↓でも子を隠して戻す。
#[test]
fn the_web_also_folds_with_command_for_a_mac_browser() {
    let on_web = |key, held| pressed(KeyPlatform::Web, key, held, in_the_middle("", 0));
    let fold = r#"Some(SetCollapsed { item_id: "a", collapsed: true })"#;
    assert_eq!(on_web(Key::Up, "ctrl"), fold);
    assert_eq!(on_web(Key::Up, "cmd"), fold);
    assert_eq!(
        on_web(letter("o"), "ctrl"),
        r#"Some(OpenLinkAt { item_id: "a", cursor: 0 })"#
    );
}

// 表にない組み合わせは、文字の操作として編集欄に任される。
#[test]
fn a_chord_not_in_the_map_is_left_to_the_text() {
    assert_eq!(on_windows(letter("a"), "ctrl", cursor_at("", 0)), "None");
    assert_eq!(on_windows(Key::Up, "alt", cursor_at("", 0)), "None");
}

// ツールバーのボタンのように、シェルが自分で読んだキーも、同じ決まりで扱われる。
#[test]
fn a_key_the_shell_read_itself_follows_the_same_rules() {
    let press = |editor| KeyPress::Named {
        item_id: "a".into(),
        key: EditorKey::Up,
        editor,
    };
    let map = KeyMap::of(KeyPlatform::Windows);
    let event = |editor| format!("{:?}", map.event_of(&press(editor), NOW));
    assert_eq!(
        event(cursor_at("ab", 1)),
        r#"Some(NavigateUp { item_id: "a", cursor: 1 })"#
    );
    assert_eq!(event(in_the_middle("ab", 1)), "None");
}

// 項目を選んでいる間は、Escapeで選択を解き、BackspaceとDeleteで消し、Tabで字下げし、Shift+↑↓で範囲を広げる。
#[test]
fn keys_while_items_are_selected() {
    use SelectionKey as S;
    let on_windows = |key, held| in_selection(KeyPlatform::Windows, key, held);
    assert_eq!(on_windows(Key::Escape, ""), Some(S::Escape));
    assert_eq!(on_windows(Key::Backspace, ""), Some(S::Delete));
    assert_eq!(on_windows(Key::Delete, ""), Some(S::Delete));
    assert_eq!(on_windows(Key::Tab, ""), Some(S::Indent));
    assert_eq!(on_windows(Key::Tab, "shift"), Some(S::Outdent));
    assert_eq!(on_windows(Key::Right, "shift alt"), Some(S::Indent));
    assert_eq!(on_windows(Key::Left, "shift alt"), Some(S::Outdent));
    assert_eq!(on_windows(Key::Up, "shift alt"), Some(S::MoveUp));
    assert_eq!(on_windows(Key::Down, "shift alt"), Some(S::MoveDown));
    assert_eq!(on_windows(Key::Up, "shift"), Some(S::ExtendUp));
    assert_eq!(on_windows(Key::Down, "shift"), Some(S::ExtendDown));
    assert_eq!(on_windows(Key::Enter, ""), None);
}

// 選んだ項目の写しと切り取りは、WindowsではCtrl、MacではCmd、Webではどちらでもよい。
#[test]
fn copy_and_cut_of_selected_items_follow_the_platform() {
    use KeyPlatform::{Mac, Web, Windows};
    use SelectionKey as S;
    assert_eq!(in_selection(Windows, letter("c"), "ctrl"), Some(S::Copy));
    assert_eq!(in_selection(Windows, letter("x"), "ctrl"), Some(S::Cut));
    assert_eq!(in_selection(Windows, letter("z"), "ctrl"), Some(S::Undo));
    assert_eq!(in_selection(Mac, letter("c"), "cmd"), Some(S::Copy));
    assert_eq!(in_selection(Mac, letter("x"), "cmd"), Some(S::Cut));
    assert_eq!(in_selection(Mac, letter("c"), "ctrl"), None);
    assert_eq!(in_selection(Mac, letter("z"), "cmd shift"), Some(S::Redo));
    assert_eq!(in_selection(Mac, letter("z"), "ctrl"), Some(S::Undo));
    assert_eq!(in_selection(Web, letter("c"), "ctrl"), Some(S::Copy));
    assert_eq!(in_selection(Web, letter("c"), "cmd"), Some(S::Copy));
}

// 選択中のキーは、SelectionKeyのイベントになる。
#[test]
fn a_key_in_a_selection_sends_the_selection_key() {
    let press = KeyPress::InSelection {
        chord: chord(Key::Escape, ""),
    };
    let event = KeyMap::of(KeyPlatform::Windows).event_of(&press, NOW);
    assert_eq!(format!("{event:?}"), "Some(SelectionKey(Escape))");
}

// ページは、Shiftでブラウザに、CtrlかCommandでアプリの新しい画面に、何もなければその場で開く。両方を押すとブラウザになる（ADR 083）。
#[test]
fn shift_opens_the_browser_and_ctrl_or_command_a_new_view() {
    let open_in = |shift, ctrl, cmd| Held { shift, ctrl, cmd }.open_in();
    assert_eq!(open_in(false, false, false), OpenIn::Here);
    assert_eq!(open_in(false, true, false), OpenIn::NewView);
    assert_eq!(open_in(false, false, true), OpenIn::NewView);
    assert_eq!(open_in(true, false, false), OpenIn::Browser);
    assert_eq!(open_in(true, true, false), OpenIn::Browser);
}
