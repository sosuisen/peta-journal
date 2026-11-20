use std::fs;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use client_cloud_sync::{Cloud, CloudSync, SyncError, SyncListener, SyncState};

use crate::fake::Fixture;

/// Hears the sync, and keeps what it told and logged.
struct Ear {
    told: Sender<(SyncState, bool)>,
    logged: Arc<Mutex<Vec<SyncError>>>,
}

impl SyncListener for Ear {
    fn told(&self, state: SyncState, in_front: bool) {
        let _ = self.told.send((state, in_front));
    }

    fn log(&self, failure: &SyncError) {
        self.logged.lock().unwrap().push(failure.clone());
    }
}

/// A sync whose core is held in its first work until the test lets it go.
struct Held {
    fixture: Fixture,
    sync: CloudSync,
    told: Receiver<(SyncState, bool)>,
    logged: Arc<Mutex<Vec<SyncError>>>,
    /// Tells of each work of the core, as it starts.
    working: Receiver<()>,
    /// Lets the core do one work.
    go: Sender<()>,
    rounds: Arc<Mutex<u32>>,
}

/// What the core does with the files in a round, in place of the core.
type Work = Box<dyn Fn(&std::path::Path) -> Result<(), SyncError> + Send + Sync>;

fn held(work: Work) -> Held {
    held_waiting(work, Duration::from_secs(60))
}

/// A sync that starts `after_change` after the last change of a page.
fn held_waiting(work: Work, after_change: Duration) -> Held {
    let fixture = Fixture::new();
    fixture.cloud.answer(None, vec![], "c1", true);
    fixture.cloud.answer(Some("c1"), vec![], "c1", false);
    let local = client_cloud_sync::Local {
        folder: client_cloud_sync::MarkdownFolder::new(
            &fixture.root,
            fixture.dir.path().join("temp"),
        ),
        cursor: fixture.dir.path().join("cursor"),
        record: client_cloud_sync::CloudRecord::new(fixture.record_file()),
    };
    let (told_tx, told) = channel();
    let (working_tx, working) = channel();
    let (go, go_rx) = channel::<()>();
    let go_rx = Mutex::new(go_rx);
    let logged = Arc::new(Mutex::new(Vec::new()));
    let rounds = Arc::new(Mutex::new(0));
    let root = fixture.root.clone();
    let counted = Arc::clone(&rounds);
    let sync = CloudSync::new(
        local,
        true,
        after_change,
        move || {
            *counted.lock().unwrap() += 1;
            let _ = working_tx.send(());
            go_rx.lock().unwrap().recv().unwrap();
            work(&root)
        },
        Ear {
            told: told_tx,
            logged: Arc::clone(&logged),
        },
    );
    Held {
        fixture,
        sync,
        told,
        logged,
        working,
        go,
        rounds,
    }
}

impl Held {
    fn cloud(&self) -> Arc<dyn Cloud> {
        self.fixture.cloud.clone()
    }

    /// Waits for the core to start a work.
    fn working(&self) {
        self.working.recv_timeout(Duration::from_secs(2)).unwrap();
    }

    /// Lets the core do all the work that comes.
    fn let_go(&self) {
        for _ in 0..10 {
            self.go.send(()).unwrap();
        }
    }

    /// What the sync told until it rested: idle or failed.
    fn until_rest(&self) -> Vec<(SyncState, bool)> {
        let mut seen = Vec::new();
        loop {
            let told = self.told.recv_timeout(Duration::from_secs(2)).unwrap();
            seen.push(told);
            if matches!(told.0, SyncState::Idle | SyncState::Failed { .. }) {
                return seen;
            }
        }
    }

    fn rounds(&self) -> u32 {
        *self.rounds.lock().unwrap()
    }

    /// True when the sync tells nothing more for a while.
    fn stays_quiet(&self) -> bool {
        self.told.recv_timeout(Duration::from_millis(300)).is_err()
    }
}

fn nothing() -> Work {
    Box::new(|_| Ok(()))
}

// 同期の途中で頼まれた同期は、捨てられない。今の同期が終わったあとに、1回だけ始まる。
#[test]
fn a_sync_that_is_asked_for_while_one_runs_starts_once_after_it() {
    let held = held(nothing());
    held.sync.start(held.cloud(), false);
    held.working();
    held.sync.start(held.cloud(), false);
    held.sync.start(held.cloud(), false);
    held.let_go();
    held.until_rest();
    assert!(held.stays_quiet());
    assert_eq!(held.sync.state(), SyncState::Idle);
    assert_eq!(held.rounds(), 2);
}

// アプリが前に出たときの同期は、終わるまで「前の同期」である。画面は、その間ダイアログを出す。
#[test]
fn a_sync_of_the_app_that_came_to_the_front_is_in_front_until_it_rests() {
    let held = held(nothing());
    held.sync.start(held.cloud(), true);
    held.working();
    assert!(held.sync.in_front());
    held.let_go();
    assert_eq!(held.until_rest().last(), Some(&(SyncState::Idle, false)));
    assert!(!held.sync.in_front());
}

