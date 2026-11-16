use std::fs;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;

use crate::cloud::{Cloud, SyncError};
use crate::round::SyncRound;
use crate::Local;

/// Where the sync with the cloud stands, for the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncState {
    /// The user has not signed in to the cloud yet.
    SignedOut,
    /// Nothing runs.
    Idle,
    Downloading {
        done: u32,
        total: u32,
    },
    Uploading {
        done: u32,
        total: u32,
    },
    /// The last sync failed. What it had downloaded and uploaded stays. `offline` says that
    /// the cloud could not be reached. The screen tells only that: the words of the SDK are
    /// long, and mean nothing to the user. They go to the log.
    Failed {
        offline: bool,
    },
}

impl SyncState {
    fn is_running(self) -> bool {
        matches!(
            self,
            SyncState::Downloading { .. } | SyncState::Uploading { .. }
        )
    }
}

/// What the shell gives the sync, to hear from it.
pub trait SyncListener: Send + Sync {
    /// The sync stands at `state` now. `in_front` says that the user waits for the sync that
    /// runs: see [`CloudSync::in_front`].
    ///
    /// It is called on a thread of the sync, one call at a time, in order. It must not call
    /// [`CloudSync`]: it hands the state to the main thread of the shell and returns.
    fn told(&self, state: SyncState, in_front: bool);

    /// Keeps why a sync failed, for the one who looks into it.
    fn log(&self, failure: &SyncError);
}

/// Runs the syncs with the cloud, one at a time, and tells the shell where they stand. A sync
/// runs on a thread of its own.
pub struct CloudSync {
    shared: Arc<Shared>,
}

struct Shared {
    local: Local,
    take_in: Box<dyn Fn() -> Result<(), SyncError> + Send + Sync>,
    listener: Box<dyn SyncListener>,
    status: Mutex<Status>,
}

struct Status {
    state: SyncState,
    in_front: bool,
    /// The cloud of a sync that was asked for while another ran.
    next: Option<Arc<dyn Cloud>>,
}

impl CloudSync {
    /// `take_in` lets the core take in the files of the Markdown folder and write the pages
    /// that changed in the app. It returns when the core is done.
    pub fn new(
        local: Local,
        signed_in: bool,
        take_in: impl Fn() -> Result<(), SyncError> + Send + Sync + 'static,
        listener: impl SyncListener + 'static,
    ) -> Self {
        let state = if signed_in {
            SyncState::Idle
        } else {
            SyncState::SignedOut
        };
        CloudSync {
            shared: Arc::new(Shared {
                local,
                take_in: Box::new(take_in),
                listener: Box::new(listener),
                status: Mutex::new(Status {
                    state,
                    in_front: false,
                    next: None,
                }),
            }),
        }
    }

    pub fn state(&self) -> SyncState {
        self.shared.lock().state
    }

    /// Whether the sync that runs is one the user waits for: the app came to the front, and
    /// the pages may be old until the sync rests. The screen then shows where it stands in a
    /// dialog. A sync after an edit is not in front: the user goes on typing.
    pub fn in_front(&self) -> bool {
        self.shared.lock().in_front
    }

    /// Starts a sync with the cloud. While one runs, another starts after it, once: what was
    /// typed since the running one read the pages is not left behind. A sync that runs comes
    /// to the front when `in_front` asks for it.
    pub fn start(&self, cloud: Arc<dyn Cloud>, in_front: bool) {
        let mut status = self.shared.lock();
        if in_front {
            status.in_front = true;
        }
        if status.state.is_running() {
            status.next = Some(cloud);
            self.shared.tell(&status);
            return;
        }
        status.next = None;
        status.state = SyncState::Downloading { done: 0, total: 0 };
        self.shared.tell(&status);
        drop(status);
        let shared = Arc::clone(&self.shared);
        thread::Builder::new()
            .name("cloud-sync".into())
            .spawn(move || shared.run(cloud))
            .expect("a thread for the sync");
    }

    /// The user has signed out of the cloud. What was downloaded stays, and so does the record
    /// of the versions: it tells a copy from what was written here and is not uploaded yet.
    ///
    /// The cursor goes. The next sign-in may be to another account, where it means nothing.
    /// Without it, the next download lists everything: it takes only the files that differ,
    /// and deletes the copies of what the account does not hold.
    ///
    /// It is not for the time a sync runs: the screen offers no sign-out then.
    pub fn sign_out(&self) {
        let mut status = self.shared.lock();
        assert!(!status.state.is_running(), "a sync runs");
        let _ = fs::remove_file(&self.shared.local.cursor);
        status.state = SyncState::SignedOut;
        self.shared.tell(&status);
    }
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, Status> {
        self.status.lock().unwrap()
    }

    fn tell(&self, status: &Status) {
        self.listener.told(status.state, status.in_front);
    }

    fn set(&self, state: SyncState) {
        let mut status = self.lock();
        status.state = state;
        self.tell(&status);
    }

    /// Runs syncs until none waits.
    fn run(&self, mut cloud: Arc<dyn Cloud>) {
        loop {
            let done = SyncRound::new(cloud.as_ref(), &self.local, &*self.take_in).run(
                &|done, total| self.set(SyncState::Downloading { done, total }),
                // Most syncs have nothing to upload, and say nothing of it
                &|done, total| {
                    if total > 0 {
                        self.set(SyncState::Uploading { done, total });
                    }
                },
            );
            let mut status = self.lock();
            match done {
                Ok(()) => match status.next.take() {
                    // The sync that waited goes on in front, when this one was
                    Some(waits) => {
                        status.state = SyncState::Downloading { done: 0, total: 0 };
                        self.tell(&status);
                        cloud = waits;
                    }
                    None => {
                        status.state = SyncState::Idle;
                        status.in_front = false;
                        self.tell(&status);
                        return;
                    }
                },
                Err(failure) => {
                    self.listener.log(&failure);
                    status.next = None;
                    status.in_front = false;
                    status.state = SyncState::Failed {
                        offline: matches!(failure, SyncError::Offline(_)),
                    };
                    self.tell(&status);
                    return;
                }
            }
        }
    }
}
