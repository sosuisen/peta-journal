use crate::app::{EditPlace, Event};
use crate::harness::{item, typed, update_calls, Harness};
use crate::model::TranscludedTrees;
use crate::testing::item_with;

// 入力したテキストはすぐに表示され、自動保存は最後の入力から一定時間後に一度だけ行われる。
#[test]
fn typing_shows_the_text_at_once_and_schedules_the_auto_save() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    typed(&mut h, "a", "hel");
    typed(&mut h, "a", "hello");
    assert_eq!(h.bodies(), ["a=hello"]);
    assert!(
        update_calls(&h.api).is_empty(),
        "nothing is sent before the pause"
    );
    assert_eq!(
        h.pending_timer_ids().len(),
        1,
        "the second keystroke replaced the first timer"
    );
    h.fire_timers();
    assert_eq!(update_calls(&h.api), [("a".into(), Some("hello".into()))]);
    assert!(h.model.edits.pending_saves.is_empty());
}

// 自動保存は、本文の前後の空行を削らない。打ちかけの行が、打っている間に消えないためである。
#[test]
fn the_auto_save_keeps_the_blank_lines_around_the_body() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    typed(&mut h, "a", "text\n");
    h.fire_timers();
    assert_eq!(update_calls(&h.api), [("a".into(), Some("text\n".into()))]);
}

// 本文が閉じていないブロックで終わる間は、自動保存は送られない。項目は保存待ちのまま残り、編集を離れるときに保存される（ADR 050）。
#[test]
fn the_auto_save_is_not_sent_while_a_block_is_unclosed() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    typed(&mut h, "a", ">>>\nabc");
    h.fire_timers();
    assert!(update_calls(&h.api).is_empty());
    assert!(h.model.edits.dirty_item_ids.contains("a"));
}

// トランスクルージョンのルート項目への入力は、元の項目に対する保存として予約される。
#[test]
fn a_transclusion_root_schedules_the_save_for_its_source_item() {
    let mut h = Harness::with_items(vec![item_with("root", 1, |i| {
        i.transclude_item_id = Some("s".into())
    })]);
    let mut trees = TranscludedTrees::new();
    trees.insert("s".into(), vec![item("s", 1, "src")]);
    h.model.state.set_transcluded_trees(trees);
    h.model.state.add_editing_item("root");
    typed(&mut h, "root", "typed");
    h.fire_timers();
    assert_eq!(update_calls(&h.api), [("s".into(), Some("typed".into()))]);
}

// 編集終了で入力したテキストが一度だけ保存され、自動保存のタイマーが取り消されてエディタが閉じられる。
#[test]
fn finish_edit_saves_typed_text_once_cancelling_the_auto_save_and_closes_the_editor() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    typed(&mut h, "a", "hello");
    h.send(Event::EditFinished {
        item_id: "a".into(),
    });
    assert_eq!(update_calls(&h.api), [("a".into(), Some("hello".into()))]);
    assert!(h.timers.is_empty(), "the auto-save timer was cancelled");
    assert!(!h.model.state.is_editing("a"));
    h.fire_timers();
    assert_eq!(update_calls(&h.api).len(), 1, "the text is not sent twice");
}

// 編集を終えると、本文の末尾の空白だけの行は削られて保存される。先頭の空行は、ファイルでは`\`だけの行として残る（ADR 070）。末尾の`\`だけの行は、ほかのMarkdownビューアで文字の`\`が見えるためである。
#[test]
fn finish_edit_saves_the_body_without_its_blank_lines_at_the_end() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    typed(&mut h, "a", " \n\n  text\nmore  \n\t\n");
    h.send(Event::EditFinished {
        item_id: "a".into(),
    });
    assert_eq!(
        update_calls(&h.api),
        [("a".into(), Some("\\\n\\\n  text\nmore  ".into()))]
    );
}

