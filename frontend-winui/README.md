# WinUI frontend (frontend-winui)

The WinUI shell of PetaJournal for Windows. The app is named PetaStickies: each window is a
sticky that shows one page. It does what the SwiftUI shell does (`frontend-swiftui`): a page is
searched for and opened in a sticky, its items are shown as an outline and can be edited, and
the stickies are kept between runs. The keys of the outline work as in the web frontend (see
"Keys" below).

The core is the same Crux core as in the web, the Emacs and the SwiftUI frontends.
`frontend-rust/crates/shell-native` wraps it for a shell that is not written in Rust. Calls to
the journal server, timers and the clock stay in Rust. C# gets the view model and the requests
of the screen as bytes (bincode), and sends events and answers the same way. ADR 028 says why
the shell is made this way.

## Prerequisites

- Windows 11 on x64. Only `win-x64` is built.
- The .NET SDK 10. `dotnet build` builds the app: Visual Studio is not needed.
- The Rust toolchain, with the target `x86_64-pc-windows-msvc`.
- [BoltFFI](https://boltffi.dev) 0.32.1, the command line tool. It makes the Rust library and
  the C# code that calls it:

  ```
  cargo install boltffi_cli --version '=0.32.1' --locked
  ```

  The version has to be the same as the `boltffi` dependency in
  `frontend-rust/crates/shell-native/Cargo.toml`.

## Build and test

```
cd peta-journal/frontend-winui
./generate.ps1     # makes generated/ from the Rust crates
dotnet test PetaBorder.Tests
```

`generate.sh` and `run.sh` do the same as the two `.ps1` scripts, for Git Bash.

`generate.ps1` builds the Rust library optimized (`boltffi pack csharp --release`). It makes
two C# projects under `generated/`. Run it again after a change in Rust. `generated/` is not
kept in the repository.

- `generated/csharp`: the types that cross the border, generated from the Rust types (`Event`,
  `ViewModel`, `Request`, ...), in the namespace `PetaTypes`. `facet_generate` 0.22.3 makes
  them. Crux brings an older version, whose C# does not compile (ADR 028).
- `generated/PetaCore`: the C# class `PetaCore` that calls the core, and the core as a library
  (`peta_shell_native.dll`). BoltFFI makes both, from
  `frontend-rust/crates/shell-native/src/ffi.rs` and `boltffi.toml` next to it.

`PetaBorder` is hand-written. It has no WinUI in it, so `dotnet test` can test it:

- `Border` puts the two projects together, so the rest of the shell works with the generated
  types and never with bytes.
- `Shell` is what one window shows. It keeps the view model up to date with the core and sends
  what the user did. It also holds what the core asks and waits for: a question to the user
  (`Question`, shown as a dialog), an order to put the cursor in an item (`Focus`), and the
  text on the clipboard. The core calls from its own threads; `Shell` moves each request to
  the main thread, in order.
- `Keys.cs` says what a key means: in the editor of an item (`Keyboard.InEditor`,
  `Shell.Pressed`) and while items are selected (`Keyboard.InSelection`). `Cursor` turns the
  place of a cursor from the UTF-16 units of WinUI into the characters the core counts, and
  back.
- `ShownText` makes the text of an item as it is to be shown: with its decoration, not as
  Markdown (ADR 025). It does not read the AST. `ItemHtml` of the core library makes HTML of
  the AST, with flatmarkdown-ast2html. Windows has nothing that turns HTML into text with
  decoration, so `ShownText` reads the HTML with AngleSharp and gives paragraphs of pieces
  with a style each. The screen draws them.
- `StickyStore` and `StickyLook` keep the stickies and how each one looks.

## Build and run the app

The app needs a running journal server (see `BUILD.md` at the top of the repository).

`run.ps1` builds the app (the Release build), ends a running app, and starts the new one. What
follows its own option goes to the app:

```
cd peta-journal/frontend-winui
./run.ps1 -url /pages/notes     # -Generate first makes generated/ again, after a change in Rust
./run.sh -url /pages/notes      # the same in Git Bash; --generate, --debug, --help
```

By hand, the steps are:

```
cd peta-journal/frontend-winui
./generate.ps1
dotnet build PetaStickies -c Release
PetaStickies/bin/Release/net10.0-windows10.0.22621.0/win-x64/PetaStickies.exe
```

The app is a folder with an `.exe` in it. It has no package, and the Windows App SDK lies
beside the `.exe`: nothing has to be installed to run it.

Without arguments the app shows the stickies of the run before, with their pages, places and
sizes, from the server at `http://localhost:8087`. A sticky that was closed does not come back.
The first time there is one sticky without a page: it shows the search box, to search for a
page. The stickies are kept in `%APPDATA%\PetaStickies\stickies.json`; `-stickies <file>`
names another file.

With `-url` the app shows only that page, and neither reads nor keeps stickies. To choose the
server and the page:

```
PetaStickies.exe -server http://localhost:8087 -url /pages/notes
```

Start the journal server before the app: the app does not load the page again when the server
comes up later.

`PetaStickies` holds the app. The views draw the view model of `Shell` and decide nothing about
the outline.

- `StickyDesk` holds the stickies that are open.
- `StickyWindow` is one sticky: its title bar with the tools, the search box, the list of the
  rows, the dialogs, and the keys of the window.
- `RowView` draws one item. The list makes views only for the rows near the window.
- `SearchBox` is the search box with what it found.

Only the item with the cursor shows its Markdown, in a text box. Every other item shows its
text with its decoration (ADR 025) and cannot be typed into. A click into an item makes it
Markdown at once; when the cursor leaves it, the shell tells the core that the edit is over
(`Shell.FinishEditing`), as the web frontend does, and the item shows its decoration again.
Text that is still being composed by an input method is not sent to the core until it is
confirmed, and the Enter that confirms it does not split the item.

A click on a link of an item that shows its decoration opens the link in the browser
(`Shell.Follow`), as in the SwiftUI shell. A wikilink and a hashtag open their page in the
web frontend, which the journal server serves: `WikilinkUrl` and `HashtagUrl` of the core
library make the address. The sticky keeps its page. Of the other links, only those to the
web and to a mail address are opened. Such a click does not put the cursor in the item.

Items that are copied go to the clipboard as Markdown and as HTML (ADR 027). `CopiedHtml`
of the core library makes the HTML.

A journal (`/today`, `/journals/2026-11-11`) shows the days before below the day that was
opened. Above the first item of each earlier day there is a line and the date of that day. The
title bar shows the date of the day at the top of the window, with "(Today)" behind the date
of today.

## Keys

The keys are those of the web frontend.

| Key | What it does |
|---|---|
| Enter | splits the item at the cursor |
| Shift+Enter | breaks the line inside the item |
| Ctrl+Enter | makes the item a task, then steps its state |
| Tab, Shift+Tab | indents, outdents |
| Up, Down on the first or last line | goes to the item before or after |
| Left at the start, Right at the end | goes to the end of the item before, the start of the item after |
| Backspace at the start | joins the item to the one before |
| Delete at the end | joins the next item to this one |
| Ctrl+Up | hides the children |
| Ctrl+Down | shows the children |
| Shift+Alt+Up, Down | moves the item up or down |
| Shift+Alt+Left, Right | outdents, indents |
| Ctrl+Z | undo; with Shift, redo |
| Shift+Up on the first line, Shift+Down on the last line | selects the item with its children |

While items are selected, no item has the cursor and the keys mean this:

| Key | What it does |
|---|---|
| Shift+Up, Down | makes the selection larger or smaller |
| Escape | ends the selection |
| Backspace, Delete | deletes the selected items |
| Shift+Alt+Up, Down | moves the selected items up or down |
| Tab, Shift+Tab, Shift+Alt+Right, Left | indents, outdents the selected items |
| Ctrl+C, Ctrl+X | copies, cuts the selected items as Markdown |
| Ctrl+Z | undo; with Shift, redo |

A click into the text of an item also ends the selection.

The keys of a sticky:

| Key | What it does |
|---|---|
| Ctrl+N | opens another sticky |
| Ctrl+F | opens or closes the search box |
| Ctrl+Plus, Ctrl+Minus, Ctrl+0 | makes the items a tenth larger, a tenth smaller, of the usual size |
| Ctrl+Q | ends the app and keeps all stickies |

Each window is a sticky with a page of its own. Its title bar shows the name of the page, and
the tools of the sticky at the right: search and a new sticky. The tools are
hidden while the window does not have the focus.

A right click on the title bar shows a menu with the pin and the colors. It is there also
while the sticky is rolled up. The pin and the color of the sticky are checked. The menu of
the window, which Windows shows there, comes with Alt+Space.

The buttons that minimize, maximize and close the window stand at the right of the tools. The
app draws them itself, as narrow as its tools: the buttons of Windows are wide, and leave
little of the title bar of a narrow sticky. The menu of snap layouts does not come under the
mouse on the maximize button. Windows+Z shows it.

A sticky that is closed with the button of its title bar is gone. A sticky whose window is
closed in any other way is kept and comes back in the next run: Alt+F4, the taskbar, or
Windows that ends the session. The app ends when the last window is closed. Ctrl+Q ends the
app with all its stickies kept, as Command+Q does on a Mac.

The menu gives the sticky another color: yellow, red, green, blue, orange,
purple, white, gray or light gray. They are the colors of the SwiftUI shell.
A sticky is light in every color, also when Windows is in dark mode.

A double click on the title bar rolls the sticky up, so that only its title bar is left, and
another one rolls it down again to the size it had. A sticky that is rolled up shows no tools
and no buttons, and comes back rolled up in the next run.

The button of the title bar maximizes the sticky. A double click on the title bar of a maximized sticky makes it as it was before. The
place and the size of a sticky are not kept while it is maximized: it comes back in the next
run as it was before.

"Keep on top" in the menu keeps the sticky on top of the windows of other apps, until it is
chosen again. The zoom stays between 30% and 300%. Color, pin and zoom are kept with each
sticky.

In the search box, what is typed is searched for a moment later. Down and Up choose an entry,
Enter opens it, and Escape hides the entries, then closes the box. The entry "Create" makes a
page with the typed name.

## Tests

`dotnet test PetaBorder.Tests` runs the tests of `PetaBorder`.

`BorderTests` need no journal server. They check that C# reads what Rust writes (the view
model, the requests) and that Rust reads what C# writes (events, answers). `KeysTests`,
`ShownTextTests` and `StickyTests` need none either.

