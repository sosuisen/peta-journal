//! Asks the user a yes-or-no question for an extension, and waits for the answer of a client
//! (see ADR 009).

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
}