// Shift+↑↓で項目の選択を始めると、打った文字が保存される。選んだ項目を動かしても、文字は消えない（ADR 052）。
#[test]
fn starting_a_selection_from_an_editor_saves_the_typed_text() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    typed(&mut h, "a", "hello");
    h.send(Event::ShiftSelect {
        item_id: "a".into(),
    });
    assert_eq!(update_calls(&h.api), [("a".into(), Some("hello".into()))]);
    assert!(h.model.state.item_selection_mode());
}

// 変更のない項目の編集終了では保存されずにエディタが閉じられる。
#[test]
fn finish_edit_closes_the_editor_of_a_clean_item_without_a_save() {
    let mut h = Harness::with_items(vec![item("a", 1, "x")]);
    h.model.state.add_editing_item("a");
    h.send(Event::EditFinished {
        item_id: "a".into(),
    });
    assert!(h.api.calls.is_empty());
    assert!(!h.model.state.is_editing("a"));
}

// 表示していない項目の編集終了は、保存するものがなくても、その編集中の印を消す。印が残ると、他のクライアントの変更が取り込まれなくなる。
#[test]
fn finish_edit_of_an_item_not_shown_drops_its_edit_mark() {
    let mut h = Harness::with_items(vec![item("a", 1, "x")]);
    h.model.state.add_editing_item("gone");
    h.send(Event::EditFinished {
        item_id: "gone".into(),
    });
    assert!(h.api.calls.is_empty());
    assert!(!h.model.state.is_editing("gone"));
}

// 編集終了で、閉じていないブロックは閉じる記号を補われて保存される。どの種類のブロックでも、何も入力されていなくても同様である。
#[test]
fn finish_edit_closes_an_unclosed_block_even_when_nothing_was_typed_since() {
    for (body, closed) in [
        ("```js", "```js\n```"),
        (">>>\nabc", ">>>\nabc\n>>>"),
        ("<!--\nabc", "<!--\nabc\n-->"),
    ] {
        let mut h = Harness::with_items(vec![item("a", 1, body)]);
        h.model.state.add_editing_item("a");
        h.send(Event::EditFinished {
            item_id: "a".into(),
        });
        assert_eq!(
            update_calls(&h.api),
            [("a".into(), Some(closed.into()))],
            "{body:?}"
        );
    }
}

// 引き継ぎがその項目を保存している間は、編集終了が無視される。
#[test]
fn finish_edit_is_ignored_while_a_handover_is_saving_the_item_itself() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    h.model.edits.edit_handovers = 1;
    typed(&mut h, "a", "hello");
    h.send(Event::EditFinished {
        item_id: "a".into(),
    });
    assert!(update_calls(&h.api).is_empty());
    assert!(h.model.state.is_editing("a"));
}

// 別の項目をクリックすると変更中の項目が保存され、クリックされた項目のエディタが開かれる。
#[test]
fn clicking_another_item_saves_the_dirty_one_and_opens_the_clicked_editor() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    h.send(Event::EditRequested {
        item_id: "a".into(),
        place: EditPlace::Markdown(0),
    });
    typed(&mut h, "a", "typed");
    h.send(Event::EditRequested {
        item_id: "b".into(),
        place: EditPlace::Markdown(0),
    });
    assert_eq!(update_calls(&h.api), [("a".into(), Some("typed".into()))]);
    assert!(!h.model.state.is_editing("a"));
    assert!(h.model.state.is_editing("b"));
    assert_eq!(h.model.edits.edit_handovers, 0);
}

// 別の項目をクリックすると、閉じていないブロックは閉じる記号を補われて保存される。編集を離れる操作だからである。
#[test]
fn clicking_another_item_closes_the_unclosed_block_of_the_one_left() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    h.send(Event::EditRequested {
        item_id: "a".into(),
        place: EditPlace::Markdown(0),
    });
    typed(&mut h, "a", "~~~\ncode");
    h.send(Event::EditRequested {
        item_id: "b".into(),
        place: EditPlace::Markdown(0),
    });
    assert_eq!(
        update_calls(&h.api),
        [("a".into(), Some("~~~\ncode\n~~~".into()))]
    );
}

