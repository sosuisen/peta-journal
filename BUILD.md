# PetaJournal Build Guide

PetaJournal is two programs: the journal server (`journal/`) and the frontend
(`client/`). Both are written in Rust. Node.js 20+ is needed only for the end-to-end tests.

## Rust server (journal)

`journal/` holds the journal server (axum, rusqlite, tantivy, rmcp). It serves the
REST API, the SSE stream, the MCP tools and the web frontend on port 8087, and keeps
`journal.db`, `config.json` and the markdown folder under `~/.peta-journal`. It replaced the Java (Quarkus) server.

### Prerequisites

- `rustup`, with a C compiler for the bundled SQLite. `rust-toolchain.toml` sets the version of
  Rust, and rustup installs it on the first build.
- Network access during the first build: the Japanese dictionary (IPADIC) for the search
  index is downloaded by the `lindera-ipadic` crate and compiled into the binary. Set
  `LINDERA_BUILD_DICTIONARY_CACHE_DIR` to a directory to keep the built dictionary between
  clean builds.

### Build and run

```powershell
cd peta-journal/journal
cargo build --release
./target/release/journal-server            # port 8087, data under ~/.peta-journal
```

Two scripts in `journal/journal-server` do the same from any folder. Each has an `.sh` twin for
macOS, Linux and Git Bash:

```powershell
./journal/journal-server/debug.ps1      # cargo run: builds the debug build and starts it
./journal/journal-server/release.ps1    # cargo build --release
```

Every app under `client/apps` has the same two scripts in its folder: `debug` builds the
debug build and starts the app, and `release` makes the release build. An app for one
system has only the scripts that can run there.

Every setting has a flag and an environment variable (the first column is the name the Java
server used in `application.properties`, kept for people with old notes):

| Former Java property | Flag | Environment variable |
|---|---|---|
| `quarkus.http.port` | `--port` | `PETAJOURNAL_PORT` |
| `petajournal.data.dir` | `--data-dir` | `PETAJOURNAL_DATA_DIR` |
| `petajournal.data.db` | `--db` | `PETAJOURNAL_DATA_DB` |
| `petajournal.index.dir` | `--index-dir` | `PETAJOURNAL_INDEX_DIR` |
| `petajournal.config.dir` | `--config-dir` | `PETAJOURNAL_CONFIG_DIR` |
| (none) | `--throwaway-dir` | (none) |
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
`client/apps/web-app/outliner-react/dist`, the folder `npm run build` of the React shell
writes. The executable started by hand (`./target/release/journal-server`) does not see that
setting: pass `--frontend-dir`, or copy `dist/` to `target/release/frontend/`. With neither
folder the server serves the API only, and logs `Frontend: not served` at startup.

The server reads the files from disk on every request. After a new build of the frontend,
reload the browser; the server need not restart.

`RUST_LOG` controls logging (default `info,tantivy=warn`), e.g. `RUST_LOG=journal_server=debug`.

The search index format is not the same as the Java server's Lucene index. On the first start
the server replaces a Lucene index in `~/.peta-journal/index` with a tantivy index and rebuilds
it from the database; the database itself is not changed.

### Tests

```powershell
cd peta-journal/journal
cargo test
```

`cargo test` runs the unit tests and two test binaries. `journal-server/tests/it/` holds the API
tests and the other integration tests. `journal-server/tests/mcp/` holds the MCP tests. Each
binary starts the server once on a free port with everything under a temporary directory.

## Rust frontend (client)

