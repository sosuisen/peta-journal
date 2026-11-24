//! What the keys of the keyboard mean to the outline, for every shell that reads keys (ADR 052).
//! A shell turns a key of its toolkit into a [`Chord`], and the [`KeyMap`] of its platform tells
//! which [`EditorKey`] or [`SelectionKey`] that is. [`event_of`] then decides, from the editor
//! of the item, which [`Event`] the key sends. The shell only reads its keys and measures the
//! lines of its editor.
//!
//! The map is data, so that a setting may change it one day without a change of the shells.

use crate::app::{Event, SelectionKey};
use crate::util::utf16::chars_of_utf16;

/// A key of the keyboard that the outline knows, whatever toolkit read it.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum Key {
    Enter,
    Tab,
    Up,
    Down,
    Left,
    Right,
    Backspace,
    /// The key that deletes forward.
    Delete,
    Escape,
    /// A key of a letter, in lower case, such as `"z"`.
    Letter(String),
}

/// The keys held down while a link to a page is clicked. `cmd` is the Command key of a Mac,
/// and the Meta key of a browser.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize, serde::Serialize)]
pub struct Held {
    pub shift: bool,
    pub ctrl: bool,
    pub cmd: bool,
}

/// Where a clicked page opens (ADR 083).
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize, serde::Serialize)]
pub enum OpenIn {
    /// In place of the shown page.
    #[default]
    Here,
    /// In a new view of the app: a new tab of the web frontend, or a new sticky.
    NewView,
    /// The page of the web frontend, in the browser.
    Browser,
}

impl Held {
    /// Shift opens the browser, Ctrl or Command a new view. Shift wins when both are held.
    pub fn open_in(self) -> OpenIn {
        if self.shift {
            OpenIn::Browser
        } else if self.ctrl || self.cmd {
            OpenIn::NewView
        } else {
            OpenIn::Here
        }
    }
}

/// A key with the keys held down with it. `cmd` is the Command key of a Mac, and the Meta key
/// of a browser.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Chord {
    pub key: Key,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub cmd: bool,
}

/// A key that means something to the outline in the editor of an item, and not only to its
/// text.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum EditorKey {
    Enter,
    Tab,
    BackTab,
    Up,
    Down,
    Left,
    Right,
    Backspace,
    Delete,
    /// Makes the item a task, or steps the state of the task.
    TaskCycle,
    /// Hides the children of the item.
    Fold,
    /// Shows the children of the item again.
    Unfold,
    MoveUp,
    MoveDown,
    MoveIn,
    MoveOut,
    Undo,
    Redo,
    /// Shift with Up. On the first line it selects the item; on any other line it selects
    /// text, as in any editor.
    SelectUp,
    /// Shift with Down, which selects the item on the last line.
    SelectDown,
    /// Opens the page that the link at the cursor names.
    OpenLink,
}

/// The editor of an item as a key finds it. The places count UTF-16 units, as the editors of
/// the shells do.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct EditorState {
    /// The Markdown in the editor, with `\n` between its lines.
    pub text: String,
    /// Where the selection starts. The cursor, when nothing is selected.
    pub selection_start: u64,
    /// Where the selection ends: the same as the start when nothing is selected.
    pub selection_end: u64,
    /// The cursor is on the first line of the text as it is drawn.
    pub on_first_line: bool,
    /// The cursor is on the last line of the text as it is drawn.
    pub on_last_line: bool,
}

/// The default key maps.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum KeyPlatform {
    Windows,
    Mac,
    Web,
}

/// A key that a native shell hands to the core.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum KeyPress {
    /// A key of the keyboard in the editor of an item.
    InEditor {
        item_id: String,
        chord: Chord,
        editor: EditorState,
    },
    /// A key that the shell has read itself, as from a button of its toolbar or from what the
    /// system made of a key (`moveUp:` on a Mac).
    Named {
        item_id: String,
        key: EditorKey,
        editor: EditorState,
    },
    /// A key of the keyboard while items are selected. No editor has the cursor then.
    InSelection { chord: Chord },
}

/// What the chords of the keyboard mean: in an editor, and while items are selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyMap {
    editor: Vec<(Chord, EditorKey)>,
    selection: Vec<(Chord, SelectionKey)>,
}

impl KeyMap {
    /// The map of a platform.
    pub fn of(platform: KeyPlatform) -> KeyMap {
        match platform {
            KeyPlatform::Windows => KeyMap::windows(),
            KeyPlatform::Mac => KeyMap::mac(),
            KeyPlatform::Web => KeyMap::web(),
        }
    }