// 別の項目をクリックしたときの保存が失敗しても、引き継ぎは終わる。クリックされた項目のエディタは開き、保存できなかった項目は編集中のまま残る。
#[test]
fn a_failed_save_on_clicking_another_item_still_ends_the_handover() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    h.send(Event::EditRequested {
        item_id: "a".into(),
        place: EditPlace::Markdown(0),
    });
    typed(&mut h, "a", "typed");
    h.api.fail_next("UpdateItem", "offline");
    h.send(Event::EditRequested {
        item_id: "b".into(),
        place: EditPlace::Markdown(0),
    });
    assert_eq!(h.model.edits.edit_handovers, 0);
    assert!(h.model.state.is_editing("b"));
    assert!(h.model.state.is_editing("a"));
}

// 何も入力していない項目から別の項目をクリックすると、保存は送られず、引き継ぎも残らない。
#[test]
fn clicking_another_item_from_a_clean_one_saves_nothing_and_leaves_no_handover() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    h.send(Event::EditRequested {
        item_id: "a".into(),
        place: EditPlace::Markdown(0),
    });
    h.send(Event::EditRequested {
        item_id: "b".into(),
        place: EditPlace::Markdown(0),
    });
    assert!(update_calls(&h.api).is_empty());
    assert_eq!(h.model.edits.edit_handovers, 0);
    assert!(h.model.state.is_editing("b"));
}

// 矢印キーでの移動のときの保存が失敗しても、エディタは移動先の項目に移される。
#[test]
fn arrow_navigation_moves_the_editor_even_when_the_save_fails() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    h.model.state.add_editing_item("a");
    typed(&mut h, "a", "x");
    h.api.fail_next("UpdateItem", "offline");
    h.send(Event::NavigateDown {
        item_id: "a".into(),
        cursor: 3,
    });
    assert!(h.model.state.is_editing("b"));
    assert!(!h.model.state.is_editing("a"));
    assert_eq!(h.focus_cursor(), Some(3));
}

// 矢印キーでの移動で項目が保存され、カーソル桁を保ったままエディタが移動先の項目に移される。
#[test]
fn arrow_navigation_saves_and_moves_the_editor_with_the_cursor_column() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    h.model.state.add_editing_item("a");
    typed(&mut h, "a", "x");
    h.send(Event::NavigateDown {
        item_id: "a".into(),
        cursor: 3,
    });
    assert!(h.model.state.is_editing("b"));
    assert!(!h.model.state.is_editing("a"));
    assert_eq!(h.focus_cursor(), Some(3));
    assert_eq!(update_calls(&h.api), [("a".into(), Some("x".into()))]);
}

// 上矢印は、カーソル桁を保ったままエディタを上の項目に移す。先頭の項目では何も起きない。
#[test]
fn arrow_up_moves_the_editor_to_the_item_above_and_stops_at_the_top() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    h.model.state.add_editing_item("b");
    h.send(Event::NavigateUp {
        item_id: "b".into(),
        cursor: 2,
    });
    assert!(h.model.state.is_editing("a"));
    assert!(!h.model.state.is_editing("b"));
    assert_eq!(h.focus_cursor(), Some(2));

    h.send(Event::NavigateUp {
        item_id: "a".into(),
        cursor: 2,
    });
    assert!(h.model.state.is_editing("a"));
}

// 編集を終えると、開いていたページ名のポップアップも閉じる。
#[test]
fn finish_edit_closes_the_wiki_popup() {
    let mut h = Harness::with_items(vec![item("a", 1, "[[To]]")]);
    h.model.state.add_editing_item("a");
    h.model.wiki.item_id = Some("a".into());
    h.model.wiki.suggestions = vec!["Todo".into()];
    h.send(Event::EditFinished {
        item_id: "a".into(),
    });
    assert_eq!(h.view().outline.wiki.item_id, None);
}

