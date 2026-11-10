# PetaJournal Build Guide

PetaJournal is two programs: the journal server (`journal-server-rust`) and the frontend
(`frontend-rust`). Both are written in Rust. Node.js 20+ is needed only for the end-to-end tests.

## Rust server (journal-server-rust)

`journal-server-rust` is the journal server (axum, rusqlite, tantivy, rmcp). It serves the
REST API, the SSE stream, the MCP tools and the web frontend on port 8087, and keeps
`journal.db`, `config.json` and the markdown folder under `~/.peta-journal`. It replaced the Java (Quarkus) server; the
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
| (none) | `--frontend-dir` | `PETAJOURNAL_FRONTEND_DIR` |

The server also serves the web frontend, so the app opens at `http://localhost:8087`
(see `docs/adr/adr-010-journal-server-serves-the-web-frontend.md`). It takes the files from the
first of these folders:

1. the folder given with `--frontend-dir` or `PETAJOURNAL_FRONTEND_DIR`
2. the `frontend/` folder beside the server executable

Under `cargo run`, `.cargo/config.toml` sets `PETAJOURNAL_FRONTEND_DIR` to
`frontend-rust/crates/shell-leptos/dist`, the folder `trunk build` writes. The executable started
by hand (`./target/release/journal-server`) does not see that setting: pass `--frontend-dir`, or
copy `dist/` to `target/release/frontend/`. With neither folder the server serves the API only,
and logs `Frontend: not served` at startup.

The server reads the files from disk on every request. After `trunk build`, reload the browser;
the server need not restart.

`RUST_LOG` controls logging (default `info,tantivy=warn`), e.g. `RUST_LOG=journal_server=debug`.

The search index format is not the same as the Java server's Lucene index. On the first start
the server replaces a Lucene index in `~/.peta-journal/index` with a tantivy index and rebuilds
it from the database; the database itself is not changed.

### Tests

```powershell
cd peta-journal/journal-server-rust
cargo test
```

`cargo test` runs the unit tests and two test binaries. `crates/server/tests/it/` holds the API
tests and the other integration tests. `crates/server/tests/mcp/` holds the MCP tests. Each
binary starts the server once on a free port with everything under a temporary directory.

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
trunk build                 # debug files in dist/, for development
trunk build --release       # small files in dist/, for daily use and distribution
```

The journal server serves `dist/` at `http://localhost:8087`; see "Build and run" of the Rust
server above. `trunk serve` is not used: `Trunk.toml` has no proxy to the journal server, so its
page cannot reach the API.

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
cargo test                  # core (the ported service specs) and fm-leptos
```

The Playwright suite in `e2e/` drives the Rust server and the Rust frontend the way they run for
the user: it builds the server with `cargo build --release` and the frontend with `trunk build`
into `dist-e2e/`, then starts one journal server on port 8187 that serves those files and the
API. Install trunk once with `cargo install trunk`:

```bash
cd peta-journal/e2e
npm install
npx playwright install chromium   # once
npm run e2e
```

The test server runs with its own `--data-dir`, `--index-dir` and `--config-dir` under a temporary
directory, so its markdown export lands there and never in the markdown folder of
`~/.peta-journal/config.json`. A global setup checks this before the first test and refuses to
run otherwise. When starting a second server by hand for an experiment, always pass
`--config-dir` as well, or it will export into your real markdown folder.

### Coverage

Setup, once:

```
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
```

After a change, read the coverage for the lines no test reaches (the numbers are not a target):

```
cd peta-journal/frontend-rust
cargo cov            # a table per file
cargo cov --html     # target/llvm-cov/html/index.html, line by line
```

`cargo cov` is an alias in `.cargo/config.toml`. It measures `peta-core`, `fm-leptos` and
`peta-shell-emacs`. The wasm shell `peta-frontend` (`crates/shell-leptos`) is left out: it only runs in
the browser, so the E2E tests cover it instead.

No file of `peta-shell-emacs` is left out. Its tests start the program itself, and `cargo cov`
measures that run too.

### Emacs frontend

`crates/shell-emacs` is a second shell around the same core. It is a native program, and an Emacs
mode draws what it sends. `crates/shell-emacs/README.md` describes the setup, the keys and the tests.

```powershell
cd peta-journal/frontend-rust
cargo build --release -p peta-shell-emacs
```

## Atlas crate (atlas-rust)

`atlas-rust` is the crate `peta-atlas`. The server and the frontend both use it, and so can
another program that shows an atlas, such as a static site. It reads the outline of an
`index_atlas` page, draws the marks of the atlas as SVG, and writes the scene that
`atlas.js` draws. Its features choose what is built:

- no feature: reading the outline only, as the server does
- `marks`: the SVG marks and the scene, as the frontend uses them
- `measure`: the width of a card from its label, for a card whose label is hidden. It
  measures in the font of the labels, `frontend-rust/assets/NotoSansJP-Regular.ttf`.

```powershell
cd peta-journal/atlas-rust
cargo test --all-features
```

## CI

`.github/workflows/ci.yml` runs on every pull request and every push to `main`. A change to the
documents alone does not start it. On Ubuntu, each of the three workspaces runs `cargo fmt --check`,
`cargo clippy` and `cargo test`. The frontend also checks the shell for wasm. A fourth job checks
`ext-web-app/todo` on Node 24: the format, lint, the tests and the build. The end-to-end tests
in `e2e/` do not run there, so run them by hand. A warning from clippy does not fail the run.

## Development Workflow

1. Build the frontend: `cd frontend-rust/crates/shell-leptos && trunk build`
2. Start the journal server: `cd journal-server-rust && cargo run`
3. Open `http://localhost:8087`

After a change to the frontend, run `trunk build` again and reload the browser. The journal
server keeps running.