    /// The keys of the stickies for Windows: those of the web frontend.
    pub fn windows() -> KeyMap {
        use EditorKey as E;
        use SelectionKey as S;
        KeyMap {
            editor: [
                (plain(Key::Enter), E::Enter),
                (ctrl(Key::Enter), E::TaskCycle),
                (plain(Key::Tab), E::Tab),
                (shift(Key::Tab), E::BackTab),
                (plain(Key::Up), E::Up),
                (plain(Key::Down), E::Down),
                (plain(Key::Left), E::Left),
                (plain(Key::Right), E::Right),
                (plain(Key::Backspace), E::Backspace),
                (plain(Key::Delete), E::Delete),
                (shift(Key::Up), E::SelectUp),
                (shift(Key::Down), E::SelectDown),
                (ctrl(Key::Up), E::Fold),
                (ctrl(Key::Down), E::Unfold),
                (shift_alt(Key::Up), E::MoveUp),
                (shift_alt(Key::Down), E::MoveDown),
                (shift_alt(Key::Left), E::MoveOut),
                (shift_alt(Key::Right), E::MoveIn),
                (ctrl(letter("z")), E::Undo),
                (ctrl_shift(letter("z")), E::Redo),
                (ctrl(letter("o")), E::OpenLink),
            ]
            .into(),
            selection: [
                (plain(Key::Escape), S::Escape),
                (plain(Key::Backspace), S::Delete),
                (plain(Key::Delete), S::Delete),
                (shift(Key::Up), S::ExtendUp),
                (shift(Key::Down), S::ExtendDown),
                (shift_alt(Key::Up), S::MoveUp),
                (shift_alt(Key::Down), S::MoveDown),
                (shift_alt(Key::Left), S::Outdent),
                (shift(Key::Tab), S::Outdent),
                (shift_alt(Key::Right), S::Indent),
                (plain(Key::Tab), S::Indent),
                (ctrl(letter("c")), S::Copy),
                (ctrl(letter("x")), S::Cut),
                (ctrl(letter("z")), S::Undo),
                (ctrl_shift(letter("z")), S::Redo),
            ]
            .into(),
        }
    }

    /// The keys of the stickies for macOS. Command takes the place of Control where the Mac
    /// uses it, and Option is Alt. Control+O stays with the text: macOS opens a line there.
    pub fn mac() -> KeyMap {
        use EditorKey as E;
        use SelectionKey as S;
        let mut map = KeyMap::windows();
        map.editor.retain(|(chord, _)| !chord.ctrl);
        map.editor.extend([
            (ctrl(Key::Enter), E::TaskCycle),
            (cmd(Key::Enter), E::TaskCycle),
            (cmd(Key::Up), E::Fold),
            (cmd(Key::Down), E::Unfold),
            (ctrl(letter("z")), E::Undo),
            (ctrl_shift(letter("z")), E::Redo),
            (cmd(letter("z")), E::Undo),
            (cmd_shift(letter("z")), E::Redo),
            (cmd(letter("o")), E::OpenLink),
        ]);
        map.selection.retain(|(chord, _)| !chord.ctrl);
        map.selection.extend([
            (cmd(letter("c")), S::Copy),
            (cmd(letter("x")), S::Cut),
            (ctrl(letter("z")), S::Undo),
            (ctrl_shift(letter("z")), S::Redo),
            (cmd(letter("z")), S::Undo),
            (cmd_shift(letter("z")), S::Redo),
        ]);
        map
    }

    /// The keys of the web frontend. A browser on a Mac also folds with Command and copies
    /// with it.
    pub fn web() -> KeyMap {
        use EditorKey as E;
        use SelectionKey as S;
        let mut map = KeyMap::windows();
        map.editor
            .extend([(cmd(Key::Up), E::Fold), (cmd(Key::Down), E::Unfold)]);
        map.selection
            .extend([(cmd(letter("c")), S::Copy), (cmd(letter("x")), S::Cut)]);
        map
    }

    /// The event that a key sends, or None when it means nothing to the outline where it was
    /// pressed. `now_ms` is the time of the clock, for a new item.
    pub fn event_of(&self, press: &KeyPress, now_ms: u64) -> Option<Event> {
        match press {
            KeyPress::InEditor {
                item_id,
                chord,
                editor,
            } => event_of(self.in_editor(chord)?, item_id, editor, now_ms),
            KeyPress::Named {
                item_id,
                key,
                editor,
            } => event_of(*key, item_id, editor, now_ms),
            KeyPress::InSelection { chord } => self.in_selection(chord).map(Event::SelectionKey),
        }
    }

