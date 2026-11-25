# WinUI frontend (stickies-win)

The WinUI shell of PetaJournal for Windows. The app is named PetaStickies: each window is a
sticky that shows one page. It does what the SwiftUI shell does (`stickies-mac`): a page is
searched for and opened in a sticky, its items are shown as an outline and can be edited, and
the stickies are kept between runs. The keys of the outline work as in the web frontend (see
"Keys" below).

The core is the same Crux core as in the web, the Emacs and the SwiftUI frontends.
`client/libs/client-native-bridge` wraps it for a shell that is not written in Rust. Calls to
the journal server, timers and the clock stay in Rust. C# gets what changed on the screen (ADR 076) and
the requests of the core as bytes (bincode), and sends events and answers the same way. ADR 028 says why
the shell is made this way.

## Prerequisites

- Windows 11 on x64. Only `win-x64` is built.
- The .NET SDK 11. `dotnet build` builds the app: Visual Studio is not needed.
- The Rust toolchain, with the target `x86_64-pc-windows-msvc`.
- [BoltFFI](https://boltffi.dev) 0.32.1, the command line tool. It makes the Rust library and
  the C# code that calls it:

  ```
  cargo install boltffi_cli --version '=0.32.1' --locked
  ```

  The version has to be the same as the `boltffi` dependency in
  `client/libs/client-shell-http/Cargo.toml`.

## Build and test

```
cd peta-journal/client/apps/native-app/stickies-win
./generate.ps1     # makes generated/ from the Rust crates
dotnet test PetaBorder.Tests
```

`generate.sh` does the same as `generate.ps1`, for Git Bash. It runs on a Mac too (see "Tests
on a Mac" below).

`generate.ps1` builds the Rust library optimized (`boltffi pack csharp --release`). It makes
two C# projects under `generated/`. Run it again after a change in Rust. `generated/` is not
kept in the repository.

- `generated/csharp`: the types that cross the border, generated from the Rust types (`Event`,
  `ViewModel`, `Request`, ...), in the namespace `PetaTypes`. `facet_generate` 0.22.3 makes
  them. Crux brings an older version, whose C# does not compile (ADR 028).
- `generated/PetaCore`: the C# class `PetaCore` that calls the core, and the core as a library
  (`client_shell_http.dll`). BoltFFI makes both, from
  `client/libs/client-shell-http/src/ffi.rs` and `boltffi.toml` next to it.

`PetaBorder` is hand-written. It has no WinUI in it, so `dotnet test` can test it:

- `Border` puts the two projects together, so the rest of the shell works with the generated
  types and never with bytes.
- `Shell` is what one window shows. It keeps the view model up to date with the core and sends
  what the user did. It also holds what the core asks and waits for: a question to the user
  (`Question`, shown as a dialog), an order to put the cursor in an item (`Focus`), and the
  text on the clipboard. The core calls from its own threads; `Shell` moves each request to
  the main thread, in order.
- `Keys.cs` hands a key to the core (`Shell.Pressed`), which decides what it means in the
  editor of an item and while items are selected (ADR 052). The app only names the key and
  the keys held with it (`Input.ChordOf`), and measures the lines of the editor. `Cursor` turns the
  place of a cursor from the UTF-16 units of WinUI into the characters the core counts, and
  back.
- `ShownText` makes the text of an item as it is to be shown: with its decoration, not as
  Markdown. It reads the parts that the core makes of each body (`Row.BodyModel`,
  ADR 042) and gives the parts that the screen draws: paragraphs of pieces with a style
  each, tables and images. `RichBlock`, `RichLink` and `ShownPart` are `closed record`s
  (C# 15, .NET 11), and CS8509 is an error: a `switch` expression that leaves out a kind of
  part stops the build. The generator writes `abstract record`; `codegen csharp` makes it
  `closed`.
- `StickyStore` and `StickyLook` keep the stickies and how each one looks. Their rules (the
  file, the colors, the zoom, the sizes of the window) are in the Rust crate `client-stickies`,
  shared with the stickies of macOS; `StickyStore` and `StickyLook` call it through `PetaCore`.

## Build and run the app

The app needs a running journal server (see `BUILD.md` at the top of the repository).

`release.ps1 -Run` builds the app (the Release build), ends a running app, and starts the new
one. Without `-Run` it only builds. `debug.ps1` does the same with the Debug build, and always
starts the app. What follows the options goes to the app:

```
cd peta-journal/client/apps/native-app/stickies-win
./release.ps1 -Run -url /pages/notes     # -Generate first makes generated/ again, after a change in Rust
./debug.ps1 -url /pages/notes
./release.sh -Run -url /pages/notes      # the same in Git Bash, with the same options; also debug.sh
```

By hand, the steps are:

```
cd peta-journal/client/apps/native-app/stickies-win
./generate.ps1
dotnet build PetaStickies -c Release
PetaStickies/bin/Release/net11.0-windows10.0.22621.0/win-x64/PetaStickies.exe
```

The app is a folder with an `.exe` in it. It has no package, and the Windows App SDK lies
beside the `.exe`: nothing has to be installed to run it.

The icon of the app is `PetaStickies/PetaStickies.ico`. `icon.ps1` makes it from
`images/peta-journal.svg` at the top of the repository: the notebook in the middle of a clear
square. The icon is kept in the repository, so a build does not run the script. Run it again
after a change of the SVG. `icon.sh` does the same in Git Bash.

The script needs nothing but Rust. It runs the crate `client/tools/app-icon`, whose README
says how the icon is made. The `.exe` carries the icon (`ApplicationIcon`), and so does each
window (`AppWindow.SetIcon`): Alt+Tab shows the icon of the window, not that of the `.exe`.

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

The journal server may start after the app: the core keeps trying to connect, at most 30 seconds
apart, and loads the page once the server is up.

`PetaStickies` holds the app: its start, its arguments, its resources and its icon. The windows
are in `PetaScreens`, a WinUI class library, so that the tests of `PetaScreens.Tests` can open
them (ADR 059). Only `StickyDesk` is public; the tests reach the rest through
`InternalsVisibleTo`. The views draw the view model of `Shell` and decide nothing about the
outline.

- `StickyDesk` holds the stickies that are open.
- `StickyWindow` is one sticky: its title bar with the tools, the search box, the list of the
  rows, the dialogs, and the keys of the window.
- `RowView` draws one item. The list makes views only for the rows near the window.
- `SearchBox` is the search box with what it found.

Only the item with the cursor shows its Markdown, in a text box. Every other item shows its
text with its decoration (ADR 041) and cannot be typed into. A click into an item makes it
Markdown at once; when the cursor leaves it, the shell tells the core that the edit is over
(`Shell.FinishEditing`), as the web frontend does, and the item shows its decoration again.
The shell tells the core how many letters of the shown text stand before the click
(`Shell.Touched`). The core finds that place in the Markdown, and the cursor goes there
(ADR 043). The mark of an entry of a list and the line of a rule are not letters of the core.
A click beside the letters puts the cursor at the end.
Text that is still being composed by an input method is not sent to the core until it is
confirmed, and the Enter that confirms it does not split the item.

A click on a link of an item that shows its decoration follows the link (`Shell.Follow`).
The core says where each link leads (`RichLink`). A page, such as that of a wikilink or a
hashtag, opens in the sticky. With Control held, it opens in a new sticky. With Shift held,
it opens in the web frontend in the browser. The journal server serves that frontend. A file
of the assets folder opens in the browser, as the journal server serves it (`/assets/...`).
Of the other links, only those to the web are opened. Such a click does not put the cursor
in the item.

An image of the assets folder is drawn from the journal server, at the size that the page
asks for. An image whose page asks for no size fits the width of the sticky, and is never
larger than its file. Its words stand there until its file is here. A table has a line
around each cell, and its columns share the width of the sticky.

Raised and lowered letters are 0.75 times as large as the letters of their paragraph, as in
the other shells. `-# subtext` is lowered too. Unlike the other shells, they stay on the
baseline: they are only smaller, not higher or lower. A WinUI `Run` has no setting that moves
its letters off the baseline. The OpenType variants (`Typography.Variants`) do not help
either. Segoe UI and Yu Gothic UI change only the digits, and Noto Sans JP has no such
variants. The other shells do not use them either. They make the letters smaller and move
them themselves: `baselineOffset` on macOS and iOS, `BaselineShift` on Android, and the
`<sub>` and `<sup>` of the browser on the web. An `InlineUIContainer` could move the letters, but it was not chosen. Its letters
are not a `Run`, so a click on them would not find its place in the Markdown. It cannot stand
in a `Hyperlink`, and a `TextHighlighter` does not reach it. In a subtext the whole line is
lowered, so the baseline would not change its look anyway.

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
| Ctrl+O | opens the page of the wikilink or the hashtag at the cursor |
| Shift+Alt+Up, Down | moves the item up or down |
| Shift+Alt+Left, Right | outdents, indents |
| Ctrl+Z | undo; with Shift, redo |
| Ctrl+V | pastes; a list becomes items |
| Ctrl+Shift+V | pastes the text into the item as it is |
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

The mouse selects items too, as in the web frontend. A drag that starts on the text of an item
selects its letters. Once the mouse leaves that text by more than 10px up or down, the drag
selects the item instead, and every item the mouse goes over joins the selection. Near the top
or the bottom edge of the list, the list scrolls: the nearer the edge, the faster. The mouse
may leave the window; the item nearest to it counts. The core decides this (`drag.rs` of
`client-core`); `StickyWindow` gives it the mouse through `Shell.Dragged`.

The keys of a sticky:

| Key | What it does |
|---|---|
| Ctrl+N | opens another sticky |
| Ctrl+F | opens or closes the search box |
| Alt+Left, Alt+Right | goes back to the page opened before, and forward again |
| Ctrl+Plus, Ctrl+Minus, Ctrl+0 | makes the items a tenth larger, a tenth smaller, of the usual size |
| Ctrl+Q | ends the app and keeps all stickies |

Each window is a sticky with a page of its own. Its title bar shows the name of the page, and
the arrows of the history and the search tool at the right. The tools and the buttons of the
window are hidden while the window does not have the focus: the bar is left to the name.

The arrows, or Alt+Left and Alt+Right, go back to the page opened before in the sticky and
forward again, as in the browsers and the File Explorer. Each sticky has a history of its own,
which the core keeps (ADR 035). An arrow is dimmed while there is nowhere to go. The history
is not kept between runs: the sticky comes back with the page it showed last.

The button with the plus stands at the left of the name, and opens another sticky. It is
always there: also while the window does not have the focus, and while the sticky is rolled
up. It answers when it is pressed.

A right click on the title bar shows a menu with the pin and the colors. It is there also
while the sticky is rolled up. A right click on an item that is not edited shows the same
menu, with "Paste as Transclusion" on top. It makes the item show the item whose id is on the
clipboard. The item that is edited has the menu of its text: cut, copy and paste. The pin and the color of the sticky are checked. The menu of
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
After the colors, the menu offers "Transparent": a sticky without paper, which shows the
windows behind it. While another window has the focus, it shows only its text. The line
around the window, its shadow, the name and the button of its title bar, the bullets, and the
scroll bar are not seen then. Its edges cannot be dragged then: a click gives it the focus
first. They keep their place, so the text does not move when they come back. The SwiftUI shell has no
transparent sticky yet.

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

`dotnet test PetaBorder.Tests` runs the tests of `PetaBorder`. `PetaScreens.Tests` tests the
windows (see "Tests of the windows" below).

`AppliedTests` need no journal server. They check that the changes of the core make the view
model: the first changes make every part, a panel that comes takes the place of its part only,
and the outline panel keeps the rows.

`BorderTests` need no journal server. They check that C# reads what Rust writes (the view
model, the requests) and that Rust reads what C# writes (events, answers). `StickyTests` need
none either. `ShownTextTests` need none but one: the test of every kind of node gets the AST
from a real journal server.

`ShellTests` run `Shell` against a real journal server: a page opens, what is typed is saved
on the server, and the questions of the core are answered. The keys are pressed as the app
presses them: the chord goes to the core, which decides what it means. The tests that need a
server are skipped unless `PETA_E2E_SERVER` names the server program:

```
cd peta-journal/journal
cargo build --release
cd ../client/apps/native-app/stickies-win
$env:PETA_E2E_SERVER = "$PWD/../../../../journal/target/release/journal-server.exe"
dotnet test PetaBorder.Tests
```

The tests start the server themselves, with all its files in a temporary folder.
`ShellTests` use port 18111, and `ShownTextTests` use port 18112. The tests check that the
server writes its Markdown there before they use it. The notes of whoever runs the tests are
never touched.

### Tests on a Mac

`PetaBorder` and its tests have no WinUI in them, so they run on a Mac too. `PetaScreens` and
the app do not. On a Mac, `generate.sh` makes the core for the Mac
(`libclient_shell_http.dylib`): `boltffi.toml` makes it for the system that runs it.

```
cd peta-journal/client/apps/native-app/stickies-win
./generate.sh
export PETA_E2E_SERVER="$PWD/../../../../journal/target/release/journal-server"
dotnet test PetaBorder.Tests
```

Four tests of `ShellTests` are skipped on a Mac: those of a key with Control (folding, copying).
The core reads the keys of the system it was built for, and a Mac has Command where Windows has
Control. They run on Windows.

The hook before a pull request does not run them on a Mac. Every step of the job `stickies-win`
in `scripts/checks.mjs` is for Windows, as the developer decided (#1085): the app runs on Windows
only. On a Mac, the hook skips the job and says so.

The hook does not run the tests of the windows below either, as they press real keys (ADR 086).
They are in the coverage step, which has `screen: true`. The CI runs them, and so does
`node scripts/checks.mjs run stickies-win` run by hand.

### Tests of the windows

`PetaScreens.Tests` opens real stickies and works them as a user does (ADR 059). It is a WinUI
app without a package, on MSTest. `TestAppStart` starts `TestApp` on a thread of its own before
the tests, and MSTest runs each `[UITestMethod]` on that thread. The thread is STA, as the
thread of the app is. The clipboard of Windows fails on an MTA thread, and MSTest makes one
when it starts the app itself (`[WinUITestTarget]`, ADR 067). The keys and the clicks go through Windows (`SendInput`), not into the controls.
WinUI makes no key events for a test, and a key that went around Windows would miss what the
app does with it.

- `OutlineScreenTests`: a click puts the cursor in an item, and what is typed reaches the core.
  Enter, Shift+Enter, Ctrl+Enter, Tab, the arrows, Backspace, Delete, the moves with Shift+Alt,
  the children hidden with Ctrl+Up, Ctrl+Z, the selection with Shift+Up, Ctrl+C, Ctrl+V,
  Ctrl+Shift+V, and the search box. The tests of the clipboard use the real clipboard of
  Windows: what was on it before is lost.
- `StickyDeskTests`: Ctrl+N adds a sticky, the close button puts it away, and Alt+F4 and
  Ctrl+Q keep the stickies for the next run.

They need a real journal server, as `ShellTests` do. Run the program of the tests itself:
`dotnet test` does not show what it prints until the end, and hangs in a pipe.

```
dotnet build PetaScreens.Tests
$env:PETA_E2E_SERVER = "$PWD/../../../../journal/target/release/journal-server.exe"
PetaScreens.Tests/bin/Debug/net11.0-windows10.0.22621.0/win-x64/PetaScreens.Tests.exe
```

`OutlineScreenTests` use port 18113, and `StickyDeskTests` use port 18114. The stickies are kept
in a file in a temporary folder, not in `%APPDATA%`.

Do not use the keyboard or the mouse while the tests run. The keys go to the window in front,
and a sticky that loses the front loses the keys. The tests take about a minute. A wait that
fails tells what the test saw: the focus, the rows of the core, and where the focus went.

Not tested: the dialogs of the core, a click on a link, the zoom, and rolling up.
`StickyWindow` calls the real browser, and a test cannot catch it.
The user checked an input method for Japanese by hand.

Why the tests of the border matter: the C# types are generated from the shape of the Rust
types, not from what serde writes. A Rust type with a hand-written `Serialize` can write other
bytes than its shape says. Then everything compiles, and the shell cannot read the bytes. Only
a test that crosses the border finds this.

The CI runs these tests on Windows, in the job `stickies-win`, with a journal server. It also
runs the Rust tests of the entrance
(`cargo test -p client-native-bridge -p client-shell-http --features client-native-bridge/facet_typegen,client-shell-http/ffi`).
The tests of the windows (`PetaScreens.Tests`) press real keys. They run in the coverage step
below, on the Windows runner of the CI too. The job builds the app (`PetaStickies`) as well.

### Coverage

```
dotnet tool restore
cd PetaBorder.Tests
dotnet build -t:Coverage
```

It measures the whole app with coverlet.console, a local tool of `stickies-win`. First it runs
the tests of the windows (`PetaScreens.Tests`), then the tests of `PetaBorder`, and adds up the
lines that both reached in `PetaBorder` and `PetaScreens`. It prints a table, and writes
`PetaBorder.Tests/coverage/coverage.cobertura.xml`. It fails when less than 90% of those lines
are covered, and the CI runs it. Set `PETA_E2E_SERVER` first: a skipped test reaches no line. A
line no test reaches is either removed, or given a test of the behaviour that needs it.

Keep the keyboard and the mouse still while it runs: the tests of the windows press real keys.

The files left out of the measurement, and why, are in the comment of `PetaBorder.Tests.csproj`.

### Mutation testing

Run the mutation test of `PetaBorder` before each PR that changes it (#782):

```
$env:PETA_E2E_SERVER = "$PWD/../../../../journal/target/release/journal-server.exe"
./mutation.ps1
```

The script runs Stryker.NET (the tool `dotnet-stryker` in `dotnet-tools.json`) on
`PetaBorder`, with the tests of `PetaBorder.Tests`. The report is in
`PetaBorder.Tests/StrykerOutput/<time>/reports/mutation-report.html`. A run takes about three
minutes. The arguments of the script go to Stryker.NET.

`stryker-config.json` sets two things:

- `"coverage-analysis": "all"`: every test runs against every mutant that some test reaches.
  The default (`perTest`) runs only the tests that reached the mutant. But it sees only what
  runs on the thread of a test. The requests of the core come on its own threads, and wait
  for the main thread. With `perTest`, about 40 mutants there were taken as not tested,
  though tests reach them.
- `"ignore-methods"`: the mutants that remove `ArgumentNullException.ThrowIfNull` are not
  made. Only a test that passes null finds them. Such a test is written only where null can
  really come in.

The tests of the windows (`PetaScreens.Tests`) are not used: they send real input, and Stryker
would run them for each mutant. So the mutants of `PetaScreens` are not tested.

The script sets `PetaMutation`, and `Directory.Build.props` then allows the preview features
of C# and makes CS8509 a warning. Stryker.NET builds the mutants with a Roslyn of its own,
older than the one of the .NET SDK. In that Roslyn, the closed records of C# 15 are a preview
feature. When Stryker.NET has a Roslyn that knows them, the preview is not needed any more.
