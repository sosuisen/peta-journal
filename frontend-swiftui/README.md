# SwiftUI frontend (frontend-swiftui)

The SwiftUI shell of PetaJournal for macOS. It is a Walking Skeleton so far (ADR 024): the app
opens one page, shows its items as an outline, and what is typed into an item is saved. Enter
splits an item, Tab and Shift+Tab change its level, the arrows move between the items, and
Backspace at the start joins an item to the one before. It has no sidebar and no search yet.

The core is the same Crux core as in the web and the Emacs frontends. `frontend-rust/crates/shell-native`
wraps it for a shell that is not written in Rust. Calls to the journal server, timers and the
clock stay in Rust. Swift gets the view model and the requests of the screen as bytes (bincode),
and sends events and answers the same way.

## Prerequisites

- macOS on Apple silicon. Only `arm64` is built.
- Xcode 27.2. After installing or updating it, run `xcodebuild -runFirstLaunch` once. Without
  this, `boltffi pack apple` fails at `xcodebuild -create-xcframework` with "A required plugin
  failed to load".
- The Rust toolchain, with the target `aarch64-apple-darwin` (the default on such a Mac).
- [BoltFFI](https://boltffi.dev) 0.32.1, the command line tool. It makes the Rust library and
  the Swift code that calls it:

  ```
  cargo install boltffi_cli --version '=0.32.1' --locked
  ```

  The version has to be the same as the `boltffi` dependency in
  `frontend-rust/crates/shell-native/Cargo.toml`.
- [XcodeGen](https://github.com/yonaskolb/XcodeGen) 2.46.0. It makes the Xcode project of the
  app from `project.yml`:

  ```
  brew install xcodegen
  ```

## Build and test

```
cd peta-journal/frontend-swiftui
./generate.sh     # makes generated/ from the Rust crates
swift test
```

`generate.sh` builds the Rust library optimized (`boltffi pack apple --release`; without the
flag BoltFFI makes a debug build, which is about twice as slow). It makes two Swift packages
under `generated/`. Run it again after a change in Rust.
`generated/` is not kept in the repository.

- `generated/swift/PetaTypes`: the types that cross the border, generated from the Rust types
  (`Event`, `ViewModel`, `Request`, ...).
- `generated/PetaCore`: the core as a static library (`PetaCore.xcframework`), and the Swift
  class `PetaCore` that calls it. BoltFFI makes both, from `frontend-rust/crates/shell-native/src/ffi.rs`
  and `boltffi.toml` next to it.

`Sources/PetaBorder` is hand-written. It has no SwiftUI in it, so `swift test` can test it:

- `Border` puts the two packages together, so the rest of the shell works with the generated
  types and never with bytes.
- `Shell` is what one window shows. It keeps the view model up to date with the core and sends
  what the user did. It also holds what the core asks and waits for: a question to the user
  (`question`, shown as a dialog), an order to put the cursor in an item (`focus`), and the text
  on the clipboard.
- `Keys.swift` says what a key in the editor of an item means to the outline (`Shell.pressed`),
  and `Cursor` turns the place of a cursor from the UTF-16 units of AppKit into the characters
  the core counts, and back. The core calls from its own threads; `Shell` moves each request to the main
  thread, in order.

## Build and run the app

The app needs a running journal server (see `BUILD.md` at the top of the repository).

`run.sh` does all of the below: it builds the app (the Release build), ends a running app, and
starts the new one. What follows its own options goes to the app:

```
cd peta-journal/frontend-swiftui
./run.sh -url /pages/notes     # --generate first makes generated/ again, after a change in Rust
./run.sh --help
```

By hand, the steps are:

```
cd peta-journal/frontend-swiftui
./generate.sh
xcodegen generate     # makes PetaJournal.xcodeproj from project.yml
xcodebuild -project PetaJournal.xcodeproj -scheme PetaJournal -derivedDataPath build build
open build/Build/Products/Debug/PetaJournal.app
```

This is the Debug build of the Swift code. It reads the view model about three times slower than
the Release build: add `-configuration Release` to judge the speed of the app (the app is then
under `build/Build/Products/Release`).

Or open `PetaJournal.xcodeproj` in Xcode and run it there. The project is not kept in the
repository: run `xcodegen generate` again after a change in `project.yml` or a new file in `App/`.

Without arguments the app opens today's journal of the server at `http://localhost:8087`. To
choose the server and the page:

```
open build/Build/Products/Debug/PetaJournal.app --args -server http://localhost:8087 -url /pages/notes
```

`open` only brings a running app to the front: it starts no new one, and the arguments and a new
build are ignored. Quit the app first (Cmd+Q). Start the journal server before the app, too: the
app does not load the page again when the server comes up later.

`Sources/PetaScreens` holds the views, and `App/` only the start of the app. The views draw the
view model of `Shell` and decide nothing about the outline. They are a library so that tests can
put them in a window.

The text of an item is a text view of AppKit (`ItemText`), not a text field of SwiftUI: the keys
of the outline need the place of the cursor, which SwiftUI does not tell. Text that is still
being composed by an input method is not sent to the core until it is confirmed, and the Enter
that confirms it does not split the item.

## Tests

`swift test` runs `Tests/PetaBorderTests`.

`BorderTests` need no journal server. They check that Swift reads what Rust writes (the view
model, the requests) and that Rust reads what Swift writes (events, answers).

`ShellTests` run `Shell` against a real journal server: a page opens, what is typed is saved on
the server, and the questions of the core are answered. They are skipped unless `PETA_E2E_SERVER` names the server program:

```
cd peta-journal/journal-server-rust
cargo build --release
cd ../frontend-swiftui
PETA_E2E_SERVER=$PWD/../journal-server-rust/target/release/journal-server swift test
```

`OutlineScreenTests` put the screen in a window and give its editors the text and the keys as
AppKit does: typing, composing, Enter, the arrows, Tab and Backspace. No key of a real keyboard
is pressed, so they do not show that an input method works with the editors.

`TypingSpeedTests` measures one key on a page of 1000 items, with the views in a real window. It
prints the times and fails on none of them. Run it optimized, and alone:

```
PETA_E2E_SERVER=$PWD/../journal-server-rust/target/release/journal-server \
  swift test -c release -Xswiftc -enable-testing --filter TypingSpeed
```

On an M1 MacBook Air (2026-11-11): the event into the core 0.07 ms, the view model out of the
core and read in Swift 4.9 ms, the window drawn again 4.2 ms.

The tests start the server themselves, on ports 18094, 18095 and 18097, with all its files in a temporary folder.
They check that the server writes its Markdown there before they use it, so the notes of whoever
runs the tests are never touched.

Why these tests matter: the Swift types are generated from the shape of the Rust types, not from
what serde writes. A Rust type with a hand-written `Serialize` can write other bytes than its
shape says. Then everything compiles, and the shell cannot read the bytes. Only a test that
crosses the border finds this.

These tests do not run in CI. A macOS runner costs much more than a Linux one, so this is put
off. Run them by hand after a change to a type that crosses the border. CI does run the Rust
tests of the entrance (`cargo test -p peta-shell-native --features facet_typegen,ffi`).
