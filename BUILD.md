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
`frontend-rust/crates/client-shell-leptos/dist`, the folder `trunk build` writes. The executable started
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
(`crates/client-core`, the outline logic, tested without a browser) and a [Leptos](https://leptos.dev/) shell
(`crates/client-shell-leptos`) built to WebAssembly with trunk. It replaced the Angular frontend; the
port is described in `docs/frontend-rust-rewrite-plan.md`.

### Prerequisites

- Rust 1.85 or newer with the `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- trunk: `cargo install trunk --locked`

### Build and run

```powershell
cd peta-journal/frontend-rust/crates/client-shell-leptos
trunk build                 # debug files in dist/, for development
trunk build --release       # small files in dist/, for daily use and distribution
```

A script at the top of the repository does the same from any folder:

```powershell
./build-frontend.ps1        # trunk build --release (on macOS and Linux: ./build-frontend.sh)
./build-frontend.ps1 -Dev   # trunk build (on macOS and Linux: ./build-frontend.sh --dev)
```

The journal server serves `dist/` at `http://localhost:8087`; see "Build and run" of the Rust
server above. `trunk serve` is not used: `Trunk.toml` has no proxy to the journal server, so its
page cannot reach the API.

`vendor/highlight.min.js` is highlight.js bundled for the browser. To move to a new highlight.js
version, bundle it in a scratch directory and copy the two files over:

```powershell
mkdir hljs-build; cd hljs-build
npm i highlight.js@11 esbuild
npx esbuild node_modules/highlight.js/lib/common.js --bundle --format=iife --global-name=hljs --minify --outfile=<peta-journal>/frontend-rust/crates/client-shell-leptos/vendor/highlight.min.js
cp node_modules/highlight.js/styles/github.css <peta-journal>/frontend-rust/crates/client-shell-leptos/vendor/highlight-github.css
```

### Tests

```powershell
cd peta-journal/frontend-rust
cargo test                  # core (the ported service specs) and the shells
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

`cargo cov` is an alias in `.cargo/config.toml`. It measures `client-core`, `client-page-links`,
`client-api-http`, `client-api-embedded`, `client-shell-emacs`, `client-native-bridge`, `client-shell-http` and
`client-shell-embedded`. The wasm shell `client-shell-leptos` (`crates/client-shell-leptos`) is left out: it only runs in
the browser, so the E2E tests cover it instead.

No file of `client-shell-emacs` is left out. Its tests start the program itself, and `cargo cov`
measures that run too.

`cargo cov` turns on the feature `facet_typegen` of `client-native-bridge`, so the generation of
the types of the native shells is measured too.

### Types of the native shells

`crates/client-native-bridge` holds the core for a shell that is not written in Rust, such as the SwiftUI
shell (ADR 024). The types that cross to such a shell are generated from the Rust types:

```
cd peta-journal/frontend-rust
cargo run -p client-native-bridge --features facet_typegen --bin codegen -- swift <folder>
cargo run -p client-native-bridge --features facet_typegen --bin codegen -- csharp <folder>
cargo run -p client-native-bridge --features facet_typegen --bin codegen -- kotlin <folder>
```

The first writes the Swift package `PetaTypes` under `<folder>/swift`, the second the C#
project `PetaTypes` under `<folder>/csharp`, and the third the Kotlin package
`net.sosuisen.petajournal.types` under `<folder>/kotlin`. The generated code is not kept in the
repository.

The C# and the Kotlin types are not written by the generator that Crux brings (facet_generate
0.19): its C# and its Kotlin do not compile for these types. `src/codegen.rs` calls a later version of the same generator
directly (ADR 028). The tests of the generation need the feature too:
`cargo test -p client-native-bridge --features facet_typegen`.

This crate has no way to the journal, and no entrance for another language. A crate above it
has both (ADR 036). `crates/client-shell-http` is the one for the shells that ask a journal
server: the SwiftUI and the WinUI shell. Its feature `ffi` adds the entrance (`src/ffi.rs`),
made with BoltFFI. `frontend-swiftui/README.md` describes how the SwiftUI shell is built from both, and the
tools it needs. `frontend-winui/README.md` does so for the WinUI shell.

A type that crosses has `#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]`. The
generator reads the shape of the Rust type, not what serde writes. A type with a hand-written
`Serialize` has to write that same shape in bincode, as `PropertyValue` and `ItemPosition` do.

### Core of the Android and iOS shells

`crates/client-shell-embedded` holds the core for the Android and the iOS shell. It has no journal server to ask:
the journal core (`journal-server-rust/crates/core`) runs inside the app (ADR 029). The shell
puts Markdown files into a folder of the app and calls `sync()`, which takes them in and makes
them found by the search. `sync()` also deletes the pages of the files that have gone, without
asking (ADR 030). Then it writes the pages that changed in the app to the folder, for the
shell to upload (ADR 031). `open` shows a page that is there, and `open_or_make` also makes
one that is not. Nothing runs at times of its own: no watcher, no scan, no export.

BoltFFI packs the crate for Android, and the generator writes the Kotlin types beside it:

```
cd peta-journal/frontend-rust
cargo run -p client-native-bridge --features facet_typegen --bin codegen -- kotlin ../frontend-android/generated
cd crates/client-shell-embedded
boltffi pack android --release
```

This needs the Android NDK (`ANDROID_NDK_HOME`) and the Rust targets `aarch64-linux-android` and
`x86_64-linux-android`. The output is `frontend-android/generated`: the libraries under
`jniLibs`, and the Kotlin under `kotlin`. It is not kept in the repository.
`frontend-android/generate.ps1` runs both steps.

The tests of the crate run on the development machine, with `cargo test -p client-shell-embedded`.

The iOS shell uses the same crate (ADR 034). On a Mac, `frontend-ios/generate.sh` writes the
Swift types and packs the crate with `boltffi pack apple --release`, for a device, for the
simulator and for the Mac. This needs the Rust targets `aarch64-apple-ios` and
`aarch64-apple-ios-sim`. The output is `frontend-ios/generated`.

### Emacs frontend

`crates/client-shell-emacs` is a second shell around the same core. It is a native program, and an Emacs
mode draws what it sends. `crates/client-shell-emacs/README.md` describes the setup, the keys and the tests.

```powershell
cd peta-journal/frontend-rust
cargo build --release -p client-shell-emacs
```

## SwiftUI frontend (frontend-swiftui)

The shell for macOS, a Walking Skeleton so far: it opens one page and saves what is typed into an
item. `frontend-swiftui/README.md` describes the tools, the build of the app and the tests. Its
tests do not run in CI.

## WinUI frontend (frontend-winui)

The shell for Windows. It is the same app as the SwiftUI shell: stickies that each show one page
(ADR 028). `frontend-winui/README.md` describes the tools, the build of the app and the tests.
Its tests do not run in CI.

## Android frontend (frontend-android)

The shell for Android, in Kotlin with Jetpack Compose. It downloads the Markdown folder from
Dropbox, and a page is searched for and opened. Nothing is changed. The journal core runs
inside the app (ADR 029, ADR 030). `frontend-android/README.md` describes the tools, the build of the app and the
tests. Its tests need a device or an emulator, and do not run in CI.

## iOS frontend (frontend-ios)

The shell for iOS, in Swift with SwiftUI. It does what the Android shell does, with the same
core inside the app (ADR 034). `frontend-ios/README.md` describes the tools, the build of the
app and the tests. Its tests need a Mac, and those of the screen a simulator. They do not run
in CI.

## Atlas (atlas)

`atlas` holds the two sides of an atlas: the crate `peta-atlas` in `src/`, which writes the
scene, and the scripts in `web/`, which draw it. The scene is the JSON that the two sides
agree on. A change to it changes both sides in one pull request (ADR 038).

### The crate

The server and the frontend both use the crate, and so can
another program that shows an atlas, such as a static site. It reads the outline of an
`index_atlas` page, draws the marks of the atlas as SVG, and writes the scene that
`atlas.js` draws. Its features choose what is built:

- no feature: reading the outline only, as the server does
- `marks`: the SVG marks and the scene, as the frontend uses them
- `measure`: the width of a card from its label, for a card whose label is hidden. It
  measures in the font of the labels, `frontend-rust/assets/NotoSansJP-Regular.ttf`.

```powershell
cd peta-journal/atlas
cargo test --all-features
```

### The scripts

`web/` holds what a page needs to draw an atlas. The crate does not read these files.

- `atlas.js`: draws the scene. It sets the global `AtlasCanvas`.
- `pixi.min.js`: PixiJS, the global `PIXI`. `atlas.js` needs it.
- `pixi-ui.js`: @pixi/ui, the global `PIXI.ui`. Only the menus and the dialogs of an atlas
  that can be edited need it.
- `tweedle.umd.js`: tweedle.js, the global `tweedle_js`. `pixi-ui.js` needs it.

The web shell loads all four from `index.html` of `frontend-rust/crates/client-shell-leptos`.
A page that only shows an atlas, such as a static site, needs `pixi.min.js` and `atlas.js`.
Load `pixi.min.js` first.

The three libraries are the files that their npm packages ship in `dist/`. They are a set:
@pixi/ui 2.4.1 works with PixiJS 8 and tweedle.js 2.1.0. To move to new versions, install
them in a scratch directory and copy the three files over:

```powershell
mkdir pixi-build; cd pixi-build
npm i pixi.js@8 @pixi/ui@2 tweedle.js@2
cp node_modules/pixi.js/dist/pixi.min.js <peta-journal>/atlas/web/pixi.min.js
cp node_modules/@pixi/ui/dist/pixi-ui.js <peta-journal>/atlas/web/pixi-ui.js
cp node_modules/tweedle.js/dist/tweedle.umd.js <peta-journal>/atlas/web/tweedle.umd.js
```

Then build the frontend, open an atlas, and check the drawing and a dialog.

## CI

`.github/workflows/ci.yml` runs on every pull request and every push to `main`. A change to the
documents alone does not start it. On Ubuntu, each of the three workspaces runs `cargo fmt --check`,
`cargo clippy` and `cargo test`. The frontend also checks the shell for wasm. A fourth job checks
`ext-web-app/todo` on Node 24: the format, lint, the tests and the build. The end-to-end tests
in `e2e/` do not run there, so run them by hand. A warning from clippy does not fail the run.

## Development Workflow

1. Build the frontend: `./build-frontend.ps1 -Dev` (or `./build-frontend.sh --dev`)
2. Start the journal server: `cd journal-server-rust && cargo run`
3. Open `http://localhost:8087`

After a change to the frontend, run the script again and reload the browser. The journal
server keeps running.