// 編集のあとの同期は、前の同期ではない。その途中でアプリが前に出ると、前の同期になる。
#[test]
fn a_quiet_sync_comes_to_the_front_when_the_app_does() {
    let held = held(nothing());
    held.sync.start(held.cloud(), false);
    held.working();
    assert!(!held.sync.in_front());
    held.sync.start(held.cloud(), true);
    assert!(held.sync.in_front());
    held.let_go();
    held.until_rest();
    assert!(!held.sync.in_front());
}

// 途中で頼まれなければ、同期は1回で終わる。
#[test]
fn a_sync_runs_once_when_no_other_is_asked_for() {
    let held = held(nothing());
    held.let_go();
    held.sync.start(held.cloud(), false);
    held.until_rest();
    assert_eq!(held.rounds(), 1);
}

// 同期は、ダウンロードとアップロードがどこまで進んだかを、画面に伝える。
#[test]
fn a_sync_tells_how_far_the_download_and_the_upload_are() {
    let held = held(Box::new(|root| {
        fs::write(root.join("docs/new.md"), "- new")?;
        Ok(())
    }));
    let entries = vec![held.fixture.cloud.file("docs/a.md", "- a")];
    held.fixture.cloud.answer(None, entries, "c1", true);
    held.let_go();
    held.sync.start(held.cloud(), false);
    let states: Vec<SyncState> = held
        .until_rest()
        .into_iter()
        .map(|(state, _)| state)
        .collect();
    assert_eq!(
        states,
        vec![
            SyncState::Downloading { done: 0, total: 0 },
            SyncState::Downloading { done: 0, total: 1 },
            SyncState::Downloading { done: 1, total: 1 },
            SyncState::Uploading { done: 0, total: 1 },
            SyncState::Uploading { done: 1, total: 1 },
            SyncState::Idle,
        ]
    );
}

// 上げるファイルがない同期は、アップロードの状態にならない。
#[test]
fn a_sync_with_nothing_to_upload_tells_of_no_upload() {
    let held = held(nothing());
    held.let_go();
    held.sync.start(held.cloud(), false);
    let seen = held.until_rest();
    assert!(!seen
        .iter()
        .any(|(state, _)| matches!(state, SyncState::Uploading { .. })));
}

// 通信ができずに同期が失敗すると、そのことを持って止まる。前の同期でもなくなる。
#[test]
fn a_sync_that_cannot_reach_the_cloud_rests_as_offline() {
    let held = held(nothing());
    let entries = vec![held.fixture.cloud.file("docs/a.md", "- a")];
    held.fixture.cloud.answer(None, entries, "c1", true);
    held.fixture.cloud.break_at(Some("docs/a.md"));
    held.sync.start(held.cloud(), true);
    assert_eq!(
        held.until_rest().last(),
        Some(&(SyncState::Failed { offline: true }, false))
    );
    assert!(!held.sync.in_front());
}

// ほかの理由で同期が失敗すると、通信の失敗とは分けて止まる。理由は、画面ではなくログに残る。
#[test]
fn a_sync_that_fails_for_another_reason_rests_apart_and_logs_the_reason() {
    let refused = SyncError::Other("the core refused the files".into());
    let failure = refused.clone();
    let held = held(Box::new(move |_| Err(failure.clone())));
    held.let_go();
    held.sync.start(held.cloud(), false);
    held.until_rest();
    assert_eq!(held.sync.state(), SyncState::Failed { offline: false });
    assert_eq!(*held.logged.lock().unwrap(), vec![refused]);
}

// 失敗したあとも、同期はまた始められる。
#[test]
fn a_sync_starts_again_after_one_failed() {
    let held = held(nothing());
    let entries = vec![held.fixture.cloud.file("docs/a.md", "- a")];
    held.fixture.cloud.answer(None, entries, "c1", true);
    held.fixture.cloud.break_at(Some("docs/a.md"));
    held.sync.start(held.cloud(), false);
    held.until_rest();
    held.fixture.cloud.break_at(None);
    held.let_go();
    held.sync.start(held.cloud(), false);
    held.until_rest();
    assert_eq!(held.sync.state(), SyncState::Idle);
    assert_eq!(
        fs::read_to_string(held.fixture.root.join("docs/a.md")).unwrap(),
        "- a"
    );
}

// サインアウトすると、サインインしていない状態になり、カーソルが消える。次のサインインのあとは、クラウドの全部を尋ねる。
#[test]
fn a_sign_out_rests_signed_out_and_forgets_the_cursor() {
    let held = held(nothing());
    held.let_go();
    held.sync.start(held.cloud(), false);
    held.until_rest();
    held.sync.sign_out();
    assert_eq!(held.sync.state(), SyncState::SignedOut);
    assert!(!held.fixture.dir.path().join("cursor").exists());
    held.sync.start(held.cloud(), false);
    held.until_rest();
    assert_eq!(held.fixture.cloud.asked_with(), vec![None, None]);
}