`client/` holds the frontend: a [Crux](https://github.com/redbadger/crux) core
(`libs/client-core`, the outline logic, tested without a browser) and the shells around it.

The web shell is in React (`apps/web-app/outliner-react`, ADR 079). Its core runs in wasm.
It replaced the Leptos shell, which replaced the Angular frontend.

### Prerequisites

- `rustup`. `rust-toolchain.toml` sets the version of Rust and adds the `wasm32-unknown-unknown`
  target.
- Node.js
- [wasm-pack](https://rustwasm.github.io/wasm-pack/): `cargo install wasm-pack`

### Build and run

```powershell
cd peta-journal/client/apps/web-app/outliner-react
npm install                 # once
npm run build               # the files in dist/
```

`release.ps1` (on macOS and Linux: `release.sh`) in that folder does the same from any folder.
The journal server serves this `dist/` at `http://localhost:8087`; see "Build and run" of the
Rust server above. The README of the shell has the other commands, such as `npm run dev`.

### Tests

```powershell
cd peta-journal/client
cargo test                  # core (the ported service specs) and the shells
```

The Playwright suite in `client/apps/web-app/outliner-react/e2e/` drives the Rust server and
the React shell the way they run for the user. It builds the server with
`cargo build --release` and the shell with `npm run build` into `dist-e2e/`. Then it starts one
journal server on port 8187 that serves those files and the API:

```bash
cd peta-journal/client/apps/web-app/outliner-react/e2e
npm install
npx playwright install chromium   # once
npm run e2e
```

The test server runs with `--throwaway-dir <a temporary directory>`. Its data, its search index,
its configuration and its markdown folder are all under that directory, and the server keeps the
markdown folder there whatever `config.json` or a client says, so its export never lands in the
markdown folder of `~/.peta-journal/config.json`. A global setup checks this before the first
test and refuses to run otherwise. When starting a second server by hand for an experiment, start
it with `--throwaway-dir` too. With `--data-dir` alone it will export into your real markdown
folder.

### Coverage

Setup, once:

```
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
```

After a change, read the coverage for the lines no test reaches:

```
cd peta-journal/client
cargo cov            # a table per file
cargo cov --html     # target/llvm-cov/html/index.html, line by line
```

`cargo cov` fails when less than 90% of the lines are covered. The lines are counted over all the
measured crates together. The CI runs it too, and in `journal` and `atlas` as well. A line no
test reaches is either removed, or given a test of the behaviour that needs it.

`cargo cov` is an alias in `.cargo/config.toml`. It measures `client-core`, `client-page-links`,
`client-api-http`, `client-api-embedded`, `outliner-emacs`, `client-native-bridge`, `client-shell-http`,
`client-shell-embedded`, `client-cloud-sync`, `client-stickies`, `client-web-bridge`, `shell-check` and
`app-icon`: every crate of the workspace. `scripts/verification.test.mjs` checks that no crate is
missing, and guards the floor and the files left out.

No file of `outliner-emacs` is left out. Its tests start the program itself, and `cargo cov`
measures that run too.

`cargo cov` turns on the feature `facet_typegen` of `client-native-bridge`, so the generation of
the types of the native shells is measured too.

### Types of the native shells

`libs/client-native-bridge` holds the core for a shell that is not written in Rust, such as the SwiftUI
shell (ADR 024). The types that cross to such a shell are generated from the Rust types:

```
cd peta-journal/client
cargo run -p client-native-bridge --features facet_typegen --bin codegen -- swift <folder>
cargo run -p client-native-bridge --features facet_typegen --bin codegen -- csharp <folder>
cargo run -p client-native-bridge --features facet_typegen --bin codegen -- kotlin <folder>
```

The first writes the Swift package `PetaTypes` under `<folder>/swift`, the second the C#
project `PetaTypes` under `<folder>/csharp`, and the third the Kotlin package
`com.sosuisha.petajournal.types` under `<folder>/kotlin`. The generated code is not kept in the
repository.

The C# and the Kotlin types are not written by the generator that Crux brings (facet_generate
0.19): its C# and its Kotlin do not compile for these types. `src/codegen.rs` calls a later version of the same generator
directly (ADR 028). The tests of the generation need the feature too:
`cargo test -p client-native-bridge --features facet_typegen`.

This crate has no way to the journal, and no entrance for another language. A crate above it
has both (ADR 036). `libs/client-shell-http` is the one for the shells that ask a journal
server: the SwiftUI and the WinUI shell. Its feature `ffi` adds the entrance (`src/ffi.rs`),
made with BoltFFI. `client/apps/native-app/stickies-mac/README.md` describes how the SwiftUI shell is built from both, and the
tools it needs. `client/apps/native-app/stickies-win/README.md` does so for the WinUI shell.

A type that crosses has `#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]`. The
generator reads the shape of the Rust type, not what serde writes. A type with a hand-written
`Serialize` has to write that same shape in bincode, as `PropertyValue` and `ItemPosition` do.

### Core of the Android and iOS shells

`libs/client-shell-embedded` holds the core for the Android and the iOS shell. It has no journal server to ask:
the journal core (`journal/journal-core`) runs inside the app (ADR 029). The shell
puts Markdown files into a folder of the app and calls `sync()`, which takes them in and makes
them found by the search. `sync()` also deletes the pages of the files that have gone, without
asking (ADR 030). Then it writes the pages that changed in the app to the folder, for the
shell to upload (ADR 031). `open` shows a page that is there, and `open_or_make` also makes
one that is not. Nothing runs at times of its own: no watcher, no scan, no export.

BoltFFI packs the crate for Android, and the generator writes the Kotlin types beside it:

```
cd peta-journal/client
cargo run -p client-native-bridge --features facet_typegen --bin codegen -- kotlin apps/native-app/outliner-android/generated
cd libs/client-shell-embedded
boltffi pack android --release
```

This needs the Android NDK (`ANDROID_NDK_HOME`) and the Rust targets `aarch64-linux-android` and
`x86_64-linux-android`. Add them inside the repository, so that they go to the version in
`rust-toolchain.toml`: `rustup target add aarch64-linux-android x86_64-linux-android`.
The output is `client/apps/native-app/outliner-android/generated`: the libraries under
`jniLibs`, and the Kotlin under `kotlin`. It is not kept in the repository.
`client/apps/native-app/outliner-android/generate.ps1` runs both steps.

The tests of the crate run on the development machine, with `cargo test -p client-shell-embedded`.

The iOS shell uses the same crate (ADR 034). On a Mac, `client/apps/native-app/outliner-ios/generate.sh` writes the
Swift types and packs the crate with `boltffi pack apple --release`, for a device, for the
simulator and for the Mac. This needs the Rust targets `aarch64-apple-ios` and
`aarch64-apple-ios-sim`. Add them inside the repository, so that they go to the version in
`rust-toolchain.toml`: `rustup target add aarch64-apple-ios aarch64-apple-ios-sim`.
The output is `client/apps/native-app/outliner-ios/generated`.

### Emacs frontend

`apps/emacs/outliner` is a second shell around the same core. It is a native program, and an Emacs
mode draws what it sends. `apps/emacs/outliner/README.md` describes the setup, the keys and the tests.

```powershell
cd peta-journal/client
cargo build --release -p outliner-emacs
```

`apps/emacs/outliner/release.ps1` does the same. `debug.ps1` builds the debug build and starts
Emacs with it. Both have an `.sh` twin.

## SwiftUI frontend (stickies-mac)

The shell for macOS, a Walking Skeleton so far: it opens one page and saves what is typed into an
item. `client/apps/native-app/stickies-mac/README.md` describes the tools, the build of the app and the tests. Its
tests do not run in CI.

## WinUI frontend (stickies-win)

The shell for Windows. It is the same app as the SwiftUI shell: stickies that each show one page
(ADR 028). `client/apps/native-app/stickies-win/README.md` describes the tools, the build of the app and the tests.
The CI runs the tests of `PetaBorder` and measures their coverage. The tests of the windows do
not run in CI: they press real keys.

## Android frontend (outliner-android)

The shell for Android, in Kotlin with Jetpack Compose. It downloads the Markdown folder from
Dropbox, and a page is searched for and opened. Nothing is changed. The journal core runs
inside the app (ADR 029, ADR 030). `client/apps/native-app/outliner-android/README.md` describes the tools, the build of the app and the
tests. Its tests need a device or an emulator, and do not run in CI.

The App key of Dropbox and the folder in Dropbox are not kept in the repository. Write them
into the `gradle.properties` of your home folder (`~/.gradle/gradle.properties`), not into the
one of the app:

```
dropboxKey=<App key>
dropboxFolder=/PetaJournal
```

## iOS frontend (outliner-ios)

The shell for iOS, in Swift with SwiftUI. It does what the Android shell does, with the same
core inside the app (ADR 034). `client/apps/native-app/outliner-ios/README.md` describes the tools, the build of the
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
  measures in the font of the labels, `client/assets/NotoSansJP-Regular.ttf`.

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

The web shell loads all four in `src/atlasScripts.ts` of `client/apps/web-app/outliner-react`.
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

`.github/workflows/ci.yml` runs only when asked: from the Actions tab, or with
`gh workflow run CI --ref <branch>`. The runs are paid.

The checks of each job are listed once, in `scripts/checks.mjs`: the format, the lint, the tests
and the coverage. The workflow only installs the tools, then runs `node scripts/checks.mjs run <job>`.
The mutation tests are not in the list. Run them on your machine before a pull request.

### The checks before a pull request

A Claude Code hook (`.claude/settings.json`, `.claude/hooks/verify-before-pr.mjs`) runs the same
checks before `gh pr create`. It runs the jobs that the commits of the branch touch. A failed check
stops the pull request.

To see which jobs some files touch, or to run a job by hand:

```
node scripts/checks.mjs jobs journal/journal-core/src/lib.rs
node scripts/checks.mjs run todo
```

The hook itself is read from main, from the checkout where Claude Code started. Pull main there
before a pull request: an old hook stops it. The list of checks is read from the branch, so a job
that the branch adds runs too.

Only a step for another OS is skipped on a developer's machine, and the hook reports it. In the
CI, such a step fails the job. Any other step fails when what it needs is missing: a program, a
file, or an environment variable. The Emacs steps need two environment variables:

- `PETA_EMACS`: the `emacs` program, version 31. Not an `emacsclient` wrapper.
- `ELISP_FORMATTER`: the `elisp-formatter.el` of `sosuisen/elisp-formatter`.

On Windows, the build of memoview needs the MSBuild of Visual Studio. The hook finds it with
vswhere, or `MSBUILD` names it.

## Development Workflow

1. Build the frontend: `./client/apps/web-app/outliner-react/release.ps1` (or `release.sh`)
2. Start the journal server: `./journal/journal-server/debug.ps1` (or `debug.sh`)
3. Open `http://localhost:8087`

After a change to the frontend, run the script again and reload the browser. The journal
server keeps running.

## Daily use

`./release-all.ps1` (or `release-all.sh`) at the root of the repository brings everything up to
date for daily use. It pulls, closes the apps, and stops the journal server with Ctrl+C. Then it
makes the release builds of every app for this system. It copies `dist/` of the web frontend to
`journal/target/release/frontend/`. It copies `dist/` of todo to `apps/todo/` of the config
folder (ADR 073). The config folder is `~/.peta-journal`, or `PETAJOURNAL_CONFIG_DIR` when it is
set. Then it starts the server. When port 8087 answers, it opens `http://localhost:8087` and
`http://localhost:8087/apps/todo/` in the browser, and starts PetaStickies. Pass `-NoPull` (or
`--no-pull`) to build what is checked out.
The steps by hand are in [README.ja.md](README.ja.md).

