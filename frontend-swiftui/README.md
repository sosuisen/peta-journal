# SwiftUI frontend (frontend-swiftui)

The SwiftUI shell of PetaJournal for macOS. The app is named PetaStickies: each window is a
sticky that shows one page. It is a Walking Skeleton so far (ADR 024): a page is searched for
and opened in a sticky, its items are shown as an outline and can be edited, and the stickies
are kept between runs. The keys of the outline work as in the web frontend (see "Keys" below).

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

- `ShownText` makes the text of an item as it is to be shown: with its decoration, not as
  Markdown (ADR 025). It does not read the AST. `itemHtml` of the core library makes HTML of
  the AST, with flatmarkdown-ast2html, and macOS turns the HTML into text with attributes.
  `ShownText` puts text in the place of the three things macOS drops (the checkbox of a task
  list, the line between two parts, an image).
- `ShownTexts` keeps the texts that were made, for one screen. macOS makes such a text on the
  main thread only, and not while SwiftUI draws: a row asks for its text, shows the body of the
  item meanwhile, and is drawn again when the text is there.

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
xcodegen generate     # makes PetaStickies.xcodeproj from project.yml
xcodebuild -project PetaStickies.xcodeproj -scheme PetaStickies -derivedDataPath build build
open build/Build/Products/Debug/PetaStickies.app
```

This is the Debug build of the Swift code. It reads the view model about three times slower than
the Release build: add `-configuration Release` to judge the speed of the app (the app is then
under `build/Build/Products/Release`).

Or open `PetaStickies.xcodeproj` in Xcode and run it there. The project is not kept in the
repository: run `xcodegen generate` again after a change in `project.yml` or a new file in `App/`.

Without arguments the app shows the stickies of the run before, with their pages, places and
sizes, from the server at `http://localhost:8087`. A sticky that was closed does not come back.
The first time there is one sticky without a page: it shows the search box, to search for a
page. The stickies are kept in `~/Library/Application Support/PetaStickies/stickies.json`;
`-stickies <file>` names another file.

With `-url` the app shows only that page, and neither reads nor keeps stickies. To choose the
server and the page:

