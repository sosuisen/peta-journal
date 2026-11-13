//! Asks the user a yes-or-no question for an extension, and waits for the answer of a client
//! (see ADR 009).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use crate::application::extension_runtime::Confirmer;

/// Sends a question to the clients.
pub trait ConfirmEvents: Send + Sync {
    /// Sends the question. False when no client is there to answer it.
    fn ask_user(&self, request_id: &str, message: &str) -> bool;
}

pub struct ConfirmService {
    events: Arc<dyn ConfirmEvents>,
    /// How long a question waits for its answer.
    timeout: Duration,
    waiting: Mutex<HashMap<String, mpsc::Sender<bool>>>,
    next_id: AtomicU64,
}

impl ConfirmService {
    pub fn new(events: Arc<dyn ConfirmEvents>, timeout: Duration) -> ConfirmService {
        ConfirmService {
            events,
            timeout,
            waiting: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    /// Asks the clients and waits for the first answer. Blocks the thread.
    pub fn confirm(&self, message: &str) -> bool {
        let request_id = format!("confirm-{}", self.next_id.fetch_add(1, Ordering::SeqCst));
        let (sender, receiver) = mpsc::channel();
        self.waiting
            .lock()
            .unwrap()
            .insert(request_id.clone(), sender);
        let ok = self.events.ask_user(&request_id, message)
            && receiver.recv_timeout(self.timeout).unwrap_or(false);
        self.waiting.lock().unwrap().remove(&request_id);
        ok
    }

    /// Gives the answer to the question waiting under `request_id`. False when no question
    /// waits there: it was answered already, or it timed out.
    pub fn answer(&self, request_id: &str, ok: bool) -> bool {
        match self.waiting.lock().unwrap().remove(request_id) {
            Some(sender) => sender.send(ok).is_ok(),
            None => false,
        }
    }
}

impl Confirmer for ConfirmService {
    fn confirm(&self, message: &str) -> bool {
        ConfirmService::confirm(self, message)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use super::{ConfirmEvents, ConfirmService};

    /// Hands out the questions it gets, as the event stream does to the clients.
    struct Clients {
        listening: bool,
        asked: Mutex<Vec<(String, String)>>,
    }

    impl ConfirmEvents for Clients {
        fn ask_user(&self, request_id: &str, message: &str) -> bool {
            self.asked
                .lock()
                .unwrap()
                .push((request_id.to_string(), message.to_string()));
            self.listening
        }
    }

    fn clients(listening: bool) -> Arc<Clients> {
        Arc::new(Clients {
            listening,
            asked: Mutex::new(Vec::new()),
        })
    }

    /// Answers the first question that comes, from another thread, as a client would.
    fn answer_when_asked(service: &Arc<ConfirmService>, clients: &Arc<Clients>, ok: bool) {
        let service = Arc::clone(service);
        let clients = Arc::clone(clients);
        std::thread::spawn(move || loop {
            let asked = clients.asked.lock().unwrap().first().cloned();
            if let Some((request_id, _)) = asked {
                assert!(service.answer(&request_id, ok));
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        });
    }

    // 確認は、問い合わせを送り、クライアントの答えを返す。
    #[test]
    fn confirm_asks_the_clients_and_returns_their_answer() {
        let clients = clients(true);
        let service = Arc::new(ConfirmService::new(clients.clone(), Duration::from_secs(5)));
        answer_when_asked(&service, &clients, true);

        let ok = service.confirm("Insert the template?");

        assert!(ok);
        assert_eq!(clients.asked.lock().unwrap()[0].1, "Insert the template?");
    }

    // 答えるクライアントがいなければ、待たずに「いいえ」を返す。
    #[test]
    fn confirm_says_no_at_once_when_no_client_listens() {
        let service = ConfirmService::new(clients(false), Duration::from_secs(5));
        let started = std::time::Instant::now();

        let ok = service.confirm("Insert the template?");

        assert!(!ok);
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    // 時間内に答えがなければ、「いいえ」を返す。
    #[test]
    fn confirm_says_no_when_no_answer_comes_in_time() {
        let service = ConfirmService::new(clients(true), Duration::from_millis(50));

        let ok = service.confirm("Insert the template?");

        assert!(!ok);
    }

    // 待っていない問い合わせへの答えは、受け付けない。時間切れのあとの答えも同じ。
    #[test]
    fn an_answer_to_a_question_that_no_one_waits_for_is_refused() {
        let clients = clients(true);
        let service = ConfirmService::new(clients.clone(), Duration::from_millis(50));
        service.confirm("Insert the template?");
        let late_id = clients.asked.lock().unwrap()[0].0.clone();

        assert!(!service.answer(&late_id, true));
        assert!(!service.answer("unknown", true));
    }
}