// トランスクルージョンのルートの保存で、サーバーが返した種類は、表示しているルートにも写される。
#[test]
fn saving_a_transclusion_root_copies_the_parsed_types_to_the_shown_root() {
    let mut h = Harness::with_items(vec![item_with("root", 1, |i| {
        i.transclude_item_id = Some("s".into())
    })]);
    let mut trees = TranscludedTrees::new();
    trees.insert("s".into(), vec![item("s", 1, "src")]);
    h.model.state.set_transcluded_trees(trees);
    h.model.state.add_editing_item("root");
    typed(&mut h, "root", "# typed");
    h.api.answer_next(
        "UpdateItem",
        Ok(crate::effects::ApiOutput::Item(item_with("s", 1, |i| {
            i.body = "# typed".into();
            i.types = vec!["heading".into()];
        }))),
    );
    h.send(Event::EditFinished {
        item_id: "root".into(),
    });
    let root = h.model.state.get_item_by_id("root").unwrap();
    assert_eq!(root.types, ["heading"]);
}

// 表示した文字に触れると、その項目の編集に入り、カーソルは触れた文字のMarkdownでの位置に置かれる。
#[test]
fn a_touch_on_the_shown_text_edits_the_item_at_the_place_touched_in_its_markdown() {
    let mut h = Harness::with_items(vec![item(
        "a",
        1,
        "**bold** and [link](https://x.example/a) end",
    )]);

    // The shown text is "bold and link end": the touch is after "bold and li"
    h.send(Event::EditRequested {
        item_id: "a".into(),
        place: EditPlace::ShownLetter("bold and li".chars().count()),
    });

    assert!(h.model.state.is_editing("a"));
    let request = h.model.state.focus_request().expect("a focus request");
    assert_eq!(request.item_id, "a");
    assert_eq!(request.cursor, Some("**bold** and [li".chars().count()));
}

// 表示した文字のうち、触れた位置より前にある文字の列が届くと、その項目の編集に入る。カーソルは、その文字の列の
// 後ろにあたるMarkdownの位置に置かれる。Web版は、ページの上の文字の列をそのまま送る（ADR 044）。
#[test]
fn the_shown_letters_before_a_touch_put_the_cursor_after_them_in_the_markdown() {
    let mut h = Harness::with_items(vec![item(
        "a",
        1,
        "**bold** and [link](https://x.example/a) end",
    )]);

    h.send(Event::EditRequested {
        item_id: "a".into(),
        place: EditPlace::AfterShownText("bold and li".into()),
    });

    assert!(h.model.state.is_editing("a"));
    let request = h.model.state.focus_request().expect("a focus request");
    assert_eq!(request.cursor, Some("**bold** and [li".chars().count()));
}

// ページの上の文字の列には、タグの間の改行のように、Markdownにない文字が混ざる。それらは飛ばされる。
#[test]
fn letters_of_the_page_that_the_markdown_lacks_are_left_out() {
    let mut h = Harness::with_items(vec![item(
        "a",
        1,
        "- first
- second",
    )]);

    h.send(Event::EditRequested {
        item_id: "a".into(),
        place: EditPlace::AfterShownText(
            "

first

se"
            .into(),
        ),
    });

    let request = h.model.state.focus_request().expect("a focus request");
    assert_eq!(
        request.cursor,
        Some(
            "- first
- se"
                .chars()
                .count()
        )
    );
}

