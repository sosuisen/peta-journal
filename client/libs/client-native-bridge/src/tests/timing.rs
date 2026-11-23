//! How long the parts of `changes()` take for one key on a long page (#937). These tests print
//! times and check none of them, so they are ignored: run them in release mode with
//!
//! ```text
//! cargo test --release -p client-native-bridge --lib timing -- --ignored --nocapture
//! ```
//!
//! The page and the keys are those of `TypingSpeedTests` of stickies-win (#824): 1000 items,
//! 100 keys into the first one.

use crate::{Clock, Host, NativeShell};
use client_core::effects::{ApiError, ApiOp, ApiOutput, ApiResult, LocalDateTimeDto, SseEvent};
use client_core::model::Page;
use client_core::rest::ClientId;
use client_core::testing::{page_answer, plain_item};
use client_core::typing::Typing;
use client_core::{EditPlace, Event, Route};
use crux_core::bridge::{BincodeFfiFormat, FfiFormat};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

const ITEMS: usize = 1000;
const KEYS: usize = 100;

/// A journal server with one long page, `long`. It knows nothing else.
struct LongPage;

impl Host for LongPage {
    fn call(&self, op: ApiOp) -> ApiResult<ApiOutput> {
        match op {
            ApiOp::GetOrCreateDoc { .. } | ApiOp::GetPageById { .. } => {
                let page = Page {
                    id: "p".into(),
                    name: "long".into(),
                    page_type: "doc".into(),
                    ..Default::default()
                };
                // The levels of the page of TypingSpeedTests
                let items = (0..ITEMS)
                    .map(|n| {
                        let level = i32::try_from(n % 3 + 1).unwrap();
                        plain_item(&format!("{n:04}"), level, &format!("item {n}"))
                    })
                    .collect();
                Ok(page_answer(&op, page, items))
            }
            _ => Err(ApiError("the long page is all this server knows".into())),
        }
    }

    fn upload(&self, _path: &Path) -> ApiResult<ApiOutput> {
        Err(ApiError("the long page is all this server knows".into()))
    }

    fn events(&self, _send: &mut dyn FnMut(SseEvent)) {}
}

/// A clock whose waits do not end while the test runs: the edit is never saved, and the core
/// does not ask the server again.
struct StillClock;

impl Clock for StillClock {
    fn now(&self) -> (LocalDateTimeDto, u64) {
        let local = LocalDateTimeDto {
            year: 2026,
            month: 11,
            day: 22,
            hour: 9,
            minute: 0,
        };
        (local, 1_000)
    }

    fn sleep(&self, _ms: u32) {
        thread::sleep(Duration::from_secs(3_600));
    }
}

fn bytes_of<T: serde::Serialize>(value: &T) -> Vec<u8> {
    let mut bytes = Vec::new();
    BincodeFfiFormat::serialize(&mut bytes, value).unwrap();
    bytes
}

/// A shell with the long page open, the first changes read, and the first item edited.
fn shell_on_the_long_page() -> NativeShell {
    let shell = NativeShell::new(LongPage, StillClock, ClientId::new(), |_| {});
    shell
        .update(&bytes_of(&Event::Route {
            route: Route::Page {
                name: "long".into(),
                fragment: None,
            },
            today: "2026-11-22".into(),
        }))
        .unwrap();
    let started = Instant::now();
    while shell.whole_view().outline.rows.len() < ITEMS {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the long page never came"
        );
        thread::sleep(Duration::from_millis(5));
    }
    shell.take_changes();
    shell
        .update(&bytes_of(&Event::EditRequested {
            item_id: "0000".into(),
            place: EditPlace::Markdown(0),
        }))
        .unwrap();
    shell.take_changes();
    shell
}

fn timed<T>(work: impl FnOnce() -> T) -> (T, f64) {
    let started = Instant::now();
    let done = work();
    (done, started.elapsed().as_secs_f64() * 1_000.0)
}

fn report(name: &str, mut ms: Vec<f64>) {
    ms.sort_by(f64::total_cmp);
    println!(
        "TIMING {name}: median {:.2}ms, worst {:.2}ms",
        ms[ms.len() / 2],
        ms[ms.len() - 1]
    );
}

// 1000項目のページで1つのキーを打ったとき、changes()の各部分にかかる時間を測る。時間は確かめず、表示するだけである。
#[test]
#[ignore = "prints times: run it in release mode with --ignored --nocapture"]
fn the_parts_of_the_changes_of_a_key_on_a_long_page() {
    let shell = shell_on_the_long_page();
    let (mut event, mut built, mut found, mut written) = (vec![], vec![], vec![], vec![]);
    let (mut whole, mut all) = (vec![], vec![]);
    for n in 1..=KEYS {
        let typing = bytes_of(&Typing::Typed {
            item_id: "0000".into(),
            body: "a".repeat(n),
            cursor: u64::try_from(n).unwrap(),
        });
        event.push(timed(|| shell.typed(&typing).unwrap()).1);
        let (view, ms) = timed(|| shell.whole_view());
        built.push(ms);
        // The whole view written as bytes, as view() did before the changes (ADR 076)
        whole.push(timed(|| bytes_of(&view)).1);
        let (changes, ms) = timed(|| shell.changes_from(view));
        found.push(ms);
        written.push(timed(|| bytes_of(&changes)).1);
        assert_eq!(changes.rows.len(), 1, "one key changes one row");
        // The same steps together, as the shell calls them
        let typing = bytes_of(&Typing::Typed {
            item_id: "0000".into(),
            body: "b".repeat(n),
            cursor: u64::try_from(n).unwrap(),
        });
        shell.typed(&typing).unwrap();
        all.push(timed(|| shell.changes()).1);
    }
    report("event into the core", event);
    report("(a) the whole view model is built", built);
    report("(b) the changes are found", found);
    report("(c) the changes are written as bytes", written);
    report("(a)+(b)+(c) changes() as one call", all);
    report("the whole view model written as bytes, as before", whole);
}
