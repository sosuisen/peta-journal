# SwiftUI frontend (frontend-swiftui)

The SwiftUI shell of PetaJournal for macOS. It is a Walking Skeleton so far (ADR 024): the Rust
core can be called from Swift, and tests show that Swift and Rust read each other's bytes. The
app itself does not exist yet.

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
- [XcodeGen](https://github.com/yonaskolb/XcodeGen) 2.46.0. It will make the Xcode project of
  the app from a text file. Nothing uses it yet:

  ```
  brew install xcodegen
  ```

## Build and test

```
cd peta-journal/frontend-swiftui
./generate.sh     # makes generated/ from the Rust crates
swift test
```

`generate.sh` makes two Swift packages under `generated/`. Run it again after a change in Rust.
`generated/` is not kept in the repository.

- `generated/swift/PetaTypes`: the types that cross the border, generated from the Rust types
  (`Event`, `ViewModel`, `Request`, ...).
- `generated/PetaCore`: the core as a static library (`PetaCore.xcframework`), and the Swift
  class `PetaCore` that calls it. BoltFFI makes both, from `frontend-rust/crates/shell-native/src/ffi.rs`
  and `boltffi.toml` next to it.

`Sources/PetaBorder` is hand-written: `Border` puts the two together, so the rest of the shell
works with the generated types and never with bytes.

## Tests

`swift test` runs `Tests/PetaBorderTests`. They need no journal server. They check that Swift
reads what Rust writes (the view model, the requests) and that Rust reads what Swift writes
(events, answers).

Why these tests matter: the Swift types are generated from the shape of the Rust types, not from
what serde writes. A Rust type with a hand-written `Serialize` can write other bytes than its
shape says. Then everything compiles, and the shell cannot read the bytes. Only a test that
crosses the border finds this.

These tests do not run in CI. A macOS runner costs much more than a Linux one, so this is put
off. Run them by hand after a change to a type that crosses the border. CI does run the Rust
tests of the entrance (`cargo test -p peta-shell-native --features facet_typegen,ffi`).
