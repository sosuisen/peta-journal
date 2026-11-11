# Android frontend (frontend-android)

The Android shell of PetaJournal, written in Kotlin with Jetpack Compose. It is a Walking
Skeleton so far: a page is searched for and opened, and its items are shown with their
decoration. It changes nothing. The download from Dropbox is not there yet, so the app shows
only the files that are put into its Markdown folder by hand or by a test.

The core is the same Crux core as in the other frontends. There is no journal server on a
phone, so the journal core runs inside the app (ADR 029). `frontend-rust/crates/shell-android`
wraps both for Kotlin. Kotlin gets the view model and the requests of the screen as bytes
(bincode), and sends events the same way. ADR 030 says how the app syncs, where it keeps its
data and how it draws an item.

## Prerequisites

- The Android SDK, with `ANDROID_HOME` set. Gradle installs the platform it needs (API 37).
- The Android NDK, with `ANDROID_NDK_HOME` set.
- A JDK 21, with `JAVA_HOME` set.
- The Rust toolchain, with the targets `aarch64-linux-android` and `x86_64-linux-android`.
- [BoltFFI](https://boltffi.dev) 0.32.1, the command line tool:

  ```
  cargo install boltffi_cli --version '=0.32.1' --locked
  ```

  The version has to be the same as the `boltffi` dependency in
  `frontend-rust/crates/shell-android/Cargo.toml`.

## Build and test

```
cd peta-journal/frontend-android
./generate.ps1                              # makes generated/ from the Rust crates
./gradlew :app:assembleDebug                # builds the app
./gradlew :app:connectedDebugAndroidTest    # runs the tests on a device or an emulator
```

`generate.sh` does the same as `generate.ps1`, for Git Bash. Run it again after a change in
Rust. `generated/` is not kept in the repository:

- `generated/kotlin`: the types that cross the border (`Event`, `ViewModel`, `Request`, ...),
  made by `facet_generate` 0.22.3, and the class `PetaCore` that calls the core, made by
  BoltFFI.
- `generated/jniLibs`: the core as a library, for `arm64-v8a` and `x86_64`.

The tests need a running device or emulator: the core is a native library, and BoltFFI cannot
pack it for the JVM of a Windows machine yet (ADR 030). The device must be unlocked. An
emulator with a screen lock starts locked, and no app can start on it until someone types the
PIN.

## The code

`app/src/main/kotlin/net/sosuisen/petajournal`:

- `Border` is the core behind its border of bytes. The rest of the shell works with the
  generated types and never with bytes.
- `Shell` is what the app shows. It keeps the view model up to date with the core and sends
  what the user did. The core calls from its own threads; `Shell` moves each request to the
  main thread, in order. `sync()` takes in the files of the Markdown folder on a thread of
  its own.
- `ShownText` makes the text of an item as it is shown. Rust makes HTML of the AST of the
  item, and `AnnotatedString.fromHtml` of Compose reads the HTML.
- `JournalScreen` is the one screen: the search field, and under it the results or the page.
- `MainActivity` and `JournalModel` open the journal in the folders of the app:
  `files/markdown` for the Markdown files and `files/data` for the database, the index and
  the settings.

## Not there yet

- The download from Dropbox.
- Code blocks and tables. `fromHtml` loses their line breaks and cells.
- Links to pages and tags. Only a link to a web address opens.
- The marks of tasks, and images.
