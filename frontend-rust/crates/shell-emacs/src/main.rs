//! The process `peta-journal.el` starts. It takes the address of the journal server.

/// The server the web frontend talks to, unless Emacs says otherwise.
const DEFAULT_SERVER: &str = "http://localhost:8087";

fn main() {
    let server = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_SERVER.to_string());
    peta_shell_emacs::runtime::run(server);
}
