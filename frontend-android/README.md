# Android frontend (frontend-android)

The Android shell of PetaJournal, written in Kotlin with Jetpack Compose. It downloads the
Markdown folder from Dropbox, and a page is searched for and opened, with its items shown with
their decoration. A touch on an item edits its Markdown, and the sync uploads the pages that
changed in the app (ADR 031).

The core is the same Crux core as in the other frontends. There is no journal server on a
phone, so the journal core runs inside the app (ADR 029). `frontend-rust/crates/shell-android`
wraps both for Kotlin. Kotlin gets the view model and the requests of the screen as bytes
(bincode), and sends events the same way. ADR 030 says how the app syncs, where it keeps its
data and how it draws an item. ADR 031 says how it edits and uploads.

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
someone types the PIN. The tests of the download and the upload (`app/src/test`) use no core
and run on the development machine.

## Dropbox

The app reads and writes Dropbox with the App key of a Dropbox app, which each builder
registers at <https://www.dropbox.com/developers/apps>:

- Access type: Full Dropbox, to use the folder that the PC already uses.
- Permissions: `files.metadata.read`, `files.content.read` and `files.content.write`. The app
  asks for no more. A Dropbox app that was registered for reading only needs the third one
  turned on, on its "Permissions" tab.
- No redirect URI is needed, and the App secret is not used: the sign-in is PKCE.

A sign-in of a version of the app that only read cannot upload. The app lets go of it, and
shows the sign-in again.

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

The app syncs when it comes to the front, when it goes to the back, when "Sync" is touched,
and 3 seconds after the last change of a page. The sync of an app that came to the front shows
where it stands in a dialog while it moves files: the pages may be old until then. The
dialog cannot be closed. The other syncs show it in the line under the search field only. A sync that is asked for while another runs
starts after it, once. When Android stops the app in the back before the sync has ended, the
rest goes up when the app opens again. The first sync downloads
every Markdown file and every file of the `assets` folder, several at a time. The files of
`assets` are the images of the pages. A later sync asks Dropbox only for what changed. A
file that Dropbox lost is deleted in the app, and so is its page, without a question.

One kind of file stays when Dropbox lost it: a page that changed in the app, or was made
there, and is not uploaded yet. The next upload brings it to Dropbox as a new file. What was
written on the phone is not lost in silence.

A sync has three steps, always in this order (ADR 031):

1. It downloads what changed in Dropbox.
2. The core takes in the files. Then it writes the files of the pages that changed in the
   app. A page that changed on both sides is put together item by item.
3. It uploads the Markdown files whose content is not what Dropbox had.

The app keeps the version (`rev`) and the content hash of each file, as Dropbox last told
them. An upload names the version that it replaces. Dropbox refuses when it has another
version, and the sync starts again with the download. After three refusals, the sync fails.
The app deletes and moves nothing in Dropbox, and it uploads no file of `assets`.

"Sign out" lets go of Dropbox. The app deletes its credential and tells Dropbox to take the
token no longer. Without the network, Dropbox does not hear of it, and the token is gone
from the device only. The pages and the images that were downloaded stay, and can be read.
The app syncs nothing until the next sign-in.

The next sign-in may be to another account. So the sign-out also forgets where the last
download ended. The first sync after the sign-in lists the whole folder. It downloads only
the files that differ, and deletes the copies of what the account does not hold. The pages
that are not uploaded yet stay, and go up to the account of the new sign-in.

## The code

`app/src/main/kotlin/net/sosuisen/petajournal`:

- `Border` is the core behind its border of bytes. The rest of the shell works with the
  generated types and never with bytes.
- `Shell` is what the app shows. It keeps the view model up to date with the core and sends
  what the user did. The core calls from its own threads; `Shell` moves each request to the
  main thread, in order. `sync()` takes in the files of the Markdown folder on a thread of
  its own. `syncAndWait()` does the same for the thread of a sync with the cloud, and returns
  when the core is done.
- `ShownText` makes the parts of an item as it is shown. Rust makes HTML of the AST of the
  item, and `AnnotatedString.fromHtml` of Compose reads the HTML. That loses the line breaks
  of a code block and the cells of a table, so `ShownText` reads those two from the AST, and
  the screen draws them with parts of their own.
