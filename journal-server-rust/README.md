# journal-server-rust

The PetaJournal journal server in Rust: the same REST API, SSE stream and MCP tools as the
Java server in `../journal-server`, on the same port (8087), over the same `journal.db`,
`config.json` and markdown folder.

```
cargo build --release
./target/release/journal-server --help
cargo test
```

| Crate | What it is |
|---|---|
| `crates/hmtid` | HMTID (human-readable monotonic timestamp identifier), a port of the `hmtid` npm package |
| `crates/server` | The server: `domain/` (models, fractional index, hashtags), `application/` (page, item, journal and search use cases), `infrastructure/` (SQLite, tantivy search, markdown export/import/reconcile, folder watcher), `presentation/` (REST, SSE, MCP) |

Settings: see the "Rust server" section of `../BUILD.md`. Design and background:
`../docs/rust-rewrite-plan.md`.
