# Android frontend (outliner-android)

The Android shell of PetaJournal, written in Kotlin with Jetpack Compose. It downloads the
Markdown folder from Dropbox, and a page is searched for and opened, with its items shown with
their decoration. A touch on an item edits its Markdown, and the sync uploads the pages that
changed in the app (ADR 031).

The core is the same Crux core as in the other frontends. There is no journal server on a
phone, so the journal core runs inside the app (ADR 029). `client/libs/client-shell-embedded`
wraps both for Kotlin. Kotlin gets what changed on the screen (ADR 076) and the requests of the
core as bytes (bincode), and sends events the same way. ADR 030 says how the app syncs, where it keeps its
data and how it draws an item. ADR 031 says how it edits and uploads.

## Prerequisites

- The Android SDK, with `ANDROID_HOME` set. Gradle installs the platform it needs (API 37.1).
- The Android NDK, with `ANDROID_NDK_HOME` set.
- A JDK 21, with `JAVA_HOME` set.
- The Rust toolchain, with the targets `aarch64-linux-android` and `x86_64-linux-android`.
- [BoltFFI](https://boltffi.dev) 0.32.1, the command line tool:

  ```
  cargo install boltffi_cli --version '=0.32.1' --locked
  ```
- For `connectedDebugAndroidTest`, an emulator or a device. An emulator takes some GB, so its
  AVD can be kept on another disk: set `ANDROID_AVD_HOME` to a folder there, for both the
  commands below and the emulator. `ANDROID_HOME/cmdline-tools/latest/bin` has `sdkmanager`
  and `avdmanager`:

  ```
  export ANDROID_AVD_HOME=/Volumes/<disk>/android-avd
  sdkmanager "emulator" "system-images;android-37.0;google_apis;arm64-v8a"
  avdmanager create avd -n peta-api37 -k "system-images;android-37.0;google_apis;arm64-v8a" -d pixel_5
  $ANDROID_HOME/emulator/emulator -avd peta-api37 -no-snapshot &
  ```

  The version has to be the same as the `boltffi` dependency in
  `client/libs/client-shell-embedded/Cargo.toml`.

## Build and test

```
cd peta-journal/client/apps/native-app/outliner-android
./generate.ps1                              # makes generated/ from the Rust crates
./gradlew :app:assembleDebug                # builds the app, without the sync
./gradlew :app:testDebugUnitTest            # runs the tests that need no device
./gradlew :app:connectedDebugAndroidTest    # runs the tests on a device or an emulator
```

The formatter of the Kotlin code is ktfmt, in the Kotlin style with an indent of 4 spaces. It
needs no device and no `generated/`. It leaves `generated/kotlin` alone, as `generate.ps1`
writes it again. `./gradlew check` also runs `ktfmtCheck`.

```
./gradlew ktfmtFormat     # formats the Kotlin code
./gradlew ktfmtCheck      # changes nothing, and fails when there is something to change
```

The mutation tests are PIT, through the Gradle plugin `pl.droidsonroids.pitest`. Run them before
each pull request. PIT runs only the tests in `app/src/test`, on the JVM, so it mutates only the
classes that they test: they are listed in `app/build.gradle.kts`. Gradle fetches PIT, so
nothing more has to be installed. The report is in `app/build/reports/pitest/debug/index.html`.

```
./gradlew :app:pitestDebug
```

Two scripts do the usual steps. Both run `generate` first when `generated/` is missing, and
with `-Generate`. Everything else goes to Gradle:

```
./debug.ps1 -Pabi=arm64-v8a     # installs the debug build on the device that adb sees, and starts it
./release.ps1                   # builds app/build/outputs/apk/release/app-release-unsigned.apk
```

`debug.sh` and `release.sh` do the same, for macOS, Linux and Git Bash. Their option is
`--generate`. The release build is not signed, so no device takes it as it is.

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
someone types the PIN. The tests in `app/src/test` use no core and run on the development
machine: how a failure of the SDK is read (`DropboxFailuresTest`), and how the file of an image is found. The rules of
the sync are tested in Rust (`client-cloud-sync`, ADR 054).

## Dropbox

The app reads and writes Dropbox with the App key of a Dropbox app, which each builder
registers at <https://www.dropbox.com/developers/apps>:

- Access type: Full Dropbox, to use the folder that the PC already uses.
- Permissions: `files.metadata.read`, `files.content.read` and `files.content.write`. The app
  asks for no more. A Dropbox app that was registered for reading only needs the third one
  turned on, on its "Permissions" tab.
- No redirect URI is needed, and the App secret is not used: the sign-in is PKCE.

> [!IMPORTANT]
> Check no **Team Scope** on the "Permissions" tab of the Dropbox app (App Console), and
> press "Submit" after a change. One checked team scope stops the sign-in through the
> Dropbox app of the phone. On an iPhone it said "Mobile authorization does not support team
> authorized scopes." with `team_data.member` checked, and the sign-in worked after it was
> unchecked (2026-11-13). PetaJournal needs only the three scopes above, which are
> Individual Scopes.

A sign-in of a version of the app that only read cannot upload. The app lets go of it, and
shows the sign-in again.

The key and the path of the Markdown folder in Dropbox come from outside the repository, as
Gradle properties. Write them once into the `gradle.properties` of your home folder:
`~/.gradle/gradle.properties`, which is `C:\Users\<name>\.gradle\gradle.properties` on
Windows. Make the file when there is none:

```
dropboxKey=<App key>
dropboxFolder=/PetaJournal
```

Every build then has them, also `debug.ps1` and `release.ps1` without arguments, and in
every worktree. The file is outside the repository, so each machine needs its own.

Do not write them into the `gradle.properties` of this folder. That file is kept in the
repository, and the key would go into a commit.

For one build, they can also be arguments. They win over the file:

```
./gradlew :app:assembleDebug -PdropboxKey=<App key> -PdropboxFolder=/PetaJournal
```

An app that is built without a key syncs nothing and shows no sign-in.

The Dropbox app signs in when the device has it. The browser signs in when it has not. The
SDK decides this: it asks to see the Dropbox app with a `queries` entry of its own manifest.

For a while the browser always signed in (ADR 033). The sign-in through the Dropbox app
failed then: after a sign-out, and when the Dropbox app was signed out itself, the Dropbox
app only said that it could not connect to Dropbox. The cause was a team scope of the
Dropbox app (see the note above): `team_data.member` was checked. With no team scope, the
sign-in through the Dropbox app worked on a device (2026-11-14).

The app syncs when it comes to the front, when it goes to the back, when "Sync" is touched,
and 3 seconds after the last change of a page. The sync of an app that came to the front shows
where it stands in a dialog while it moves files: the pages may be old until then. The
dialog cannot be closed. The other syncs show it in a callout over the bottom of the page, which takes no
room: the user goes on typing. A sync that failed says why in the same callout, with
"Try again". A sync that is asked for while another runs
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

A new app shows only "Sign in to Dropbox" before the sign-in. It makes no journal of today:
Dropbox may hold that journal already, and the app would then have two files for one day.
The bar with the search and the button for today is not there either, because both make
pages. The first sync after the sign-in opens the journal of today. An app that has shown a
page before opens that page also when it is signed out.

The next sign-in may be to another account. So the sign-out also forgets where the last
download ended. The first sync after the sign-in lists the whole folder. It downloads only
the files that differ, and deletes the copies of what the account does not hold. The pages
that are not uploaded yet stay, and go up to the account of the new sign-in.

## The code

`app/src/main/kotlin/com/sosuisha/petajournal` has four packages. A package uses only the
ones below it in this list.

- The package itself: `MainActivity` and `JournalModel`, which put the others together.
- `screen`: what is drawn. `JournalScreen` is the whole screen. `Outline` is a page and its
  rows, `ItemEditor` the field of the item that is edited, and `Tools` the row above the
  keyboard, with `Pad` for its pads of four arrows. `Blocks` and `ImageBlock` draw the parts of an item, which `ShownText` makes.
  `SyncViews` shows where the sync stands.
- `shell`: `Shell` and `Border`, between the screen and the core.
- `sync` and `dropbox`: the sync with the cloud.

What each class does:

- `Border` is the core behind its border of bytes. The rest of the shell works with the
  generated types and never with bytes.
- `Shell` is what the app shows. It keeps the view model up to date with the core and sends
  what the user did. The core calls from its own threads; `Shell` moves each request to the
  main thread, in order. `sync()` takes in the files of the Markdown folder on a thread of
  its own. `syncAndWait()` does the same for the thread of a sync with the cloud, and returns
  when the core is done.
- Copy and paste go through the core, as on the desktop. While items are selected, Copy and
  Cut of `Tools` put them on the clipboard as a Markdown list (`Shell.copySelected`,
  `Shell.cutSelected`, answered by `UiOp.CopyMarkdown`). In `ItemEditor`, Paste of the menu
  of the text, Ctrl+V of a keyboard with real keys and the paste of accessibility go to
  `Shell.paste`, and the core makes items of a list. While text is chosen, the field puts the
  pasted text in its place.
- `ShownText` writes the parts of an item as it is shown, from the parts that the core gives
  with each row (`Row.bodyModel`, ADR 042). The text is an `AnnotatedString`; a code block,
  a table and an image of the assets folder are parts of their own, which the screen draws. A
  `when` takes every kind of part, so a kind that the core makes later stops the build until
  it is drawn. A touched link gives where the core says it leads.
- A list or a quote that holds a code block, a table or an image cannot be one text. The
  core takes it apart: each entry of the list gets its mark, and the quote gets a line down
  its left side. A list or a quote that holds none of these stays in the text.
- A link to a page and a tag have the URL of their page, such as `/pages/notes`.
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
- While an item is edited, the tools stand above the keyboard, two rows high. From the left:
  hide or show the children of the item over making it a task, a pad that moves the item, a
  pad that moves the cursor, redo over undo, and shrinking the tools over hiding the keyboard.
  A touch on a tool leaves the focus in the field.
- When the keyboard goes down, the edit ends and the item stays selected. This happens by the
  tool or by the button of the system. The app hears of the system button only through the
  room the keyboard takes (`KeyboardWatch` in `JournalScreen`).
  - While items are selected, the tools stay. The pad of the cursor becomes a pad of up and
    down, which selects the item above or below. The keyboard tool shows the keyboard again
    and edits the item.
  - The tools shrink to a round button at the bottom right. A touch makes them full again.
    They stay shrunk while an edit turns into a selection and back.
- The icons of the tools are drawn in `client/assets/icons`, the same for Android and iOS.
  `draw.py` there makes the SVG files, and `export.py` copies them into the two apps.
  - A pad takes touches on all of it, not only on its arrows. The lines from its corners cut
    it into the parts of the four arrows. The middle third does nothing. A touch that is
    held repeats the arrow.
  - The pad that moves the item: left moves it out of its parent, right under the item
    above, up and down past its neighbours.
  - The tool that hides the children shows them again when they are hidden. It cannot be
    touched on an item without children.
- A touch on an item edits it: the item becomes a field with its Markdown, with the cursor
  at its end. A touch on a link or an image still opens it. Only the rest of the row edits,
  so an item that is one long link is edited by a touch beside it. The core has the text
  and saves it a moment after the last key. Enter splits the item at the cursor, and the
  edit goes on in the new item. A touch on another item moves the edit there. The field
  takes no line break. It tells of the line break it could not put in a second time, and
  the screen does not take that for a second Enter.
- Backspace at the start of an item joins it to the item before. The edit goes on there,
  with the cursor where the two texts meet. An empty item goes away this way. Nothing is
  joined while text is chosen, or while the keyboard composes a word.
  - The keyboard on the screen sends no key for Backspace. It asks the field to delete the
    character before the cursor. At the start there is none: the field changes nothing and
    tells of nothing. So `ItemEditor` stands between the keyboard and the field
    (`InterceptPlatformTextInput`), and hears what the keyboard asks.
  - A keyboard with real keys sends the key, and `ItemEditor` hears that too.
- The core counts a cursor in characters, and a field of Android counts in UTF-16 units.
  The screen converts between the two. While the keyboard composes a word, as in Japanese,
  the core hears nothing, and the field is not written over.
- The core asks the shell to focus an editor and waits for the answer. `Shell` answers at
  once: the row that is edited takes the focus when it is drawn.
- The core asks before it does what is hard to undo, such as the delete of an item that
  another item transcludes. `Shell.question` holds the question, and the screen shows it in
  a dialog. A touch outside the dialog answers no. A second question answers the first
  with no.
- `JournalScreen` is the one screen: a bar at the top, and under it the results or the
  page. The bar holds the menu with "Sync" and "Sign out" at the left, the search field, and
  the button for the journal of today at the right. The bar stays while an item is edited.
  The menu takes no focus, so the keyboard of the item stays up when it opens.
- The app opens at the screen it showed last. `Shell` keeps the URL of that screen in the
  file `last-screen` of the data folder. A new app, and an app whose last page has gone,
  opens the journal of today.
- The bullet of an item is drawn, not a letter: the place of a letter in its line differs
  from font to font.
- `MainActivity` and `JournalModel` open the journal in the folders of the app:
  `files/markdown` for the Markdown files and `files/data` for the database, the index and
  the settings. They also start the sign-in and the sync. They do not know the Dropbox SDK.
  `JournalModel` knows the cloud storage only as `CloudAccount`, and takes its folders from
  outside. A test opens the model, and the real `MainActivity` over it, on folders of its own
  and with an account of its own. `viewWithAnotherApp` hands a file to another app.
- The sync with Dropbox is in Rust, in the core (`client-cloud-sync`, ADR 054): what to
  download, what to upload, the record of the versions, and one sync at a time. The iOS
  shell runs the same code.
- The package `sync` is what the app needs of the sync. It does not know the Dropbox SDK.
  - `CloudSync` keeps where the sync of the core stands, for the screen, and lets the core
    take in the files through the `Shell`. It reads the state from the core, not from what
    the core told on its thread: that may be old by the time it reaches the main thread.
  - `CloudAccount` is what the app needs of the account of the storage.
- The package `dropbox` holds all that uses the Dropbox SDK. No other package imports
  `com.dropbox`.
  - `DropboxAccount` is the `CloudAccount` of Dropbox. It keeps the credential, starts the
    sign-in, signs out, and gives the calls of the SDK that the sync makes.
  - Only two classes call the Dropbox SDK. `SdkDropbox` lists, downloads and uploads, as the
    `PetaDropboxSdk` of the core. It throws only a `PetaDropboxError`: `dropboxFailure` makes
    a failure with an `IOException` among its causes `Offline`, and the screen says that
    there is no connection. `RealDropboxSdk` signs in and revokes. They pass each call
    on and decide nothing. The other classes know them as the interfaces `PetaDropboxSdk`
    and `DropboxSdk`, so the tests put a Dropbox of their own there. No test runs the two
    classes: that needs the real Dropbox.

## Not there yet

- A sync while the app is not on screen.
- Links to a file outside the `assets` folder, such as a file of the PC.
- An image inside a cell of a table or inside a heading.
