# PetaJournal Build Guide

PetaJournal is two programs: the journal server (`journal-server-rust`) and the frontend
(`frontend-rust`). Both are written in Rust. Node.js 20+ is needed only for the end-to-end tests.

## Rust server (journal-server-rust)

`journal-server-rust` is the journal server (axum, rusqlite, tantivy, rmcp). It serves the
REST API, the SSE stream and the MCP tools on port 8087, and keeps `journal.db`, `config.json`
and the markdown folder under `~/.peta-journal`. It replaced the Java (Quarkus) server; the
port is described in `docs/rust-rewrite-plan.md`.

### Prerequisites

- Rust 1.85 or newer (`rustup`), with a C compiler for the bundled SQLite.
- Network access during the first build: the Japanese dictionary (IPADIC) for the search
  index is downloaded by the `lindera-ipadic` crate and compiled into the binary. Set
  `LINDERA_BUILD_DICTIONARY_CACHE_DIR` to a directory to keep the built dictionary between
  clean builds.

### Build and run

```powershell
cd peta-journal/journal-server-rust
cargo build --release
./target/release/journal-server            # port 8087, data under ~/.peta-journal
```

Every setting has a flag and an environment variable (the first column is the name the Java
server used in `application.properties`, kept for people with old notes):

| Former Java property | Flag | Environment variable |
|---|---|---|
| `quarkus.http.port` | `--port` | `PETAJOURNAL_PORT` |
| `petajournal.data.dir` | `--data-dir` | `PETAJOURNAL_DATA_DIR` |
| `petajournal.data.db` | `--db` | `PETAJOURNAL_DATA_DB` |
| `petajournal.index.dir` | `--index-dir` | `PETAJOURNAL_INDEX_DIR` |
| `petajournal.config.dir` | `--config-dir` | `PETAJOURNAL_CONFIG_DIR` |
| `petajournal.export.interval` | `--export-interval` | `PETAJOURNAL_EXPORT_INTERVAL` |
| `petajournal.import.enabled` | `--import-enabled` | `PETAJOURNAL_IMPORT_ENABLED` |
| `petajournal.import.debounce-ms` | `--import-debounce-ms` | `PETAJOURNAL_IMPORT_DEBOUNCE_MS` |
| `petajournal.import.scan-interval` | `--import-scan-interval` | `PETAJOURNAL_IMPORT_SCAN_INTERVAL` |
| `petajournal.import.delete-prompt-cooldown` | `--import-delete-prompt-cooldown` | `PETAJOURNAL_IMPORT_DELETE_PROMPT_COOLDOWN` |
| `quarkus.http.cors.origins` | `--cors-origins` | `PETAJOURNAL_CORS_ORIGINS` |

`RUST_LOG` controls logging (default `info,tantivy=warn`), e.g. `RUST_LOG=journal_server=debug`.

The search index format is not the same as the Java server's Lucene index. On the first start
the server replaces a Lucene index in `~/.peta-journal/index` with a tantivy index and rebuilds
it from the database; the database itself is not changed.

### Tests

```powershell
cd peta-journal/journal-server-rust
cargo test
```

`cargo test` runs the unit tests and the API tests (`crates/server/tests/api.rs`). The API
tests start the server once on a free port with everything under a temporary directory.

## Rust frontend (frontend-rust)

`frontend-rust` is the frontend: a [Crux](https://github.com/redbadger/crux) core
(`crates/core`, the outline logic, tested without a browser) and a [Leptos](https://leptos.dev/) shell
(`crates/shell-leptos`) built to WebAssembly with trunk. It replaced the Angular frontend; the
port is described in `docs/frontend-rust-rewrite-plan.md`.

### Prerequisites

- Rust 1.85 or newer with the `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- trunk: `cargo install trunk --locked`

### Build and run

```powershell
cd peta-journal/frontend-rust/crates/shell-leptos
trunk serve                 # http://localhost:4200, proxies /api and /assets to the server on 8087
trunk build --release       # production files in dist/
```

`vendor/highlight.min.js` is highlight.js bundled for the browser. To move to a new highlight.js
version, bundle it in a scratch directory and copy the two files over:

```powershell
mkdir hljs-build; cd hljs-build
npm i highlight.js@11 esbuild
npx esbuild node_modules/highlight.js/lib/common.js --bundle --format=iife --global-name=hljs --minify --outfile=<peta-journal>/frontend-rust/crates/shell-leptos/vendor/highlight.min.js
cp node_modules/highlight.js/styles/github.css <peta-journal>/frontend-rust/crates/shell-leptos/vendor/highlight-github.css
```

### Tests

```powershell
cd peta-journal/frontend-rust
cargo test                  # core (the ported service specs), fm-ast and fm-leptos
```

The Playwright suite in `e2e/` drives the Rust server and the Rust frontend (trunk serves it on
port 4300 with `Trunk.e2e.toml`). Build the server first with `cargo build --release` in
`journal-server-rust`, and install trunk with `cargo install trunk`:

```bash
cd peta-journal/e2e
npm install
npx playwright install chromium   # once
npm run e2e
```

## Development Workflow

1. Start the journal server: `cd journal-server-rust && cargo run`
2. Start the frontend: `cd frontend-rust/crates/shell-leptos && trunk serve`
3. Open `http://localhost:4200`

