//! The keys pressed while the edit moves to another item are held by the core until the field
//! of that item takes the keyboard (ADR 087).

use crate::app::Event;
use crate::effects::UiOp;
use crate::harness::{item, Harness};
use crate::model::DeferredKey;

fn key(k: &str) -> DeferredKey {
    DeferredKey {
        key: k.into(),
        code: format!("Key{}", k.to_uppercase()),
        shift_key: false,
        ctrl_key: false,
        alt_key: false,
        meta_key: false,
    }
}

/// Items `a` and `b`, `a` edited. The field of an order to focus an editor takes the
/// keyboard only at `take_focus`.
fn harness() -> Harness {
    let mut h = Harness::with_items(vec![item("a", 1, "hello world"), item("b", 1, "body of b")]);
    h.model.state.add_editing_item("a");
    h.focus_takes_time = true;
    h
}

fn enter(h: &mut Harness, item_id: &str, cursor: usize) {
    let now = h.now_ms;
    h.send(Event::EnterPressed {
        item_id: item_id.into(),
        cursor,
        now_ms: now,
    });
}

/// The keys of each order to focus an editor, in order.
fn replayed(h: &Harness) -> Vec<(Option<String>, Vec<String>)> {
    h.ui.iter()
        .filter_map(|op| match op {
            UiOp::FocusEditor {
                item_id,
                replay_keys,
            } => Some((
                item_id.clone(),
                replay_keys.iter().map(|k| k.key.clone()).collect(),
            )),
            _ => None,
        })
        .collect()
}

// エディタにフォーカスする指示を出したあとも、シェルが答えるまで、コアはキーを預かり続ける。
#[test]
fn the_core_holds_keys_until_the_shell_answers_the_order_to_focus_an_editor() {
    let mut h = harness();
    enter(&mut h, "a", 5);
    assert_eq!(replayed(&h), [(Some("new1".to_string()), vec![])]);
    assert!(h.view().outline.key_relay_holding);

    h.take_focus();

    assert!(!h.view().outline.key_relay_holding);
    assert_eq!(replayed(&h).len(), 1, "no keys, no second order");
}

// 答えまでに預かったキーは、同じ項目へのもう一度のフォーカスの指示で渡り、その指示の答えで預かるのは終わる。
#[test]
fn keys_held_before_the_answer_come_with_a_second_order_to_focus_the_same_editor() {
    let mut h = harness();
    enter(&mut h, "a", 5);
    h.send(Event::KeyHeld(key("x")));
    h.send(Event::KeyHeld(key("y")));

    h.take_focus();
    h.take_focus();

    assert_eq!(
        replayed(&h),
        [
            (Some("new1".to_string()), vec![]),
            (
                Some("new1".to_string()),
                vec!["x".to_string(), "y".to_string()]
            ),
        ]
    );
    assert!(!h.view().outline.key_relay_holding);
}

// キーを渡す注文の答えを待つ間も、コアは預かり続ける。その間のキーは、答えのあとの注文で渡る。注文より先に欄がキーボードを取り、キーが注文を追い越すシェルがあるためである。
#[test]
fn keys_held_while_the_order_that_carries_keys_waits_come_with_the_order_after_its_answer() {
    let mut h = harness();
    enter(&mut h, "a", 5);
    h.send(Event::KeyHeld(key("x")));
    h.take_focus();
    assert!(h.view().outline.key_relay_holding);

    h.send(Event::KeyHeld(key("y")));
    h.take_focus();
    assert!(h.view().outline.key_relay_holding);
    h.take_focus();

    assert!(!h.view().outline.key_relay_holding);
    assert_eq!(
        replayed(&h),
        [
            (Some("new1".to_string()), vec![]),
            (Some("new1".to_string()), vec!["x".to_string()]),
            (Some("new1".to_string()), vec!["y".to_string()]),
        ]
    );
}

// 答えを待つ間に次の移動が始まると、前の指示の答えでは預かるのをやめない。キーは、あとの指示の答えのあとで渡る。
#[test]
fn the_answer_to_an_order_that_a_later_move_overtook_does_not_end_the_hold() {
    let mut h = harness();
    enter(&mut h, "a", 5);
    // Joins b to new1: the edit moves again before the field of new1 took the keyboard
    h.send(Event::BackspaceAtStart {
        item_id: "b".into(),
    });
    h.send(Event::KeyHeld(key("x")));

    h.take_focus_of_the_oldest_order();
    assert!(h.view().outline.key_relay_holding);

    h.take_focus_of_the_oldest_order();
    assert_eq!(
        replayed(&h).last(),
        Some(&(Some("new1".to_string()), vec!["x".to_string()]))
    );
    h.take_focus();
    assert!(!h.view().outline.key_relay_holding);
}
