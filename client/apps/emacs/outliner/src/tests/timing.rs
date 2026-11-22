//! How long the Emacs shell takes for one key, on pages of every length (#859). These tests
//! print times and check none of them, so they are ignored: run them in release mode with
//!
//! ```text
//! cargo test --release -p outliner-emacs --lib timing -- --ignored --nocapture
//! ```
//!
//! The time is that of `Shell::handle` with the JSON lines of what it said to Emacs. The real
//! program writes those lines to stdout; the time Emacs takes to read them is not in here.

use super::shell::{item, Driver};
use crate::protocol::{write_line, Inbound, Outbound};
use std::time::{Duration, Instant};

const LENGTHS: [usize; 4] = [1, 10, 100, 1000];
const KEYS: usize = 100;

/// A page of `length` items, with the levels of the page of `TypingSpeedTests` (#824).
fn page(length: usize) -> Driver {
    let items: Vec<_> = (0..length)
        .map(|n| {
            item(
                &format!("{n:04}"),
                i32::try_from(n % 3 + 1).unwrap(),
                &format!("item {n}"),
            )
        })
        .collect();
    Driver::open(&items)
}

/// Sends the message, and gives the time with the JSON lines, and how many bytes went to Emacs.
fn time(driver: &mut Driver, message: Inbound) -> (Duration, usize) {
    driver.shell.host.out.clear();
    let start = Instant::now();
    driver.send(message);
    let bytes: usize = driver
        .sent()
        .iter()
        .map(|m: &Outbound| write_line(m).len() + 1)
        .sum();
    (start.elapsed(), bytes)
}

fn median(mut times: Vec<Duration>) -> Duration {
    times.sort();
    times[times.len() / 2]
}

fn report(what: &str, length: usize, runs: Vec<(Duration, usize)>) {
    let bytes = runs.iter().map(|r| r.1).max().unwrap();
    let ms = median(runs.into_iter().map(|r| r.0).collect()).as_secs_f64() * 1000.0;
    println!("{what:<8} {length:>5} items: {ms:>7.3}ms, up to {bytes:>7} bytes to Emacs");
}

// 文字を打つときと、行が変わるときに、Emacsのシェルがかかる時間を、項目の数ごとに表示する。
#[test]
#[ignore = "prints times; run in release mode"]
fn a_key_and_a_change_of_the_rows_on_pages_of_every_length() {
    for length in LENGTHS {
        let mut driver = page(length);
        let typed = (0..KEYS)
            .map(|n| {
                let body = format!("item 0{}", "x".repeat(n + 1));
                time(
                    &mut driver,
                    Inbound::BodyChanged {
                        id: "0000".into(),
                        body,
                    },
                )
            })
            .collect();
        report("type", length, typed);

        // The last item goes in and out under the one above it: every key changes a row
        let last = format!("{:04}", length - 1);
        let moved = (0..KEYS)
            .map(|n| {
                let message = if n % 2 == 0 {
                    Inbound::Indent { id: last.clone() }
                } else {
                    Inbound::Outdent { id: last.clone() }
                };
                let timed = time(&mut driver, message);
                driver.fail_all_waiting_calls();
                timed
            })
            .collect();
        report("indent", length, moved);
    }
}