`ShellTests` run `Shell` against a real journal server: a page opens, what is typed is saved
on the server, and the questions of the core are answered. They are skipped unless
`PETA_E2E_SERVER` names the server program:

```
cd peta-journal/journal-server-rust
cargo build --release
cd ../frontend-winui
$env:PETA_E2E_SERVER = "$PWD/../journal-server-rust/target/release/journal-server.exe"
dotnet test PetaBorder.Tests
```

The tests start the server themselves, on ports 18111 and 18112, with all its files in a
temporary folder. They check that the server writes its Markdown there before they use it, so
the notes of whoever runs the tests are never touched.

The windows have no tests. The SwiftUI shell puts its screen in a window in its tests; nothing
like that is written here yet. The app was checked by hand against a throwaway server
(2026-11-11): typing, Enter, Tab, a task, the decoration, links, selecting and deleting items,
the search box, a second sticky, the zoom, rolling up, a journal, and stickies that come back
in the next run. The user checked an input method for Japanese. Not checked: the dialogs of
the core, a click on a link, and Ctrl+Q in a sticky where nothing has the focus.

Why the tests of the border matter: the C# types are generated from the shape of the Rust
types, not from what serde writes. A Rust type with a hand-written `Serialize` can write other
bytes than its shape says. Then everything compiles, and the shell cannot read the bytes. Only
a test that crosses the border finds this.

These tests do not run in CI. Run them by hand after a change to a type that crosses the
border. CI does run the Rust tests of the entrance
(`cargo test -p peta-shell-native --features facet_typegen,ffi`).