// 絵文字の途中に触れても、カーソルは絵文字の途中に入らず、その後ろに置かれる。そこでEnterを押しても、
// 絵文字は2つに割れない（ADR 043）。
#[test]
fn a_touch_inside_an_emoji_puts_the_cursor_after_the_whole_emoji() {
    let mut h = Harness::with_items(vec![item("a", 1, "a👨‍👩‍👧b")]);

    // The family is five characters: three people joined by two zero width joiners
    h.send(Event::EditRequested {
        item_id: "a".into(),
        place: EditPlace::ShownLetter("a👨".chars().count()),
    });

    let request = h.model.state.focus_request().expect("a focus request");
    assert_eq!(request.cursor, Some("a👨‍👩‍👧".chars().count()));
}

// 位置のない編集の始まりでは、カーソルが本文の末尾に置かれる（ADR 057）。
#[test]
fn an_edit_without_a_place_puts_the_cursor_at_the_end() {
    let mut h = Harness::with_items(vec![item("a", 1, "![a cat](assets/cat.png)")]);
    h.send(Event::EditRequested {
        item_id: "a".into(),
        place: EditPlace::Unplaced,
    });
    let request = h.model.state.focus_request().expect("a focus request");
    assert_eq!(
        request.cursor,
        Some("![a cat](assets/cat.png)".chars().count())
    );
}

// Markdownの位置での編集の始まりでは、カーソルがその位置に置かれる。0は、本当の先頭である。
#[test]
fn an_edit_at_a_place_in_the_markdown_puts_the_cursor_there() {
    let mut h = Harness::with_items(vec![item("a", 1, "text")]);
    h.send(Event::EditRequested {
        item_id: "a".into(),
        place: EditPlace::Markdown(0),
    });
    let request = h.model.state.focus_request().expect("a focus request");
    assert_eq!(request.cursor, Some(0));
}

// 選択中に項目の編集を始めると、選択が解けて、その項目が編集される（ADR 057）。
#[test]
fn an_edit_while_items_are_selected_ends_the_selection_and_edits_the_item() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    h.model.state.add_editing_item("a");
    h.send(Event::ShiftSelect {
        item_id: "a".into(),
    });
    assert!(h.model.state.item_selection_mode());
    h.send(Event::EditRequested {
        item_id: "b".into(),
        place: EditPlace::Unplaced,
    });
    assert!(!h.model.state.item_selection_mode());
    assert!(h.model.state.selected_item_ids().is_empty());
    assert!(h.model.state.is_editing("b"));
}

// 打った文字のない項目から別の項目へ移ると、前の項目は編集中でなくなる。シェルが編集の終わりを知らせなくてもよい（ADR 057）。
#[test]
fn leaving_an_item_without_typed_text_ends_its_edit() {
    let mut h = Harness::with_items(vec![item("a", 1, "a"), item("b", 1, "b")]);
    h.send(Event::EditRequested {
        item_id: "a".into(),
        place: EditPlace::Unplaced,
    });
    h.send(Event::EditRequested {
        item_id: "b".into(),
        place: EditPlace::Unplaced,
    });
    assert!(!h.model.state.is_editing("a"));
    assert!(update_calls(&h.api).is_empty());
}

// 打った文字のある項目から別の項目へ移ると、本文の前後の空白だけの行は削られて保存される。空白だけの本文は空になる。
#[test]
fn leaving_an_item_saves_its_body_without_the_blank_lines_before_and_after() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "b")]);
    h.send(Event::EditRequested {
        item_id: "a".into(),
        place: EditPlace::Unplaced,
    });
    typed(&mut h, "a", "  \n ");
    h.send(Event::EditRequested {
        item_id: "b".into(),
        place: EditPlace::Unplaced,
    });
    assert_eq!(update_calls(&h.api), [("a".into(), Some(String::new()))]);
}

// 閉じていないブロックのある項目から、文字を打たずに別の項目へ移ると、ブロックが閉じて保存される。
#[test]
fn leaving_an_item_with_an_unclosed_block_saves_it_closed() {
    let mut h = Harness::with_items(vec![item("a", 1, "~~~\ncode"), item("b", 1, "")]);
    h.send(Event::EditRequested {
        item_id: "a".into(),
        place: EditPlace::Unplaced,
    });
    h.send(Event::EditRequested {
        item_id: "b".into(),
        place: EditPlace::Unplaced,
    });
    assert_eq!(
        update_calls(&h.api),
        [("a".into(), Some("~~~\ncode\n~~~".into()))]
    );
    assert!(!h.model.state.is_editing("a"));
}

