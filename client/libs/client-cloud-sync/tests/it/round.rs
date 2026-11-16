use std::cell::{Cell, RefCell};
use std::fs;
use std::sync::Mutex;

use client_cloud_sync::{SyncError, SyncRound};

use crate::fake::Fixture;

const NOTES: &str = "docs/notes.md";

/// A sync whose cloud has the notes, and nothing more.
fn fixture() -> Fixture {
    let sync = Fixture::new();
    sync.cloud
        .answer(None, vec![sync.cloud.file(NOTES, "- a")], "c1", true);
    sync
}

fn here(sync: &Fixture) -> String {
    fs::read_to_string(sync.root.join(NOTES)).unwrap()
}

fn write(sync: &Fixture, text: &str) {
    fs::write(sync.root.join(NOTES), text).unwrap();
}

/// The desk changes the file in the cloud, and the cloud tells of it after `since`.
fn desk_writes(sync: &Fixture, text: &str, since: &str, next: &str) {
    sync.cloud
        .answer(Some(since), vec![sync.cloud.file(NOTES, text)], next, false);
}

fn run(sync: &Fixture, take_in: &dyn Fn() -> Result<(), SyncError>) -> Result<(), SyncError> {
    SyncRound::new(sync.cloud.as_ref(), &sync.local, take_in).run(&|_, _| {}, &|_, _| {})
}

// 1回の同期は、ダウンロード、コアの取り込みと書き出し、アップロードの順に進む。
#[test]
fn a_round_downloads_then_lets_the_core_work_then_uploads() {
    let sync = fixture();
    run(&sync, &|| {
        write(&sync, &format!("{} and what the phone wrote", here(&sync)));
        Ok(())
    })
    .unwrap();
    assert_eq!(
        sync.cloud.text(NOTES).as_deref(),
        Some("- a and what the phone wrote")
    );
}

// アップロードの前にクラウドのファイルが変わったら、ダウンロードからやり直す。クラウドの変更は消えない。
#[test]
fn a_round_starts_again_when_the_cloud_changed_before_the_upload() {
    let sync = fixture();
    let rounds = Cell::new(0);
    run(&sync, &|| {
        rounds.set(rounds.get() + 1);
        if rounds.get() == 1 {
            desk_writes(&sync, "- desk", "c1", "c2");
        }
        write(&sync, &format!("{} + phone", here(&sync)));
        Ok(())
    })
    .unwrap();
    assert_eq!(sync.cloud.asked_with(), vec![None, Some("c1".to_string())]);
    assert_eq!(sync.cloud.text(NOTES).as_deref(), Some("- desk + phone"));
}

// クラウドが3回続けて断ったら、やめてエラーを出す。クラウドのファイルは上書きしない。
#[test]
fn a_round_gives_up_after_the_cloud_refused_three_times() {
    let sync = fixture();
    let rounds = Cell::new(0);
    let done = run(&sync, &|| {
        let round = rounds.get() + 1;
        rounds.set(round);
        desk_writes(
            &sync,
            &format!("- desk {round}"),
            &format!("c{round}"),
            &format!("c{}", round + 1),
        );
        write(&sync, "- phone");
        Ok(())
    });
    assert_eq!(done, Err(SyncError::ChangedInCloud(NOTES.into())));
    assert_eq!(rounds.get(), 3);
    assert_eq!(sync.cloud.text(NOTES).as_deref(), Some("- desk 3"));
}

// コアの取り込みが失敗したら、やり直さずにそのエラーを出す。何も上げない。
#[test]
fn a_round_stops_with_the_failure_of_the_core() {
    let sync = fixture();
    let refused = SyncError::Other("the core refused the files".into());
    let rounds = Cell::new(0);
    let done = run(&sync, &|| {
        rounds.set(rounds.get() + 1);
        write(&sync, "- phone");
        Err(refused.clone())
    });
    assert_eq!(done, Err(refused));
    assert_eq!(rounds.get(), 1);
    assert_eq!(sync.cloud.uploaded(), Vec::<String>::new());
}

// 進み具合は、ダウンロードとアップロードで別々に伝わる。
#[test]
fn the_progress_of_the_download_and_of_the_upload_are_told_apart() {
    let sync = fixture();
    let take_in = || {
        write(&sync, "- phone");
        Ok(())
    };
    let downloads = Mutex::new(Vec::new());
    let told = RefCell::new(Vec::new());
    SyncRound::new(sync.cloud.as_ref(), &sync.local, &take_in)
        .run(
            &|done, total| {
                downloads
                    .lock()
                    .unwrap()
                    .push(format!("down {done}/{total}"));
            },
            &|done, total| {
                told.borrow_mut()
                    .extend(downloads.lock().unwrap().drain(..));
                told.borrow_mut().push(format!("up {done}/{total}"));
            },
        )
        .unwrap();
    assert_eq!(
        told.into_inner(),
        vec!["down 0/1", "down 1/1", "up 0/1", "up 1/1"]
    );
}