```
open build/Build/Products/Debug/PetaStickies.app --args -server http://localhost:8087 -url /pages/notes
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

Only the item with the cursor shows its Markdown. Every other item shows its text with its
decoration (ADR 025) and cannot be typed into. A click into an item makes it Markdown at once;
when the cursor leaves it, the shell tells the core that the edit is over (`Shell.finishEditing`),
as the web frontend does, and the item shows its decoration again. The marks before the text
(the bullet, the box of a task) are drawn by the screen, not taken from the HTML.

A click on a link of an item that shows its decoration opens what the link names
(`Shell.follow`): a wikilink opens its page in that sticky, a hashtag the page of the tag in the
folder of the tags, and any other link opens in the browser. Such a click does not put the
cursor in the item. A click beside the links does, and the item shows its Markdown.

A journal (`/today`, `/journals/2026-11-11`) shows the days before below the day that was
opened. Above the first item of each earlier day there is a line and the date of that day. The
title bar shows the date of the day at the top of the window, with "(Today)" behind the date of
today. The screen tells the core which rows are drawn (`Shell.scrolled`); the core says which day
is at the top, and loads earlier days when the end comes near.

## Keys

The keys are those of the web frontend, but for folding: there the Mac has Command where
Windows has Control. Control in this table is the Control key.

| Key | What it does |
|---|---|
| Enter | splits the item at the cursor |
| Shift+Enter | breaks the line inside the item |
| Command+Enter, Control+Enter | makes the item a task, then steps its state |
| Tab, Shift+Tab | indents, outdents |
| Up, Down on the first or last line | goes to the item before or after |
| Left at the start, Right at the end | goes to the end of the item before, the start of the item after |
| Backspace at the start | joins the item to the one before |
| Delete at the end (fn+Delete) | joins the next item to this one |
| Command+Up | hides the children |
| Command+Down | shows the children |
| Shift+Option+Up, Down | moves the item up or down |
| Shift+Option+Left, Right | outdents, indents |
| Command+Z, Control+Z | undo; with Shift, redo |
| Shift+Up on the first line, Shift+Down on the last line | selects the item with its children |

While items are selected, no item has the cursor and the keys mean this:

| Key | What it does |
|---|---|
| Shift+Up, Down | makes the selection larger or smaller |
| Escape | ends the selection |
| Backspace, Delete | deletes the selected items |
| Shift+Option+Up, Down | moves the selected items up or down |
| Tab, Shift+Tab, Shift+Option+Right, Left | indents, outdents the selected items |
| Command+C, Command+X | copies, cuts the selected items as Markdown |
| Command+Z, Control+Z | undo; with Shift, redo |

A click into the text of an item also ends the selection.

The box of a task is drawn by the screen and is not a letter: the three states are of one size,
and the box stands in the middle of the first line of the text. A click on it steps the state
of the task, as the key does: open, running, done, and then no task any more.

Each window is a sticky with a page of its own. Its title bar shows the name of the page, and
the tools of the sticky at the right. The tools are hidden while the window does not have the
focus. The button with the plus, or Command+N, opens
another sticky. The app ends when the last one is closed.

The menu with the palette gives the sticky another color: yellow, pink, green, blue or white.
A sticky is light in every color, also when the Mac is in dark mode.

A double click on the title bar rolls the sticky up, so that only its title bar is left, and
another one rolls it down again to the size it had. The window moves in steps, as the Stickies of macOS do. A sticky that is rolled up shows no tools. A sticky that is rolled up comes back
rolled up in the next run.

The button with the pin keeps the sticky on top of the windows of other apps, until it is
pressed again.

The zoom of the items of a sticky has keys only: Command+Plus makes them a tenth
larger, Command+Minus a tenth smaller, and Command+0 gives the usual size. The zoom stays
between 30% and 300%. Color, pin and zoom are kept with each sticky.

The button with the magnifying glass, or Command+F, opens the search box. What is typed there
is searched for a moment later. Down and Up choose an entry, Enter opens it, and Escape hides
the entries, then closes the box. The entry "Create" makes a page with the typed name.

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
is pressed, so they do not show that an input method works with the editors. A click on a link
is handed to the text view the way a window hands it over; the window of a test takes no click.

`TypingSpeedTests` measures a page of 1000 items, with the views in a real window: one key, and
with items that have decoration, a jump of one screen and the cursor going to another item. It
prints the times and fails on none of them. Run it optimized, and alone:

```
PETA_E2E_SERVER=$PWD/../journal-server-rust/target/release/journal-server \
  swift test -c release -Xswiftc -enable-testing --filter TypingSpeed
```

On an M1 MacBook Air (2026-11-11): the event into the core 0.07 ms, the view model out of the
core and read in Swift 4.9 ms, the window drawn again 4.2 ms.

With decoration (ADR 025), same day: the text of one item is made in 2.5 ms. After a jump of one
screen (26 new rows) the rows are drawn with their Markdown after 55 ms, the texts are then made
in one go that holds the main thread for 72 ms, and the rows show their decoration after 178 ms.
So a jump shows Markdown for about a tenth of a second. Scrolling line by line makes one or two
texts at a time. The cursor going to another item takes 16 ms in the shell and the core and
27 ms to draw; about 10 ms of that is for drawing all rows in view again.

`JournalScreenTests` put a journal of two days in a window and scroll it.

The tests start the server themselves, on ports 18094, 18095, 18097 to 18100, with all its files in a temporary folder.
They check that the server writes its Markdown there before they use it, so the notes of whoever
runs the tests are never touched.

Why these tests matter: the Swift types are generated from the shape of the Rust types, not from
what serde writes. A Rust type with a hand-written `Serialize` can write other bytes than its
shape says. Then everything compiles, and the shell cannot read the bytes. Only a test that
crosses the border finds this.

These tests do not run in CI. A macOS runner costs much more than a Linux one, so this is put
off. Run them by hand after a change to a type that crosses the border. CI does run the Rust
tests of the entrance (`cargo test -p peta-shell-native --features facet_typegen,ffi`).
