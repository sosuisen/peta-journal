//! Opens the "files disappeared" confirmation page in the user's browser.
//! Nothing is deleted from the DB until the user ticks a page there and submits.

use std::sync::atomic::{AtomicI64, Ordering};

use journal_core::infrastructure::export::reconcile_service::MissingFilesPrompt;
use journal_core::infrastructure::persistence::db::now_millis;

use crate::infrastructure::launcher;

#[derive(Debug)]
pub struct PendingDeletePrompt {
    http_port: u16,
    cooldown_seconds: i64,
    last_opened_at: AtomicI64,
}

impl PendingDeletePrompt {
    pub fn new(http_port: u16, cooldown_seconds: u64) -> PendingDeletePrompt {
        PendingDeletePrompt {
            http_port,
            cooldown_seconds: cooldown_seconds as i64,
            last_opened_at: AtomicI64::new(0),
        }
    }

    pub fn url(&self) -> String {
        format!(
            "http://localhost:{}/api/sync/pending-deletes",
            self.http_port
        )
    }
}

impl MissingFilesPrompt for PendingDeletePrompt {
    /// Opens the browser unless it was opened within the cooldown.
    fn notify_new_missing(&self) {
        let now = now_millis();
        let last = self.last_opened_at.load(Ordering::SeqCst);
        if now - last < self.cooldown_seconds * 1000 {
            tracing::debug!("Pending-deletes page opened recently; not opening again yet");
            return;
        }
        self.last_opened_at.store(now, Ordering::SeqCst);
        let url = self.url();
        if !launcher::launch(&url) {
            tracing::warn!("Could not open a browser. Review missing files at {url}");
        }
    }
}
