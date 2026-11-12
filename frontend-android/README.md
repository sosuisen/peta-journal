# Android frontend (frontend-android)

The Android shell of PetaJournal, written in Kotlin with Jetpack Compose. It downloads the
Markdown folder from Dropbox, and a page is searched for and opened, with its items shown with
their decoration. It changes nothing, in the app or in Dropbox.

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
./gradlew :app:assembleDebug                # builds the app, without the sync
./gradlew :app:testDebugUnitTest            # runs the tests that need no device
./gradlew :app:connectedDebugAndroidTest    # runs the tests on a device or an emulator
```

The package holds the core for both kinds of device, about 70MB each. `-Pabi=x86_64` packs
only one, for an emulator with little room.

`generate.sh` does the same as `generate.ps1`, for Git Bash. Run it again after a change in
Rust. `generated/` is not kept in the repository:

- `generated/kotlin`: the types that cross the border (`Event`, `ViewModel`, `Request`, ...),
  made by `facet_generate` 0.22.3, and the class `PetaCore` that calls the core, made by
  BoltFFI.
- `generated/jniLibs`: the core as a library, for `arm64-v8a` and `x86_64`.

The tests of the screen need a running device or emulator: the core is a native library, and
BoltFFI cannot pack it for the JVM of a Windows machine yet (ADR 030). The device must be
unlocked. An emulator with a screen lock starts locked, and no app can start on it until
someone types the PIN. The tests of the download (`app/src/test`) use no core and run on the
development machine.

## Dropbox

The app reads Dropbox with the App key of a Dropbox app, which each builder registers at
<https://www.dropbox.com/developers/apps>:

- Access type: Full Dropbox, to read the folder that the PC already uses.
- Permissions: `files.metadata.read` and `files.content.read`. The app asks for no more.
- No redirect URI is needed, and the App secret is not used: the sign-in is PKCE.

The key and the path of the Markdown folder in Dropbox come from outside the repository, as
Gradle properties:

```
./gradlew :app:assembleDebug -PdropboxKey=<App key> -PdropboxFolder=/PetaJournal
```

They can also stand in `~/.gradle/gradle.properties`, as `dropboxKey=...` and
`dropboxFolder=...`. An app that is built without a key syncs nothing and shows no sign-in.

The Dropbox app signs in when the device has it. The browser signs in when it does not. The
Dropbox app must be signed in itself. When it is signed out, it only says that it could not
connect to Dropbox and asks to check the network. The network is fine then: sign in again in
the Dropbox app, and try once more.

The app syncs when it comes to the front and when "Sync" is touched. The first sync downloads
every Markdown file and every file of the `assets` folder, several at a time. The files of
`assets` are the images of the pages. A later sync asks Dropbox only for what changed. A
file that Dropbox lost is deleted in the app, and so is its page, without a question.

"Sign out" lets go of Dropbox. The app deletes its credential and tells Dropbox to take the
token no longer. Without the network, Dropbox does not hear of it, and the token is gone
from the device only. The pages and the images that were downloaded stay, and can be read.
The app syncs nothing until the next sign-in.

The next sign-in may be to another account. So the sign-out also forgets where the last
download ended. The first sync after the sign-in lists the whole folder. It downloads only
the files that differ, and deletes what the account does not hold.

## The code

`app/src/main/kotlin/net/sosuisen/petajournal`:

- `Border` is the core behind its border of bytes. The rest of the shell works with the
  generated types and never with bytes.
- `Shell` is what the app shows. It keeps the view model up to date with the core and sends
  what the user did. The core calls from its own threads; `Shell` moves each request to the
  main thread, in order. `sync()` takes in the files of the Markdown folder on a thread of
  its own.
- `ShownText` makes the parts of an item as it is shown. Rust makes HTML of the AST of the
  item, and `AnnotatedString.fromHtml` of Compose reads the HTML. That loses the line breaks
  of a code block and the cells of a table, so `ShownText` reads those two from the AST, and
  the screen draws them with parts of their own.
- A list or a quote that holds a code block, a table or an image cannot be one text.
  `ShownText` takes it apart: each entry of the list gets its mark, and the quote gets a line
  down its left side. What is in them is read the same way, at any depth. A list or a quote
  that holds none of these stays in the text, as before.
- A link to a page and a tag have the URL of their page in the HTML, such as `/pages/notes`.
  A touch gives the URL to `Shell.follow`. The core opens the page only when it is there:
  the web app makes the page of a link to a new page, and this app only reads. The screen
  then says that the page is not there yet.
- An image of the `assets` folder is drawn from its file in the Markdown folder. `ShownText`
  cuts the paragraph at the image, and the screen draws the image between the text before
  and after it. The image is as wide as the page asks with `#w=`, or as wide as it is in its
  file, and never wider than the screen. An image whose file is not there shows its alt.
- A touch opens an image over the whole screen, to read what is small in it. Two fingers
  make it larger, up to 8 times, and move it. A touch or the back key closes it. The large
  view reads the file again, with at least twice the width of the screen in pixels.
- An image of the web is not drawn. It is a link with its alt, so the app talks to no server
  but Dropbox.
- A task has a checkbox that shows where it stands. A touch does not change it.
- `JournalScreen` is the one screen: the search field, and under it the results or the page.
- `MainActivity` and `JournalModel` open the journal in the folders of the app:
  `files/markdown` for the Markdown files and `files/data` for the database, the index and
  the settings. They also keep the credential of Dropbox, and start the sign-in and the sync.
- `sync/Downloader` brings the Markdown folder of the app up to date with the cloud. It knows
  the cloud only as `Cloud`, so its tests need no Dropbox. `MarkdownFolder` is the folder of
  the app, where a path is found without regard to case, as in Dropbox. `DropboxCloud` is
  `Cloud` by the Dropbox SDK. `CloudSync` runs one download at a time and tells the screen
  where it stands.

## Not there yet

- A sync while the app is not on screen.
- Links to other things than a page or a web address, such as a file.
- An image inside a cell of a table or inside a heading.
- The height of an image that the page asks with `#h=`. The height follows the width.
