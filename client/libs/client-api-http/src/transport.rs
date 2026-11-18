//! The real transport: HTTP with `ureq`, and the server's event stream.

use crate::api::Http;
use client_core::effects::SseEvent;
use client_core::rest::{routes, ClientId, HttpReply, HttpRequest};
use std::io::BufRead;
use ureq::Agent;

/// Marks the ends of the form in an upload. It is long enough not to occur in an image.
const UPLOAD_BOUNDARY: &str = "peta-journal-emacs-upload-7f3a9c1e5b";

/// The journal server over HTTP, with `ureq`.
#[derive(Clone)]
pub struct UreqHttp {
    agent: Agent,
    server: String,
    /// Sent with every request. The server refuses a change without it (ADR-015).
    client_id: ClientId,
}

impl UreqHttp {
    pub fn new(server: String, client_id: ClientId) -> UreqHttp {
        // A status above 299 is an answer here: `api` reads the server's message from it
        let agent = Agent::config_builder()
            .http_status_as_error(false)
            .build()
            .new_agent();
        UreqHttp {
            agent,
            server,
            client_id,
        }
    }
}

impl Http for UreqHttp {
    fn send(&self, request: &HttpRequest) -> Result<HttpReply, String> {
        let url = format!("{}{}", self.server, request.path);
        let me = (ClientId::HEADER, self.client_id.as_str());
        let with_body = |to: ureq::RequestBuilder<ureq::typestate::WithBody>| {
            let to = to.header(me.0, me.1);
            match &request.body {
                Some(body) => to.send_json(body),
                None => to.send_empty(),
            }
        };
        let result = match request.method {
            "POST" => with_body(self.agent.post(&url)),
            "PUT" => with_body(self.agent.put(&url)),
            "DELETE" => self.agent.delete(&url).header(me.0, me.1).call(),
            _ => self.agent.get(&url).call(),
        };
        read_reply(result)
    }

    fn upload(&self, file_name: &str, bytes: &[u8]) -> Result<HttpReply, String> {
        let url = format!("{}{}", self.server, routes::UPLOAD_ASSET.pattern);
        let body = multipart_body(file_name, bytes);
        let result = self
            .agent
            .post(&url)
            .header(ClientId::HEADER, self.client_id.as_str())
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={UPLOAD_BOUNDARY}"),
            )
            .send(&body[..]);
        read_reply(result)
    }
}

fn read_reply(
    result: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
) -> Result<HttpReply, String> {
    let mut response = result.map_err(|e| e.to_string())?;
    let status = response.status().as_u16();
    let body = response
        .body_mut()
        .with_config()
        .limit(u64::MAX)
        .read_to_string()
        .map_err(|e| e.to_string())?;
    Ok(HttpReply { status, body })
}

/// The form with one field, `file`, as the server's asset upload reads it.
pub fn multipart_body(file_name: &str, bytes: &[u8]) -> Vec<u8> {
    // These would end the header early. The server replaces them in the saved name anyway
    let file_name = file_name.replace(['"', '\r', '\n'], "_");
    let mut body = format!(
        "--{UPLOAD_BOUNDARY}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"{file_name}\"\r\n\
         Content-Type: application/octet-stream\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{UPLOAD_BOUNDARY}--\r\n").as_bytes());
    body
}

/// Reads the server's event stream until it ends. The core decides when to connect again.
pub fn read_events(http: &UreqHttp, mut send: impl FnMut(SseEvent)) {
    let url = format!("{}{}", http.server, routes::EVENTS.pattern);
    let mut response = match http.agent.get(&url).call() {
        Ok(response) if response.status().is_success() => response,
        // No server there, or a server without an event stream
        _ => {
            send(SseEvent::Error);
            return;
        }
    };
    send(SseEvent::Open);
    let reader =
        std::io::BufReader::new(response.body_mut().with_config().limit(u64::MAX).reader());
    for line in reader.lines() {
        let Ok(line) = line else { break };
        if let Some(data) = sse_data(&line) {
            send(SseEvent::Message(data.to_string()));
        }
    }
    send(SseEvent::Error);
}

/// The data of a line of an event stream. The server sends each message as one `data:` line.
pub fn sse_data(line: &str) -> Option<&str> {
    let data = line.strip_prefix("data:")?;
    Some(data.strip_prefix(' ').unwrap_or(data))
}
