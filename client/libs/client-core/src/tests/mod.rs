//! The specs of the outline, in the order of `docs/features.md`.

// 今日を書き始める
mod blank_lines;
mod changes;
mod collapse;
mod edit;
mod home;
mod key_relay;
mod keys;
mod level;
mod list_changes;
mod merge;
mod moves;
mod split;
mod typing;
mod undo_replay;

// 日々をたどる
mod calendar;
mod day_change;
mod journal;
mod journal_fragment;
mod journal_moves;
mod stash;
mod task;

// まとめて扱う
mod drag;
mod paste;
mod selection;

// ページで知識を整える
mod backlinks;
mod delete_page;
mod new_page;
mod page_open;
mod page_properties;
mod rename;
mod route;
mod search;
mod sidebar_index;
mod sidebar_load;
mod sidebar_menu;
mod sidebar_sort;
mod tags_button;
mod transclusion;
mod view;
mod wiki;

// 地図で俯瞰する
mod atlas;

// 設定とサーバーと付き合う
mod file_reload;
mod rest;
mod settings;
mod sse;

// 外のプログラムと組む
mod launcher;
mod links;

// 開いたページをたどる
mod history;

// ネイティブのシェルとの境界
mod native_border;