// 別の項目へ移ると、前の項目のWikiリンクの候補が閉じる。打った文字があってもなくても同じである。
#[test]
fn leaving_an_item_closes_its_wiki_link_suggestions() {
    for body in [None, Some("[[pa")] {
        let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
        h.send(Event::EditRequested {
            item_id: "a".into(),
            place: EditPlace::Unplaced,
        });
        if let Some(body) = body {
            typed(&mut h, "a", body);
        }
        h.model.wiki.item_id = Some("a".into());
        h.send(Event::EditRequested {
            item_id: "b".into(),
            place: EditPlace::Unplaced,
        });
        assert_eq!(h.model.wiki.item_id, None, "typed: {body:?}");
    }
}

// 前の項目の保存中に別の項目へ移っても、打った文字のない項目の編集は終わる。
#[test]
fn an_item_left_during_a_handover_ends_its_edit() {
    let mut h = Harness::with_items(vec![item("a", 1, "a"), item("b", 1, "b")]);
    h.send(Event::EditRequested {
        item_id: "a".into(),
        place: EditPlace::Unplaced,
    });
    h.model.edits.edit_handovers = 1;
    h.send(Event::EditRequested {
        item_id: "b".into(),
        place: EditPlace::Unplaced,
    });
    assert!(!h.model.state.is_editing("a"));
}

// 編集中でない項目から移っても、何も保存されない。
#[test]
fn leaving_an_item_that_is_not_edited_saves_nothing() {
    let mut h = Harness::with_items(vec![item("a", 1, "~~~\ncode"), item("b", 1, "")]);
    h.send(Event::EditRequested {
        item_id: "a".into(),
        place: EditPlace::Unplaced,
    });
    h.model.state.remove_editing_item("a");
    h.send(Event::EditRequested {
        item_id: "b".into(),
        place: EditPlace::Unplaced,
    });
    assert!(update_calls(&h.api).is_empty());
}

// 開き括弧を打つと、閉じる側が足される（ADR 050、ADR 057）。
#[test]
fn typing_an_opening_closes_it() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    h.send(Event::Typed {
        item_id: "a".into(),
        body: "[[".into(),
        cursor: 2,
        now_ms: 0,
    });
    assert_eq!(h.model.state.get_item_by_id("a").unwrap().body, "[[]]");
}

// 消して開き括弧の後ろに戻ったときは、閉じる側が足されない。消すたびに閉じる側が戻らないためである。
#[test]
fn deleting_back_to_an_opening_does_not_close_it() {
    let mut h = Harness::with_items(vec![item("a", 1, "[[a")]);
    h.model.state.add_editing_item("a");
    h.send(Event::Typed {
        item_id: "a".into(),
        body: "[[".into(),
        cursor: 2,
        now_ms: 0,
    });
    assert_eq!(h.model.state.get_item_by_id("a").unwrap().body, "[[");
}

// 本文を先に知らせたシェルが、同じ本文で打ったことを知らせても、閉じる側が足される。Emacs版がそうする。
#[test]
fn typing_told_after_the_same_body_closes_the_opening() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    typed(&mut h, "a", "[[");
    h.send(Event::Typed {
        item_id: "a".into(),
        body: "[[".into(),
        cursor: 2,
        now_ms: 0,
    });
    assert_eq!(h.model.state.get_item_by_id("a").unwrap().body, "[[]]");
}

