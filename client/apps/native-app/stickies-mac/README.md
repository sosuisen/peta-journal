# SwiftUI frontend (stickies-mac)

The SwiftUI shell of PetaJournal for macOS. The app is named PetaStickies: each window is a
sticky that shows one page. It is a Walking Skeleton so far (ADR 024): a page is searched for
and opened in a sticky, its items are shown as an outline and can be edited, and the stickies
are kept between runs. The keys of the outline work as in the web frontend (see "Keys" below).

The core is the same Crux core as in the web and the Emacs frontends. `client/libs/client-native-bridge`
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
  `client/libs/client-shell-http/Cargo.toml`.
- [XcodeGen](https://github.com/yonaskolb/XcodeGen) 2.46.0. It makes the Xcode project of the
  app from `project.yml`:

  ```
  brew install xcodegen
  ```

## Build and test

```
cd peta-journal/client/apps/native-app/stickies-mac
./generate.sh     # makes generated/ from the Rust crates
swift test
```

`generate.sh` builds the Rust library optimized (`boltffi pack apple --release`; without the
flag BoltFFI makes a debug build, which is about twice as slow). It makes two Swift packages.
Run it again after a change in Rust. Neither is kept in the repository.

- `generated/swift/PetaTypes`: the types that cross the border, generated from the Rust types
  (`Event`, `ViewModel`, `Request`, ...).
- `generated/PetaCore`: the core as a static library (`PetaCore.xcframework`), and the Swift
  class `PetaCore` that calls it. BoltFFI makes both, from `client/libs/client-shell-http/src/ffi.rs`
  and `boltffi.toml` next to it.

`Sources/PetaBorder` is hand-written. It has no SwiftUI in it, so `swift test` can test it:

- `Border` puts the two packages together, so the rest of the shell works with the generated
  types and never with bytes.
- `Shell` is what one window shows. It keeps the view model up to date with the core and sends
  what the user did. It also holds what the core asks and waits for: a question to the user
  (`question`, shown as a dialog), an order to put the cursor in an item (`focus`), and the text
  on the clipboard.
- `Keys.swift` says what a key in the editor of an item means to the outline (`Shell.pressed`).
  The place of a cursor goes from the UTF-16 units of AppKit to the characters the core counts,
  and back, through `charsOfUtf16` and `utf16OfChars` of the Rust core (ADR 043). The core calls
  from its own threads; `Shell` moves each request to the main
  thread, in order.

- `ShownText` makes the text of an item as it is to be shown: with its decoration, not as
  Markdown. The core gives the parts of the item with each row (`Row.bodyModel`, ADR 042),
  and this makes text with attributes of the parts: a table in boxes of the text, an image of
  the assets folder as a picture in the text. An image whose page asks for no size is as wide
  as the sticky, and never wider than its file. Each link holds where the core says it leads.
- `AssetImages` reads the images of the assets folder from the journal server, which serves the
  folder at `/assets/`. A text that is made before its image has come shows the words for the
  image, and is made again when the image is there.
- `ShownTexts` keeps the texts that were made, for one screen. macOS makes such a text on the
  main thread only, and not while SwiftUI draws: a row asks for its text, shows the body of the
  item meanwhile, and is drawn again when the text is there.

## Format and lint

```
cd peta-journal/client/apps/native-app
./swift-format.sh           # formats the Swift of outliner-ios and stickies-mac, then lints it
./swift-format.sh --check   # changes nothing, and fails when there is something to change
```

The formatter and the linter are both swift-format, which comes with Xcode. Its settings are in
`client/apps/native-app/.swift-format`, one file for both apps: lines of 120 letters and an
indent of 4 spaces. Run it after every change in Swift. `generated/` is not formatted.

## Build and run the app

The app needs a running journal server (see `BUILD.md` at the top of the repository).

`release.sh --run` does all of the below: it builds the app (the Release build), ends a running
app, and starts the new one. Without `--run` it only builds. `debug.sh` does the same with the
Debug build, and always starts the app. What follows the options goes to the app:

```
cd peta-journal/client/apps/native-app/stickies-mac
./release.sh --run -url /pages/notes     # --generate first makes generated/ again, after a change in Rust
./debug.sh -url /pages/notes
./release.sh --help
```

By hand, the steps are:

```
cd peta-journal/client/apps/native-app/stickies-mac
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
decoration (ADR 041) and cannot be typed into. A click into an item makes it Markdown at once;
when the cursor leaves it, the shell tells the core that the edit is over (`Shell.finishEditing`),
as the web frontend does, and the item shows its decoration again. The shell tells the core how
many letters of the shown text stand before the click (`Shell.touched`). The core finds that
place in the Markdown, and the cursor goes there (ADR 043). The marks before the text
(the bullet, the box of a task) are drawn by the screen, not taken from the AST.

A click on a link of an item that shows its decoration follows it (`Shell.follow`). A wikilink
opens its page in the sticky, and a hashtag the page of the tag in the folder of the tags. With
Shift held, the page opens in the browser instead, in the web frontend at the address of the
journal server, and the sticky keeps its page. A link out of the journal, and a link to a file of
the assets folder, open in the browser. Such a click does not put the cursor in the item. A click
beside the links does, and the item shows its Markdown.

A journal (`/today`, `/journals/2026-11-11`) shows the days before below the day that was
opened. Above the first item of each earlier day there is a line and the date of that day. The
title bar shows the date of the day at the top of the window, with "(Today)" behind the date of
today. The screen tells the core which rows are drawn (`Shell.scrolled`); the core says which day
is at the top, and loads earlier days when the end comes near.

## The icon of the app

The icon is the notebook of `images/peta-journal.svg` on a white plate with rounded corners,
as the icons of macOS have. Its images are kept in the repository
(`App/Assets.xcassets/AppIcon.appiconset`), so a build does not make them. After a change of
the SVG, make them again:

```
./icon.sh
```

The script needs nothing but Rust. It runs the crate `client/tools/app-icon`, whose README
says how the icon is made.

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

A drag with the mouse selects items too. It starts in the text of an item and selects the
letters it goes over. Once it goes more than 10 points above or below the text, it selects the
item, and then every item it goes over, until the button comes up. Held within 40 points of
the top or the bottom of the window, it scrolls the items, faster nearer the edge, and the
selection takes the items that come under the mouse. The text view follows the mouse in its
own loop in `mouseDown` (`ItemTextView.follow`): a text view of AppKit keeps the mouse until
the button comes up and never tells that it left the text. The mouse may leave the window; the
item nearest to it that can be seen counts. The core decides this (`drag.rs` of
`client-core`), as for the Windows shell, and `ListDrag` of `PetaScreens` gives it the mouse
through `Shell.dragged`: it finds the item under the mouse by the row of the table of the
list, and scrolls the list as `EdgeScroll` says.
A double click, and a click with Shift or another key held, are left to AppKit.

While items are selected, no item has the cursor and the keys mean this:

| Key | What it does |
|---|---|
| Shift+Up, Down | makes the selection larger or smaller |
| Escape | ends the selection |
| Backspace, Delete | deletes the selected items |
| Shift+Option+Up, Down | moves the selected items up or down |
| Tab, Shift+Tab, Shift+Option+Right, Left | indents, outdents the selected items |
| Command+C, Command+X | copies, cuts the selected items as Markdown, and as HTML for other apps |
| Command+Z, Control+Z | undo; with Shift, redo |

A click into the text of an item also ends the selection.

A copy puts two things on the clipboard: the Markdown as plain text, and the same as HTML
(ADR 027). An app that takes rich text reads the HTML and keeps lists, bold and links. In the
HTML a wikilink and a hashtag are whole URLs that open the page in the web frontend, at the
address of the journal server (`-server`): this app has no address that another app opens.
`copiedHtml` of the core library makes the HTML.

An item whose body is one heading line (`## Title`) shows its Markdown at the size of its
heading, in bold, also while it is edited: the text keeps its size when the cursor goes in and
out. The sizes of the five levels are those of the web frontend (1.6, 1.4, 1.2, 1.1 and 1 times
the usual size). The bullet and the box of a task keep their size and stand in the middle of
the first line, however high it is.

The bullet of an item is a dot. While the children of the item are hidden, a pale circle lies
around the dot, as in the web frontend.

The bullet and the box of a task are drawn by the screen and are not letters: the three states of the box are of one size,
and both stand in the middle of the first line of the text. A click on it steps the state
of the task, as the key does: open, running, done, and then no task any more.

Each window is a sticky with a page of its own. Its title bar shows the name of the page, and
the tools of the sticky at the right. The search tool, the arrows of the history, and the buttons that
close, minimize and zoom the window, are hidden while the window does not have the focus: the
bar is left to the name.

The arrows, or Command+[ and Command+], go back to the page opened before in the sticky and
forward again, as in Safari and the Finder. Each sticky has a history of its own, which the
core keeps (ADR 035). An arrow is dimmed while there is nowhere to go. The history is not
kept between runs: the sticky comes back with the page it showed last.

The button with the plus, or Command+N, opens another sticky. The button is always there:
also while the window does not have the focus, and while the sticky is rolled up. One click
is enough on a window without the focus. The app ends when the last one is closed.

A right click on the title bar shows a menu with the pin and the colors. It is there also
while the sticky is rolled up. A right click on an item that is not edited shows the same
menu. The item that is edited has the menu of its text: cut, copy and paste. A click with the
Control key down is a right click here too. The pin and the color of the sticky are checked.

The menu gives the sticky another color: yellow, red, green, blue, orange,
purple, white, gray or light gray. They are the colors of the cards of Petasti.
A sticky is light in every color, also when the Mac is in dark mode.

A double click on the title bar rolls the sticky up, so that only its title bar is left, and
another one rolls it down again to the size it had. A double click on the name in the title
bar does the same: it does not zoom the window. In a quick row of clicks, every second click
rolls the sticky: the window is never zoomed. The window moves in steps, as the Stickies of
macOS do. A sticky that is rolled up shows no search tool, and no buttons that close, minimize or
zoom the window. A sticky that is rolled up comes back rolled up in the next run.

"Keep on Top" in the menu keeps the sticky on top of the windows of other apps, until it is
chosen again.

The zoom of the items of a sticky has keys only: Command+Plus makes them a tenth
larger, Command+Minus a tenth smaller, and Command+0 gives the usual size. The zoom stays
between 30% and 300%. Color, pin and zoom are kept with each sticky.

The button with the magnifying glass, or Command+F, opens the search box. What is typed there
is searched for a moment later. Down and Up choose an entry, Enter opens it, and Escape hides
the entries, then closes the box. The entry "Create" makes a page with the typed name.

## Tests

`swift test` runs `Tests/PetaBorderTests`.

`BorderTests` need no journal server. They check that Swift reads what Rust writes (the view
model, the requests) and that Rust reads what Swift writes (events, answers). `ShownTextTests`
and `AssetImagesTests` need none either.

`ShellTests` run `Shell` against a real journal server: a page opens, what is typed is saved on
the server, and the questions of the core are answered. The keys are pressed as the app
presses them: the chord goes to the core, which decides what it means. "Undo", "Redo" and
"Copy" are chosen as the Edit menu does. A question is asked by an extension of the server, and
another client changes the page through the API of the server.

`ShellTests`, `OutlineScreenTests`, `JournalScreenTests`, `StickyDeskTests` and
`TypingSpeedTests` need a server. They are skipped unless `PETA_E2E_SERVER` names the server
program:

```
cd peta-journal/journal
cargo build --release
cd ../client/apps/native-app/stickies-mac
PETA_E2E_SERVER=$PWD/../../../../journal/target/release/journal-server swift test
```

`OutlineScreenTests` put the screen in a window and give its editors the text and the keys as
AppKit does: typing, composing, Enter, the arrows, Tab and Backspace. No key of a real keyboard
is pressed, so they do not show that an input method works with the editors. A click on a link
is handed to the text view the way a window hands it over; the window of a test takes no click.

`TypingSpeedTests` measures a page of 1000 items, with the views in a real window: one key, and
with items that have decoration, a jump of one screen and the cursor going to another item. It
prints the times and fails on none of them. Run it optimized, and alone:

```
PETA_E2E_SERVER=$PWD/../../../../journal/target/release/journal-server \
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

`StickyDeskTests` open the stickies in windows: adding and closing them, what comes back at the
next start, the title bar, the colors, the menu of a sticky and the zoom.

`ShownTextTests` check the text that an item shows when it is not edited: its marks, images,
links and headings, and where a touch on it lands. `AssetImagesTests` check that the images of
the assets are read once.

The tests start the server themselves, with all its files in a temporary folder.
`OutlineScreenTests` use port 18094, `TypingSpeedTests` 18095, `ShellTests` 18097,
`StickyDeskTests` 18098 and `JournalScreenTests` 18100. The tests check that the server writes
its Markdown there before they use it. The notes of whoever runs the tests are never touched.

Why these tests matter: the Swift types are generated from the shape of the Rust types, not from
what serde writes. A Rust type with a hand-written `Serialize` can write other bytes than its
shape says. Then everything compiles, and the shell cannot read the bytes. Only a test that
crosses the border finds this.

These tests do not run in CI. A macOS runner costs much more than a Linux one, so this is put
off. Run them by hand after a change to a type that crosses the border. CI does run the Rust
tests of the entrance (`cargo test -p client-native-bridge -p client-shell-http --features client-native-bridge/facet_typegen,client-shell-http/ffi`).
