//! How long the parts of the view model take to build for one key on a long page (#937).
//! These tests print times and check none of them, so they are ignored: run them in release
//! mode with
//!
//! ```text
//! cargo test --release -p client-core --lib view::timing -- --ignored --nocapture
//! ```
//!
//! The page and the keys are those of `TypingSpeedTests` of stickies-win (#824): 1000 items,
//! 100 keys into the first one.

use super::outline;
use crate::body_model::Bodies;
use crate::harness::{item, typed, Harness};
use crate::model::{Item, ItemPosition};
use crate::RichText;
use std::time::Instant;

const ITEMS: usize = 1000;
const KEYS: usize = 100;

fn timed<T>(work: impl FnOnce() -> T) -> f64 {
    let started = Instant::now();
    std::hint::black_box(work());
    started.elapsed().as_secs_f64() * 1_000.0
}

fn report(name: &str, mut ms: Vec<f64>) {
    ms.sort_by(f64::total_cmp);
    println!(
        "TIMING {name}: median {:.3}ms, worst {:.3}ms",
        ms[ms.len() / 2],
        ms[ms.len() - 1]
    );
}

// 1000項目のページで1つのキーを打ったとき、ビューモデルの各部分の組み立てにかかる時間を測る。時間は確かめず、表示するだけである。
#[test]
#[ignore = "prints times: run it in release mode with --ignored --nocapture"]
fn the_parts_of_the_view_model_of_a_key_on_a_long_page() {
    let mut h = Harness::with_items(
        (0..ITEMS)
            .map(|n| {
                let level = i32::try_from(n % 3 + 1).unwrap();
                item(&format!("{n:04}"), level, &format!("item {n}"))
            })
            .collect(),
    );
    let bodies = Bodies::<RichText>::default();
    outline(&h.model, &bodies);
    let mut parts: [(&str, Vec<f64>); 7] = [
        ("the whole view model", vec![]),
        ("the outline", vec![]),
        ("the bodies of the rows", vec![]),
        ("has_children of every row", vec![]),
        ("is_broken_transclusion of every row", vec![]),
        ("a copy of the item of every row", vec![]),
        ("all but the outline", vec![]),
    ];
    for n in 1..=KEYS {
        typed(&mut h, "0000", &"a".repeat(n));
        let state = &h.model.state;
        let visible = state.visible_items();
        let whole = timed(|| h.view());
        let outlined = timed(|| outline(&h.model, &bodies));
        let times = [
            whole,
            outlined,
            timed(|| {
                bodies.of(
                    visible.iter().map(|item| item.body.as_str()),
                    &h.model.config.tags_dir,
                )
            }),
            timed(|| {
                visible
                    .iter()
                    .filter(|item| state.has_children(&item.id))
                    .count()
            }),
            timed(|| {
                visible
                    .iter()
                    .filter(|item| state.is_broken_transclusion(&item.id))
                    .count()
            }),
            timed(|| {
                visible
                    .iter()
                    .map(|item| Item::clone(item))
                    .collect::<Vec<_>>()
            }),
            whole - outlined,
        ];
        for ((_, ms), time) in parts.iter_mut().zip(times) {
            ms.push(time);
        }
    }
    for (name, ms) in parts {
        report(name, ms);
    }
}

/// A doc page with `count` items, at the levels of the page of `TypingSpeedTests`.
fn page_of(count: usize) -> Harness {
    Harness::with_items(
        (0..count)
            .map(|n| {
                let level = i32::try_from(n % 3 + 1).unwrap();
                item(&format!("{n:04}"), level, &format!("item {n}"))
            })
            .collect(),
    )
}

// 項目が1、10、100、1000のページで、キー、項目の追加、削除、移動のそれぞれに、ビューモデルを作り終えるまでの時間を測る。索引が行数の少ないページで逆効果にならないかを見るためである（#939）。
#[test]
#[ignore = "prints times: run it in release mode with --ignored --nocapture"]
fn a_key_and_a_change_of_the_items_on_pages_of_every_length() {
    for count in [1, 10, 100, 1000] {
        let mut h = page_of(count);
        let (mut keys, mut added, mut deleted, mut moved) = (vec![], vec![], vec![], vec![]);
        let last = format!("{:04}", count - 1);
        for n in 1..=KEYS {
            keys.push(timed(|| {
                typed(&mut h, "0000", &"a".repeat(n));
                h.view()
            }));
            added.push(timed(|| {
                h.model
                    .state
                    .add_item(item("new", 2, "new"), &ItemPosition::after("0000"));
                h.view()
            }));
            deleted.push(timed(|| {
                h.model.state.delete_item("new");
                h.view()
            }));
            moved.push(timed(|| {
                // The last item goes to the top and back, by turns
                let item = Item::clone(&h.model.state.get_item_by_id(&last).unwrap());
                let place = if n % 2 == 1 {
                    ItemPosition::after("0000")
                } else {
                    ItemPosition::Last
                };
                h.model
                    .state
                    .replace_items(&[last.clone()], vec![item], &place);
                h.view()
            }));
        }
        report(&format!("{count} items: a key"), keys);
        report(&format!("{count} items: an item added"), added);
        report(&format!("{count} items: an item deleted"), deleted);
        report(&format!("{count} items: an item moved"), moved);
    }
}