// サインインしていないアプリの同期は、サインインしていない状態から始まる。
#[test]
fn a_sync_of_an_app_that_is_signed_out_starts_signed_out() {
    let fixture = Fixture::new();
    let (told, _heard) = channel();
    let sync = CloudSync::new(
        fixture.local,
        false,
        Duration::from_secs(60),
        || Ok(()),
        Ear {
            told,
            logged: Arc::default(),
        },
    );
    assert_eq!(sync.state(), SyncState::SignedOut);
}

// ページが変わると、待ち時間のあとに、ユーザーが待たない同期が始まる。続けて変わったときは、最後の変更から待ち、同期は1回である。
#[test]
fn a_change_starts_one_quiet_sync_after_the_wait_from_the_last_change() {
    let held = held_waiting(nothing(), Duration::from_millis(500));
    held.let_go();
    held.sync.start(held.cloud(), false);
    held.until_rest();
    held.sync.changed();
    std::thread::sleep(Duration::from_millis(150));
    held.sync.changed();
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(held.rounds(), 1);
    assert_eq!(held.until_rest().last(), Some(&(SyncState::Idle, false)));
    assert!(held.stays_quiet());
    assert_eq!(held.rounds(), 2);
}

// まだ1回も同期していないアプリでは、ページが変わっても同期は始まらない。Dropboxの呼び出しを、まだ受けていないためである。
#[test]
fn a_change_before_the_first_sync_starts_none() {
    let held = held_waiting(nothing(), Duration::from_millis(50));
    held.let_go();
    held.sync.changed();
    assert!(held.stays_quiet());
    assert_eq!(held.rounds(), 0);
}

// サインアウトのあとは、ページが変わっても同期は始まらない。サインアウトの前に待っていた同期も始まらない。
#[test]
fn a_change_after_the_sign_out_starts_no_sync() {
    let held = held_waiting(nothing(), Duration::from_millis(100));
    held.let_go();
    held.sync.start(held.cloud(), false);
    held.until_rest();
    held.sync.changed();
    held.sync.sign_out();
    held.sync.changed();
    let told_after = held.told.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(told_after.0, SyncState::SignedOut);
    assert!(held.stays_quiet());
    assert_eq!(held.rounds(), 1);
}

// サインアウトの前に待っていた同期は、待ち時間のうちにまたサインインしても始まらない。
#[test]
fn a_change_before_the_sign_out_starts_no_sync_after_the_next_sign_in() {
    let held = held_waiting(nothing(), Duration::from_millis(200));
    held.let_go();
    held.sync.start(held.cloud(), false);
    held.until_rest();
    held.sync.changed();
    held.sync.sign_out();
    held.sync.start(held.cloud(), true);
    held.until_rest();
    assert!(held.stays_quiet());
    assert_eq!(held.rounds(), 2);
}

// 同期の途中でページが変わると、待ち時間のあと、今の同期が終わってからもう1回同期する。今の同期が読んだあとの変更も、上がる。
#[test]
fn a_change_while_a_sync_runs_brings_one_more_sync_after_it() {
    let held = held_waiting(nothing(), Duration::from_millis(50));
    held.sync.start(held.cloud(), false);
    held.working();
    held.sync.changed();
    std::thread::sleep(Duration::from_millis(200));
    held.let_go();
    held.until_rest();
    assert!(held.stays_quiet());
    assert_eq!(held.rounds(), 2);
}

// 変更のあとの同期を待つ間に、まだ保存していない入力が続くと、同期はその入力から待ち直す。IMEの変換中の文字は、確定するまで保存されないためである。
#[test]
fn typing_while_a_sync_waits_moves_the_sync_later() {
    let held = held_waiting(nothing(), Duration::from_millis(500));
    held.let_go();
    held.sync.start(held.cloud(), false);
    held.until_rest();
    held.sync.changed();
    std::thread::sleep(Duration::from_millis(300));
    held.sync.typing();
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(held.rounds(), 1);
    assert_eq!(held.until_rest().last(), Some(&(SyncState::Idle, false)));
    assert!(held.stays_quiet());
    assert_eq!(held.rounds(), 2);
}

// 待っている同期がなければ、入力が続いても同期は始まらない。入力はまだ保存されておらず、上げるものがないためである。
#[test]
fn typing_with_no_sync_waiting_starts_none() {
    let held = held_waiting(nothing(), Duration::from_millis(50));
    held.let_go();
    held.sync.start(held.cloud(), false);
    held.until_rest();
    held.sync.typing();
    assert!(held.stays_quiet());
    assert_eq!(held.rounds(), 1);
}