- A list or a quote that holds a code block, a table or an image cannot be one text.
  `ShownText` takes it apart: each entry of the list gets its mark, and the quote gets a line
  down its left side. What is in them is read the same way, at any depth. A list or a quote
  that holds none of these stays in the text, as before.
- A link to a page and a tag have the URL of their page in the HTML, such as `/pages/notes`.
  A touch gives the URL to `Shell.follow`. The core opens the page only when it is there.
  The web app makes the page of a link to a new page. Here a touch by mistake must not put
  an empty page into Dropbox: the screen says that the page is not there yet, and "Make it"
  beside that makes the page.
- The search has two results that make a page. "Open Today" is for the journal of today:
  it shows when the word `today` is typed. "New page" has the name that was typed: it shows
  when no page has that name. A page that is made in the app has no items from a template:
  the app runs no extension functions.
- An image of the `assets` folder is drawn from its file in the Markdown folder. `ShownText`
  cuts the paragraph at the image, and the screen draws the image between the text before
  and after it. The image is as wide as the page asks with `#w=`, or as wide as it is in its
  file, and never wider than the screen. A page may also ask for a height with `#h=`. With
  one side, the other follows the shape of the image. With both, the image takes that shape,
  as in a browser. An image whose file is not there shows its alt.
- A touch opens an image over the whole screen, to read what is small in it. Two fingers
  make it larger, up to 8 times, and move it. A touch or the back key closes it. The large
  view reads the file again, with at least twice the width of the screen in pixels.
- An image of the web is not drawn. It is a link with its alt, so the app talks to no server
  but Dropbox.
- A link to a file of the `assets` folder, such as a PDF, opens the file with the app that the
  device has for its kind. `FileProvider` lends the file for reading. Only the `assets` folder
  is lent: the pages and the database are not. A link to a file that is not there says so.
- A task has a checkbox that shows where it stands. A touch moves the task on: not started,
  running, done, and then no task.
- While an item is edited, a row of tools stands above the keyboard: move the item out of
  its parent or under the item above, move it up or down, make it a task or move the task
  on, and delete it. "Done" ends the edit. A touch on a tool leaves the focus in the field.
- A touch on an item edits it: the item becomes a field with its Markdown, with the cursor
  at its end. A touch on a link or an image still opens it. Only the rest of the row edits,
  so an item that is one long link is edited by a touch beside it. The core has the text
  and saves it a moment after the last key. Enter splits the item at the cursor, and the
  edit goes on in the new item. A touch on another item moves the edit there. The field
  takes no line break. It tells of the line break it could not put in a second time, and
  the screen does not take that for a second Enter.
- The core counts a cursor in characters, and a field of Android counts in UTF-16 units.
  The screen converts between the two. While the keyboard composes a word, as in Japanese,
  the core hears nothing, and the field is not written over.
- The core asks the shell to focus an editor and waits for the answer. `Shell` answers at
  once: the row that is edited takes the focus when it is drawn.
- The core asks before it does what is hard to undo, such as the delete of an item that
  another item transcludes. `Shell.question` holds the question, and the screen shows it in
  a dialog. A touch outside the dialog answers no. A second question answers the first
  with no.
- `JournalScreen` is the one screen: the search field, and under it the results or the page.
- `MainActivity` and `JournalModel` open the journal in the folders of the app:
  `files/markdown` for the Markdown files and `files/data` for the database, the index and
  the settings. They also keep the credential of Dropbox, and start the sign-in and the sync.
- `sync/Downloader` brings the Markdown folder of the app up to date with the cloud. It knows
  the cloud only as `Cloud`, so its tests need no Dropbox. `MarkdownFolder` is the folder of
  the app, where a path is found without regard to case, as in Dropbox. `DropboxCloud` is
  `Cloud` by the Dropbox SDK.
- `sync/Uploader` uploads the Markdown files that changed in the app. `CloudRecord` keeps the
  version and the hash of each file in the cloud, between two runs of the app. `SyncRound` is
  one sync: download, core, upload, and again when the cloud refuses. `CloudSync` runs one
  sync at a time and tells the screen where it stands.

## Not there yet

- A sync while the app is not on screen.
- Links to a file outside the `assets` folder, such as a file of the PC.
- An image inside a cell of a table or inside a heading.