    /// What a chord means in the editor of an item. None for a chord that only means
    /// something to the text, such as Shift+Enter, which breaks the line inside the item.
    pub fn in_editor(&self, chord: &Chord) -> Option<EditorKey> {
        let found = self.editor.iter().find(|(known, _)| known == chord);
        found.map(|(_, key)| *key)
    }

    /// What a chord means while items are selected.
    pub fn in_selection(&self, chord: &Chord) -> Option<SelectionKey> {
        let found = self.selection.iter().find(|(known, _)| known == chord);
        found.map(|(_, key)| *key)
    }
}

fn chord(key: Key, shift: bool, ctrl: bool, alt: bool, cmd: bool) -> Chord {
    Chord {
        key,
        shift,
        ctrl,
        alt,
        cmd,
    }
}

fn plain(key: Key) -> Chord {
    chord(key, false, false, false, false)
}

fn shift(key: Key) -> Chord {
    chord(key, true, false, false, false)
}

fn ctrl(key: Key) -> Chord {
    chord(key, false, true, false, false)
}

fn ctrl_shift(key: Key) -> Chord {
    chord(key, true, true, false, false)
}

fn shift_alt(key: Key) -> Chord {
    chord(key, true, false, true, false)
}

fn cmd(key: Key) -> Chord {
    chord(key, false, false, false, true)
}

fn cmd_shift(key: Key) -> Chord {
    chord(key, true, false, false, true)
}

fn letter(letter: &str) -> Key {
    Key::Letter(letter.to_string())
}

/// The event that `key` sends from the editor of the item. None when the key means nothing to
/// the outline there, and the editor does with it what it does in any text: Up below the first
/// line moves the cursor in the text.
pub fn event_of(key: EditorKey, item_id: &str, editor: &EditorState, now_ms: u64) -> Option<Event> {
    let item_id = item_id.to_string();
    let start = usize::try_from(editor.selection_start).unwrap_or(usize::MAX);
    let cursor = chars_of_utf16(&editor.text, start);
    let only_cursor = editor.selection_start == editor.selection_end;
    let at_start = only_cursor && start == 0;
    let at_end = only_cursor && start >= editor.text.encode_utf16().count();
    let event = match key {
        EditorKey::Enter => Event::EnterPressed {
            item_id,
            cursor,
            now_ms,
        },
        EditorKey::Tab | EditorKey::MoveIn => Event::Indent { item_id },
        EditorKey::BackTab | EditorKey::MoveOut => Event::Outdent { item_id },
        EditorKey::Up if editor.on_first_line => Event::NavigateUp { item_id, cursor },
        EditorKey::Down if editor.on_last_line => Event::NavigateDown { item_id, cursor },
        // A place past any text: the core puts the cursor at the end of the item above. Half
        // of the largest, so that a shell that reads it as a signed number gets it whole
        EditorKey::Left if at_start => Event::NavigateUp {
            item_id,
            cursor: usize::MAX / 2,
        },
        EditorKey::Right if at_end => Event::NavigateDown { item_id, cursor: 0 },
        EditorKey::Backspace if at_start => Event::BackspaceAtStart { item_id },
        EditorKey::Delete if at_end => Event::DeleteAtEnd { item_id },
        EditorKey::TaskCycle => Event::TaskCycle { item_id },
        EditorKey::SelectUp if editor.on_first_line => Event::ShiftSelect { item_id },
        EditorKey::SelectDown if editor.on_last_line => Event::ShiftSelect { item_id },
        EditorKey::Fold => Event::SetCollapsed {
            item_id,
            collapsed: true,
        },
        EditorKey::Unfold => Event::SetCollapsed {
            item_id,
            collapsed: false,
        },
        EditorKey::MoveUp => Event::MoveUp { item_id },
        EditorKey::MoveDown => Event::MoveDown { item_id },
        EditorKey::Undo => Event::Undo,
        EditorKey::Redo => Event::Redo,
        EditorKey::OpenLink => Event::OpenLinkAt { item_id, cursor },
        EditorKey::Up
        | EditorKey::Down
        | EditorKey::Left
        | EditorKey::Right
        | EditorKey::Backspace
        | EditorKey::Delete
        | EditorKey::SelectUp
        | EditorKey::SelectDown => return None,
    };
    Some(event)
}
