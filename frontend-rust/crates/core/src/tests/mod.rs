//! The specs of the outline, in the order of `docs/features.md`.

// 今日を書き始める
mod collapse;
mod edit;
mod home;
mod level;
mod merge;
mod moves;
mod split;
mod undo_replay;

// 日々をたどる
mod journal_fragment;
mod journal_moves;
mod stash;
mod task;

// まとめて扱う
mod paste;
mod selection;

// ページで知識を整える
mod backlinks;
mod page_properties;
mod rename;
mod sidebar_menu;
mod sidebar_sort;
mod tags_button;
mod transclusion;
mod view;

// 地図で俯瞰する
mod atlas;

// 設定とサーバーと付き合う
mod file_reload;
mod settings;

// 外のプログラムと組む
mod extensions;