// 打った本文は、閉じる側がなくても、本文の変更として自動保存の予定に入る。
#[test]
fn what_is_typed_is_saved_as_a_change_of_the_body() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    h.send(Event::Typed {
        item_id: "a".into(),
        body: "plain".into(),
        cursor: 5,
        now_ms: 0,
    });
    assert_eq!(h.model.state.get_item_by_id("a").unwrap().body, "plain");
    assert!(h.model.edits.dirty_item_ids.contains("a"));
}

// 編集中の項目がほかの端末で消えたあとに別の項目へ移ると、消えた項目の編集の印が外れる。
#[test]
fn leaving_an_item_deleted_meanwhile_drops_its_editing_mark() {
    let mut h = Harness::with_items(vec![item("a", 1, "a"), item("b", 1, "b")]);
    h.send(Event::EditRequested {
        item_id: "a".into(),
        place: EditPlace::Unplaced,
    });
    h.model.state.delete_item("a");
    h.send(Event::EditRequested {
        item_id: "b".into(),
        place: EditPlace::Unplaced,
    });
    assert!(!h.model.state.is_editing("a"));
    assert!(update_calls(&h.api).is_empty());
}

/// A page that shows the item `s` of another page as the transclusion root `root`.
fn transclusion_of(body: &str) -> Harness {
    let mut h = Harness::with_items(vec![item_with("root", 1, |i| {
        i.transclude_item_id = Some("s".into())
    })]);
    let mut trees = TranscludedTrees::new();
    trees.insert("s".into(), vec![item("s", 1, body)]);
    h.model.state.set_transcluded_trees(trees);
    h.model.state.add_editing_item("root");
    h
}

/// The body that the row of `item_id` shows.
fn shown_body(h: &Harness, item_id: &str) -> String {
    let rows = h.view().outline.rows;
    rows.iter()
        .find(|r| r.item.id == item_id)
        .map(|r| r.item.body.clone())
        .unwrap()
}

// トランスクルージョンのルート項目に打った文字は、保存を待たずに、その行に出る。打った文字は元の項目の本文に入るためである。
#[test]
fn text_typed_into_a_transclusion_root_shows_in_its_row_before_the_save() {
    let mut h = transclusion_of("memo");
    typed(&mut h, "root", "memo typed");
    assert_eq!(shown_body(&h, "root"), "memo typed");
}

// トランスクルージョンのルート項目でも、開き括弧を打つと閉じる側が足され、行に出る（ADR 057）。
#[test]
fn an_opening_typed_into_a_transclusion_root_is_closed_in_its_row() {
    let mut h = transclusion_of("memo");
    h.send(Event::Typed {
        item_id: "root".into(),
        body: "memo[[".into(),
        cursor: 6,
        now_ms: 0,
    });
    assert_eq!(shown_body(&h, "root"), "memo[[]]");
}

// 元の項目が同じページにもあるとき、トランスクルージョンに打った文字は、元の項目の行にも出る。
#[test]
fn text_typed_into_a_transclusion_root_shows_in_the_row_of_its_source_on_the_same_page() {
    let mut h = Harness::with_items(vec![
        item("s", 1, "memo"),
        item_with("root", 1, |i| i.transclude_item_id = Some("s".into())),
    ]);
    let mut trees = TranscludedTrees::new();
    trees.insert("s".into(), vec![item("s", 1, "memo")]);
    h.model.state.set_transcluded_trees(trees);
    h.model.state.add_editing_item("root");
    typed(&mut h, "root", "memo typed");
    assert_eq!(shown_body(&h, "root"), "memo typed");
    assert_eq!(shown_body(&h, "s"), "memo typed");
}

// トランスクルージョンのルート項目に打った文字を取り消すと、行は打つ前の本文に戻る。
#[test]
fn undoing_text_typed_into_a_transclusion_root_shows_the_body_before_it() {
    let mut h = transclusion_of("memo");
    typed(&mut h, "root", "memo typed");
    h.send(Event::Undo);
    assert_eq!(shown_body(&h, "root"), "memo");
}
