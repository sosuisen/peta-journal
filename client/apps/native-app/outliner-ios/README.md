# iOS frontend (outliner-ios)

The iOS shell of PetaJournal, written in Swift with SwiftUI. It does what the Android shell
does (`outliner-android`): it downloads the Markdown folder from Dropbox, a page is searched
for and opened, and its items are shown with their decoration. A touch on an item edits its
Markdown, and the sync uploads the pages that changed in the app.

The core is the same Crux core as in the other frontends. There is no journal server on a
phone, so the journal core runs inside the app (ADR 029). `client/libs/client-shell-embedded`
wraps both, and is packed here for iOS as it is for Android. Swift gets what changed on the
screen and the requests of the core as bytes (bincode), and sends events the same way. ADR 034 says what
the iOS shell takes from the Android shell and where it differs. ADR 030 and ADR 031 say how
the app syncs, where it keeps its data, how it edits and how it uploads.

## Prerequisites

- A Mac with Apple silicon, and Xcode with the iOS platform and a simulator (Xcode 27.2 and
  iOS 27.2 were used). The core is built for arm64 only.
- [XcodeGen](https://github.com/yonaskolb/XcodeGen), which makes the Xcode project.
- The Rust toolchain, with the targets `aarch64-apple-ios` and `aarch64-apple-ios-sim`.
- [BoltFFI](https://boltffi.dev) 0.32.1, the command line tool:

  ```
  cargo install boltffi_cli --version '=0.32.1' --locked
  ```

  The version has to be the same as the `boltffi` dependency in
  `client/libs/client-shell-embedded/Cargo.toml`.
- [swift-mutation-testing](https://github.com/ericodx/swift-mutation-testing) 1.7.1, for the
  mutation tests. Its tap has to be trusted once:

  ```
  brew tap ericodx/homebrew-tools
  brew trust ericodx/tools
  brew install swift-mutation-testing
  ```

## Build and test

```
cd peta-journal/client/apps/native-app/outliner-ios
./generate.sh        # makes generated/ from the Rust crates
swift test           # runs the tests that need no simulator, on the Mac
./debug.sh --test    # runs the tests of the screen in a simulator
./debug.sh           # builds the app (the Debug build) and starts it in a simulator
./release.sh         # builds the app (the Release build) for a simulator
```

`generate.sh` makes two Swift packages. Run it again after a change in Rust. Neither is kept in
the repository:

- `../swift-shared/generated/swift/PetaTypes`: the types that cross the border (`Event`,
  `ViewModel`, `Request`, ...), made by `facet_generate` 0.22.3. They are in the package that the
  two Swift apps share, because SwiftPM takes a package of one name from one place only.
- `generated/PetaCore`: the class `PetaCore` that calls the core, and the core as a library
  for a device, for the simulator and for the Mac (`PetaCore.xcframework`). BoltFFI makes
  both. The library for the Mac is there for `swift test`.

`debug.sh` makes `PetaJournal.xcodeproj` from `project.yml` with XcodeGen. The project is not
kept in the repository either. To run the app on a device, open the project in Xcode and
choose a team for signing.

## Format and lint

```
cd peta-journal/client/apps/native-app
./swift-format.sh           # formats the Swift of outliner-ios and stickies-mac, then lints it
./swift-format.sh --check   # changes nothing, and fails when there is something to change
```

The formatter and the linter are both swift-format, which comes with Xcode. Its settings are in
`client/apps/native-app/.swift-format`, one file for both apps: lines of 120 letters and an
indent of 4 spaces. Run it after every change in Swift. `generated/` is not formatted.

The mutation tests are `../mutation-test.sh outliner-ios`. Run them before each pull request.
They are told of in the README of `stickies-mac`.

## The icon of the app

The icon is the notebook of `images/peta-journal.svg` in the middle of a white square, as in
the Android shell. Its image is kept in the repository
(`App/Assets.xcassets/AppIcon.appiconset/icon-1024.png`), so a build does not make it.
After a change of the SVG, make the image again:

```
./icon.sh
```

The script needs nothing but Rust. It runs the crate `client/tools/app-icon`, whose README
says how the icon is made.

## Dropbox

The app reads and writes Dropbox with the App key of a Dropbox app, which each builder
registers at <https://www.dropbox.com/developers/apps>, as for the Android shell:

- Access type: Full Dropbox, to use the folder that the PC already uses.
- Permissions: `files.metadata.read`, `files.content.read` and `files.content.write`.
- No redirect URI is needed, and the App secret is not used: the sign-in is PKCE.

> [!IMPORTANT]
> Check no **Team Scope** on the "Permissions" tab of the Dropbox app (App Console), and
> press "Submit" after a change. One checked team scope stops the sign-in through the
> Dropbox app of the phone: after the account is chosen, it shows 「アプリの承認エラー」, or
> "Mobile authorization does not support team authorized scopes." This happened on an iPhone
> with `team_data.member` checked, and the sign-in worked after it was unchecked
> (2026-11-13). PetaJournal needs only the three scopes above, which are Individual Scopes.

The key and the path of the Markdown folder in Dropbox come from outside the repository, in
the file `Dropbox.xcconfig` of this folder:

```
DROPBOX_KEY = <App key>
DROPBOX_FOLDER = /PetaJournal
```

`debug.sh` makes an empty file when there is none. An app that is built without a key syncs
nothing and shows no sign-in.

The SDK of Dropbox signs in (SwiftyDropbox). It lets the Dropbox app sign in when the device
has it, and else opens a browser inside the app. It keeps the token in the keychain of the
device. The Android shell always uses the browser, after the Dropbox app failed there (ADR
033). The SDK for iOS has no such choice: it does not sign in at all in an app that may not
ask for the Dropbox app. The sign-in through the Dropbox app works on an iPhone (2026-11-13),
when the Dropbox app has no team scope: see the note above.

A new app shows only "Sign in to Dropbox" before the sign-in. It makes no journal of today:
Dropbox may hold that journal already, and the app would then have two files for one day.
The first sync after the sign-in opens the journal of today. An app that has shown a page
before opens that page also when it is signed out.

The app syncs when it comes to the front, when it goes to the back, when "Sync" of the menu is
touched, and 3 seconds after the last change of a page. What the README of the Android shell
says of the sync holds here too: the dialog of a sync in front, the callout of the others, the
three steps of a sync, what stays when Dropbox lost a file, and what "Sign out" does. iOS
gives an app that goes to the back about half a minute. When that is not enough, the rest goes
up when the app opens again.

The app keeps all its files in `Application Support/PetaJournal` of its container: the
Markdown folder (`markdown`), the database and the index of the core (`data`), where the last
download ended and the versions of the files in Dropbox. The folder is in no backup: it is a
copy of Dropbox.

## The code

`Package.swift` has the parts without a screen. They are tested on the Mac:

- The sync with Dropbox is in Rust, in the core (`client-cloud-sync`, ADR 054): what to
  download, what to upload, the record of the versions, and one sync at a time. The Android
  shell runs the same code.
- `PetaBorder` is the core behind its border, the sync, and what the app shows. `CloudSync`
  keeps where the sync of the core stands, for the screen, and lets the core take in the
  files through the `Shell`. `CloudAccount` is what the app needs of the account of the
  storage. `Border` is the core
  behind its border of bytes. `Shell` keeps the view model up to date with the core and sends
  what the user did. It makes the view model from the changes of the core (ADR 076) with
  `PetaShared` of `../swift-shared`, which the Mac app shares: the first changes hold every panel,
  and after them only the panels that changed come, with the changes to the rows of the
  outline, which are applied in order. The core has no call that gives the whole view model any
  more: the Rust tests of `client-native-bridge` see that the changes make the view model of the
  core. The parts of an item as it is shown come from the core with each row
  (`Row.bodyModel`, ADR 042). `RichLinks` makes the link of a run a URL, which a text of
  SwiftUI takes, and back again when it is touched.
- `PetaDropbox` is Dropbox, with the SDK. `DropboxAccount` is the sign-in and the sign-out, as
  a `CloudAccount`. It calls the SDK through a protocol (`DropboxSdk`), which a test fills with
  its own SDK. `SdkDropbox` gives the core the calls of the SDK that the sync makes (list,
  download, upload), as the `PetaDropboxSdk` of the core. `DropboxFailures` tells a failure
  for want of a connection from the others. The classes that call the SDK itself
  (`SdkDropbox`, `RealDropboxSdk`) only pass each call on.

`App` holds the app. It is built with Xcode only:

- `PetaJournalApp` starts the app, and syncs when the app comes to the front or goes to the
  back. `JournalModel` holds the shell and the sync, and the folders of the app. It knows the
  cloud only as a `CloudAccount`.
- `JournalScreen` is the one screen: the bar with the menu, the search field and the button
  for today, and under it the results of the search or the page. The button for today shows
  the icon of today of the web frontend, a book with a ribbon: `TodayIcon` draws it, with the
  lines of the Android shell. `SyncViews` shows where the sync stands: the menu, the dialog
  and the callout. `Tools`, the tools of the item that is edited or selected, stand above the
  keyboard. While items are selected, Copy and Cut put them on the clipboard as a Markdown
  list (`Shell.copySelected`, `Shell.cutSelected`).
- `Outline` draws the page, and `OutlineRow` one item of it. `Blocks` draws the parts of an
  item. `ImageBlock` reads an image smaller than
  it is in its file, and shows it large after a touch. A link to a file of the assets folder
  opens the file with Quick Look.
- `ItemEditor` is the field of the item that is edited. It is a `UITextView`: it tells where
  the cursor is when Enter is pressed, and whether the keyboard still composes a word. Text
  that is still being composed is not sent to the core, and the Enter that confirms it does
  not split the item. A paste goes to the core (`Shell.paste`), which makes items of a list;
  while text is chosen, the field puts the pasted text in its place.
- An image is pasted when the clipboard has no text: by "Paste" of the menu of the field, and by
  Command+V of a hardware keyboard. A sticker of the keyboard is pasted as an image too: the
  field takes no picture into its text, as the Markdown would keep only the character in its
  place (U+FFFC), and the keyboard then pastes the sticker. The field writes the image to a
  folder of its own in the temporary folder (`PastedImage`), with the name it came with, or
  `image` and the extension of its type. A HEIC, as a sticker or a photo comes, becomes a PNG
  when it has transparency and a JPEG when not: a browser may not show HEIC. The
  shell hands the path to the core (`Shell.pasteImage`), which saves the image in the assets
  folder of the Markdown, puts a link to it at the cursor, and deletes the file (ADR 071).
- Backspace at the start of an item joins it to the item before. The edit goes on there,
  with the cursor where the two texts meet. An empty item goes away this way. The field
  tells of no change then, so it hears the key itself (`deleteBackward`). Nothing is joined
  while text is chosen, or while the keyboard composes a word.
- After Enter, the edit moves to the field of the new item. That takes a moment: the core
  splits the item, and the screen gives the keyboard to the new field. What is typed
  meanwhile is kept and typed into the new field. `KeyboardKeeper` has the keyboard while
  neither field is on the screen. Without this, the first letters after Enter were lost now
  and then.

## Tests

`swift test` runs the tests of `PetaDropbox` and `PetaBorder` on the Mac. Those of
`PetaDropbox` use a Dropbox and an SDK of the test. The rules of the sync are tested in Rust
(`client-cloud-sync`). The tests of `Shell` and of `ShownText` run the real
core, with its files in a temporary folder.

`./debug.sh --test` runs, in the simulator, the tests of `JournalModel` (`AppTests`) inside the
app, with an account of the test, and then the tests of the screen (`UITests`). Each test of
the screen gives
the app a folder of its own, by `PETA_ROOT`. The app syncs with nothing then. They take
several minutes. `debug.sh` keeps the Mac awake meanwhile (`caffeinate`): on a Mac that goes to
sleep, the tests end with "Timed out while preparing execution worker".

Not tested: `SdkDropbox` and `RealDropboxSdk`, which need an account of Dropbox, and what
the screen shows of a sync (the menu, the dialog, the callout). The tests do not run in CI.
