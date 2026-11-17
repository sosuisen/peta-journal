use crate::app::Event;
use crate::drag::{edge_step, place_in_list, DragMove, ItemDrag};
use crate::harness::{item, typed, update_calls, Harness};

/// The events are as expected. `Event` has no `PartialEq`, so they are compared as text.
macro_rules! assert_events {
    ($left:expr, $right:expr $(, $($message:tt)+)?) => {
        assert_eq!(
            format!("{:?}", $left),
            format!("{:?}", $right) $(, $($message)+)?
        )
    };
}

/// The mouse went down on the text of `a`, which stands from 100 to 130.
fn pressed_on_a() -> ItemDrag {
    let mut drag = ItemDrag::default();
    let events = drag.events_of(DragMove::Pressed {
        item_id: "a".into(),
        top: 100.0,
        bottom: 130.0,
    });
    assert_events!(events, Vec::<Event>::new());
    drag
}

fn moved(drag: &mut ItemDrag, y: f64, item_under: Option<&str>) -> Vec<Event> {
    drag.events_of(DragMove::Moved {
        y,
        item_under: item_under.map(Into::into),
    })
}

fn enter(item_id: &str) -> Event {
    Event::ItemMouseEnter {
        item_id: item_id.into(),
    }
}

// 押した項目のテキストの中と、その上下10までで動くあいだは、何も送らない。文字を選ぶドラッグである。
#[test]
fn moving_within_10_of_the_pressed_text_sends_nothing() {
    for y in [90.0, 115.0, 140.0] {
        let mut drag = pressed_on_a();

        assert_events!(
            moved(&mut drag, y, Some("b")),
            Vec::<Event>::new(),
            "at {y}"
        );
    }
}

// 押していないときの動きでは、何も送らない。
#[test]
fn a_move_without_the_button_pressed_sends_nothing() {
    let mut drag = ItemDrag::default();

    assert_events!(moved(&mut drag, 300.0, Some("b")), Vec::<Event>::new());
}

// 選択が始まる前に離したときは、何も送らない。文字を選んだだけである。
#[test]
fn releasing_before_a_selection_starts_sends_nothing() {
    let mut drag = pressed_on_a();
    moved(&mut drag, 120.0, Some("a"));

    assert_events!(drag.events_of(DragMove::Released), Vec::<Event>::new());
}

// テキストの上か下へ10より大きく出ると、押した項目の選択を始める。
#[test]
fn leaving_the_text_by_more_than_10_selects_the_pressed_item() {
    for y in [89.0, 141.0] {
        let mut drag = pressed_on_a();

        let events = moved(&mut drag, y, Some("a"));

        let start = Event::DragBeyondBounds {
            item_id: "a".into(),
        };
        assert_events!(events, [start], "at {y}");
    }
}

// 選択を始めた動きで、マウスが別の項目の上にあれば、選択はそこまで伸びる。
#[test]
fn the_move_that_starts_the_selection_over_another_item_extends_it_there() {
    let mut drag = pressed_on_a();

    let events = moved(&mut drag, 160.0, Some("b"));

    let start = Event::DragBeyondBounds {
        item_id: "a".into(),
    };
    assert_events!(events, [start, enter("b")]);
}

// 選択中に別の項目へ入ると、選択がそこまで伸びる。同じ項目へは2度送らない。
#[test]
fn entering_another_item_during_the_selection_extends_it_there_once() {
    let mut drag = pressed_on_a();
    moved(&mut drag, 141.0, Some("a"));

    let events: Vec<Event> = [
        (150.0, Some("b")),
        (155.0, Some("b")),
        (160.0, None),
        (180.0, Some("c")),
    ]
    .into_iter()
    .flat_map(|(y, under)| moved(&mut drag, y, under))
    .collect();

    assert_events!(events, [enter("b"), enter("c")]);
}

// ボタンを離すと、ドラッグが終わる。そのあとの動きでは、何も送らない。
#[test]
fn releasing_the_button_ends_the_drag_and_later_moves_send_nothing() {
    let mut drag = pressed_on_a();
    moved(&mut drag, 141.0, Some("b"));

    assert_events!(
        drag.events_of(DragMove::Released),
        [Event::SelectionMouseUp]
    );
    assert_events!(moved(&mut drag, 180.0, Some("c")), Vec::<Event>::new());
}

// 選択の後に押し直したドラッグは、テキストの中では何も送らない。前の選択の続きではない。
#[test]
fn a_new_press_after_a_selection_starts_a_new_drag() {
    let mut drag = pressed_on_a();
    moved(&mut drag, 141.0, Some("b"));
    drag.events_of(DragMove::Pressed {
        item_id: "c".into(),
        top: 300.0,
        bottom: 330.0,
    });

    assert_events!(moved(&mut drag, 310.0, Some("c")), Vec::<Event>::new());
}

// ドラッグで項目の選択を始めると、打った文字が保存される。Shift+↑↓と同じである。
#[test]
fn a_drag_that_starts_a_selection_saves_the_typed_text() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    typed(&mut h, "a", "hello");

    h.send(Event::DragBeyondBounds {
        item_id: "a".into(),
    });

    assert_eq!(update_calls(&h.api), [("a".into(), Some("hello".into()))]);
    assert!(h.model.selection.is_dragging());
}

// リストの上端か下端から40の帯では、深いほど速くスクロールする。端と、その外では1回に24である。帯の外ではスクロールしない。
#[test]
fn the_list_scrolls_faster_the_deeper_the_mouse_is_in_the_band_at_an_edge() {
    let steps = [
        (150.0, 0.0),
        (140.0, 0.0),
        (120.0, -12.0),
        (100.0, -24.0),
        (80.0, -24.0),
        (460.0, 0.0),
        (480.0, 12.0),
        (500.0, 24.0),
        (520.0, 24.0),
    ];
    for (y, step) in steps {
        assert_eq!(edge_step(y, 100.0, 500.0), step, "at {y}");
    }
}

// マウスの下の項目は、見えているリストの中で探す。リストの上や下の外にあるマウスは、見えている範囲のいちばん近い場所にあるとみなす。マウスはウインドウの外へも出るからである。
#[test]
fn the_item_under_the_mouse_is_looked_for_at_the_nearest_place_of_the_list_that_can_be_seen() {
    let places = [
        (-200.0, 101.0),
        (100.0, 101.0),
        (300.0, 300.0),
        (500.0, 499.0),
        (900.0, 499.0),
    ];
    for (y, in_list) in places {
        assert_eq!(place_in_list(y, 100.0, 500.0), in_list, "at {y}");
    }
}
