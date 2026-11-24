# outliner-emacs

English | [日本語](README.ja.md)

The Emacs frontend of PetaJournal. It has two parts:

- `outliner-emacs` is a Rust program. It holds the Crux core (`client-core`) and performs its
  effects: HTTP, the event stream and timers.
- `lisp/peta-journal.el` is an Emacs mode. It draws the rows it is sent and reports what the user
  does.

They talk in JSON lines over stdin and stdout. `src/protocol.rs` defines the messages.
`lisp/peta-journal-protocol.el` holds the names of the messages for Lisp. It is written from the
Rust types, so do not edit it by hand. After a change to the types, write it again:

```powershell
$env:PETA_WRITE_LISP = "1"; cargo test -p outliner-emacs the_committed_lisp_protocol
```

## Setup

Build the program:

```powershell
cd peta-journal/client
cargo build --release -p outliner-emacs
```

`release.ps1` in this folder does the same, and `release.sh` on macOS and Linux. `debug.ps1`
and `debug.sh` build the debug build and start Emacs with today's journal, for development.

Add this to your Emacs init file:

```elisp
(add-to-list 'load-path "<peta-journal>/client/apps/emacs/outliner/lisp")
(autoload 'peta-journal "peta-journal" nil t)
```

Then run `M-x peta-journal` while the journal server is running. It opens today's journal.

The mode looks for the program on the `exec-path` first, then in `target/release` and
`target/debug`. To use another one, set `peta-journal-shell-program`. Emacs 29.1 or newer is
needed.

### After a new build

Close every PetaJournal buffer with `C-x k` and run `M-x peta-journal` again. A running program
stays the old one until its buffer is closed.

If a copy of the program is on the `exec-path`, such as in `~/.local/bin`, the mode uses that copy
first. The old copy keeps running after a build, so when a new feature does not show, suspect
this.

### Building on an external disk

If `client/target` is a symbolic link to an external disk, the mode cannot find the program while
the disk is out. Put only the intermediate files of the build on the external disk: set
`build.build-dir`, which Cargo has had since 1.91, in an environment variable or in
`~/.cargo/config.toml`.

```sh
export CARGO_BUILD_BUILD_DIR='/Volumes/<disk>/rust-target/build/{workspace-path-hash}'
```

The program stays in `target/release`, so the mode finds it while the disk is out.

### Using another server

The mode connects to `http://localhost:8087` by default. To use another server, add this to your
Emacs init file, with the address of that server:

```elisp
(setq peta-journal-server "http://localhost:18087")
```

To change it for the current session only, press `M-:` and enter the same expression.

The setting takes effect only when a shell starts. If PetaJournal is already open, kill each of its
buffers with `C-x k`. This also stops their shells. Then run `M-x peta-journal` again.

If a shell stops by itself, Emacs shows a warning and `[shell stopped]` in the mode line. Edits in
that buffer are not saved until the shell runs again. Run `M-x peta-journal` in that buffer to start
it. This shows the saved text, so copy what you typed after the warning first. Errors of the shell
are in `*peta-journal-log*`.

## Editing

The whole page is in one buffer. Type in a body as in any other buffer. Typing `[[` adds `]]`, and
typing `]]` then goes over them. Typing three backticks at the start of a line adds the closing code
fence. The bullets and the line
ends between items are read-only. The outline changes only through these keys:

| Key | Action |
|---|---|
| `RET` | Split the item, or add an item below it |
| `S-RET`, `C-j` | Add a line in the item |
| `TAB`, `M-S-<right>` | Indent |
| `<backtab>`, `M-S-<left>` | Outdent |
| `M-S-<up>`, `M-S-<down>` | Move the item up or down |
| `DEL` at the start of a body | Join the item to the one above |
| `C-d` at the end of a body | Join the item below to this one |
| `C-k` at the end of a body | Join the item below to this one |
| `C-c C-t` | Cycle the task state |
| `C-c C-f` | Hide or show the children |
| `C-c C-d` | Delete the item with its children |
| `C-c SPC` | Select the item with its children, to act on several items |
| `C-c C-x w` | Copy the reference (the id) of the item |
| `C-c C-x t` | Make the item a transclusion of the copied reference |
| `C-c C-x T` | Make a transclusion a plain item again |
| `C-c C-x S` | Send the item with its children to the `# Stash` heading at the end of the page |
| `C-/`, `C-?` | Undo and redo, through the history of the core |
| `C-x C-s` | Save now |

The keys like `M-<right>`, with `M-` and an arrow, keep their Emacs meanings, such as moving by a
word.

Text is also saved by itself after a pause. The edit of an item ends when Emacs idles for
`peta-journal-idle-finish-seconds`, as the core does not take in changes to the file during an edit.

These keys go to other places:

| Key | Action |
|---|---|
| `C-c C-j` | Today's journal |
| `C-c C-s` | The search box |
| `C-c C-n`, `C-c C-p` | The next or the previous journal |
| `C-c C-b` | Back to the page before |
| `C-c C-o` | Open the wiki link or the tag at the point |
| `C-M-i` | Complete the name of a page inside `[[` |

The commands on the whole page are `C-c C-x` and then one letter:

| Key | Action |
|---|---|
| `C-c C-x s` | Show or hide the sidebar on the left |
| `C-c C-x r` | Rename the page, or the folder of an index page |
| `C-c C-x D` | Make an index page the page of the name of its folder |
| `C-c C-x d` | Delete the page |
| `C-c C-x p` | Show or hide the list of page properties |
| `C-c C-x n` | Open a page by name in a new buffer |
| `C-c C-x i` | Insert an image file at the point |
| `C-c C-x b` | Backlinks of this page |
| `C-c C-x f` | Forward again to the page left by going back |
| `C-c C-x c` | Pick a date in the calendar and open it |

To open a page by name, use the search box.

## The sidebar

`C-c C-x s` or `M-x peta-journal-sidebar` opens a sidebar on the left edge of the frame. Press it
again to close it. `peta-journal-sidebar-width` sets its width. The default is `30`.

It lists what the sidebar of the web app lists, in the same order. The calendar of the web app is
left out, as `C-c C-x c` opens the calendar of Emacs.

- The pages in the folder of the page on screen. The name of the folder at the top opens its index
  page, and `../` opens the folder above. The page on screen has a color.
- Contents: the headings of the page, indented by their level
- Recent Pages

`RET` or a click on a line opens its page. On a heading, the point moves to that item of the page.
The sidebar is drawn again when the page changes.

`TAB` on any line of a section closes it to its title, and opens it again. A click on the `▾` or
`▸` before a title does the same. The closed sections are saved in `peta-journal-sidebar-collapsed`,
so they stay closed in the next Emacs.

## Completing a wiki link

Between `[[` and `]]`, `C-M-i` completes the name of a page. This is the standard completion of
Emacs, `completion-at-point`. A completion UI such as Corfu or Company shows the names in its own way.

- The server searches for the text between `[[` and `]]`. A name does not have to start with that
  text: a folder in its path or a word inside it also matches. The names keep the order of the
  server. There are 10 at most.
- One name is put in at once. With more names, choose one from the list.
- With no text typed, the names are the recent pages.
- The chosen name replaces all the text between `[[` and `]]`. The point stays in front of `]]`.
- Emacs alone shows the names only when you press the key. They do not pop up while you type.

## The calendar

`C-c C-x c` or `M-x peta-journal-calendar` opens the calendar of Emacs. Move to a day with the usual
keys of the calendar. These keys are added:

| Key | Action |
|---|---|
| `RET` | Open the journal of the day at the point |
| `w` | Open the page of the week of that day |
| `m` | Open the page of the month of that day |
| `y` | Open the page of the year of that day |

Opening the calendar shows these keys in the echo area. Each key closes the calendar first. `q` closes the calendar and opens nothing.

The server sets how the weeks are numbered: US or ISO. The first day of a week in the calendar
comes from `calendar-week-start-day` of Emacs, not from the server. With ISO weeks, set it to
Monday: `(setq calendar-week-start-day 1)`.

A calendar opened by `M-x calendar` does not have these keys.

## The gap between items

`peta-journal-item-gap` sets the gap between items, in pixels. The default is `4`, and `0` removes
the gap. A new value applies to the rows drawn after the change. Only a graphical Emacs shows the gap.

## Selecting items

`C-c SPC` or `M-x peta-journal-select` selects the item at the point, with its children. The edit
of the item ends. While items are selected, the selected rows have the color of the region. The
buffer is in `peta-journal-selection-mode`, and its text cannot change. These keys act on all the
selected items:

| Key | What it does |
|---|---|
| `n`, `C-n`, `<down>`, `S-<down>` | Add the item below, or take the top one away |
| `p`, `C-p`, `<up>`, `S-<up>` | Add the item above, or take the bottom one away |
| `M-S-<up>`, `M-S-<down>` | Move the items up or down |
| `TAB`, `M-S-<right>` | Indent the items |
| `<backtab>`, `M-S-<left>` | Outdent the items |
| `d`, `DEL`, `C-d`, `<delete>` | Delete the items with their children |
| `w`, `M-w` | Put the Markdown of the items in the kill ring |
| `C-w` | Put the Markdown in the kill ring, and delete the items |
| `C-/`, `C-?` | Undo, redo |
| `C-g`, `q`, `<escape>` | End the selection |

The selection ends when the selected items are deleted too.

## Transclusion

A transclusion is an item that shows another item, with its children. To make one, copy the
reference of the item to show with `C-c C-x w`. Then press `C-c C-x t` on the item that shows it.
The reference is the id of the item, and it goes into the kill ring. The reference that "Copy
Reference" of the web app copies works too, through the clipboard. Text that is not an id does
nothing.

`C-c C-x T` makes a transclusion a plain item again. It works on a broken transclusion too. On a
plain item, the command stops without asking.

## Rename a page

`C-c C-x r` or `M-x peta-journal-rename-page` asks for a new name and offers the path of the page. A
`/` in the name puts the page in a folder, and a name without `/` puts it outside any folder. If
another page has that name, Emacs asks whether to overwrite it.

On an index page, the command asks for the new path of its folder instead, and offers the path
there is. The index page moves to the new folder. A journal, the index of the root and an atlas
cannot be renamed: the command stops there without asking.

`C-c C-x D` or `M-x peta-journal-demote-index` makes an index page the page of the name of its
folder. For example, the index page of `work/memo/` becomes the page `work/memo`. Emacs asks first.
Only an index page without pages under it can become a page. On another page, the command stops
without asking.

## Delete a page

`C-c C-x d` or `M-x peta-journal-delete-page` deletes the page on screen, with its file. Emacs asks
first, with the name of the page. Today's journal opens then. Today's journal itself is not deleted:
the command does nothing there.

## Inserting an image

`C-c C-x i` or `M-x peta-journal-insert-image` asks for a file. The server keeps a copy in the `assets`
folder of the notes, and a link to it goes in at the point.

The server names the copy. The name is an ID made of the time and random letters, an HMTID, with the
extension of the file. The link text keeps the name of the file. For example, inserting `構成図.png`
puts in `![構成図.png](../assets/20261110090501_QZ998PX.png)`. The time in the name is UTC.

The server refuses a file over 50MB. Emacs then shows the reason, and the body stays as it was.

### Pasting an image from the clipboard

Copy an image, such as a screenshot, and press `C-y`. The image goes to the server, and a link to it
goes in at the point. A pasted image has no name of its own, so the link text is `image.png`.

- The image is pasted only when the clipboard has an image and no text. With text in the clipboard,
  `C-y` pastes the text as before.
- Emacs must be able to read the image in the clipboard as PNG. On Windows, Emacs can do so from
  version 31. On Windows with Emacs 30 or older, `C-y` pastes only text as before. This was tested
  only with Emacs 31.1 on Windows.

## Seeing images

An item without the point shows the image in place of a link like `![name](../assets/file)`. An
image outside the server, like `![name](https://example.com/file.png)`, is shown too: Emacs fetches
it from its URL, as a web browser does. The item with the point shows the link as text. The image comes back when the point leaves. Only the
display changes: the text of the body stays as it is.

An image fits the window, and fits again when the window changes size. A link with `#w=300&h=240`
sets the size. The image scale of Emacs (`image-scaling-factor`) applies to this size too.

The link stays as text in these cases:

- Emacs cannot show images here, as in a terminal.
- The server does not have the file, or Emacs cannot read its format. Some builds of Emacs
  cannot show PNG at all.
- The link is in code.
- The link points neither to the `assets` of the server nor to an `http://` or `https://` URL.
- The file name in `assets` has characters other than ASCII letters, digits, `.`, `_` and `-`.

The images come from the server. Typing does not wait for them: an image appears when it arrives.
Emacs tries a failed image again when you open another page. Emacs keeps the bytes of the images in
memory, up to `peta-journal-image-memory-limit` (64MB). Over it, Emacs forgets the images that no
open page links to, the one used longest ago first. A page that shows a forgotten image fetches it
again.

## Page properties

`C-c C-x p` shows the properties of the page in another window. The point goes to the first
property in the list. The same key in the window of the page closes the list.

These keys work in the list:

| Key | Action |
|---|---|
| `s` | Set a property. It asks for a key and a value |
| `d` | Remove a property. It asks for the key to remove |
| `q` | Close the list |
| `n`, `p` | Go to the next or the previous line |

`s` and `d` offer the key on the line of the point by default. `RET` alone then takes the property
on that line. The point stays on its line after the change.

The list and the completion show each key by the body of its name and its type, as
`rating (number)` for the key `rating_n`, so two keys that share a body are told apart.
Keys are completed from the property definitions of the server. A new key is defined in the
settings of the web app, not here. Separate the values of a list with commas.
`M-x peta-journal-set-property` and `M-x peta-journal-delete-property` do the same.

A date and a time appear in local time, as `YYYY-MM-DD HH:MM`, and you type them in the same form.
The server keeps them in UTC. A new key starts at the current time, so `RET` alone accepts it.

A journal cannot have properties, so on a journal the list does not open.

## Two pages at once

`C-c C-x n` or `M-x peta-journal-new-buffer` opens a page in a new buffer, and you can edit both pages.
An empty name opens today's journal. Each buffer has its own shell and its own undo history. Killing
a buffer stops only its shell.

If a buffer already shows the page, this command goes to that buffer. The other keys can still show
one page in two buffers, and Emacs then warns you. Avoid this. Each buffer is a client of its own,
so it shows what the other saves ([ADR 015](../../../../docs/adr/adr-015-client-id-on-every-change-request.md)).
But a buffer waits for its own edit to end first. Edits of one item in two buffers could overwrite
each other.

## The search box

Press `C-c C-s` and type in the minibuffer. The results appear in a window below and change as you
type. They list the pages, an entry that makes a new page, and the items.

| Key | Action |
|---|---|
| `TAB` before typing | List the recent pages |
| `C-n`, `<down>` | Select the next result |
| `C-p`, `<up>` | Select the previous result |
| `RET` | Open the selected result |
| `C-g` | Close the search box and stay on the page |

You can press these keys right after typing. They wait for the results of what you typed.

The recent pages are up to 10 doc pages, the most recently modified first. Typing after that brings
back the usual search results.

To open the search box from any buffer, add this to your Emacs init file. Keys of `C-c` and one
letter are kept free for users.

```elisp
(autoload 'peta-journal-search "peta-journal" nil t)
(global-set-key (kbd "C-c s") #'peta-journal-search)
```

## Not in this version

The atlas, extensions and settings.

## Tests

`cargo test -p outliner-emacs` runs two kinds of tests:

- `src/tests/` tests the shell around the core, with a fake host.
- `tests/process.rs` starts the program itself. The test plays Emacs on its stdin and stdout, and
  a small server of the test answers its HTTP requests. Neither Emacs nor the journal server is
  needed. One of these tests waits 5 seconds for the shell to give up on a server.

`cargo cov` measures both kinds (see `BUILD.md`). CI runs them, the tests of the buffer below and
the E2E tests. The E2E tests run in a job of their own, `emacs-e2e`, on Linux.

The tests of the buffer run without a process:

```powershell
cd peta-journal/client/apps/emacs/outliner
emacs -Q --batch -L lisp -l lisp/test/peta-journal-test.el -f ert-run-tests-batch-and-exit
```

The E2E tests press real keys and use the real shell and a real server. They run what the other
tests cannot: the mode, the shell and the server together. Build both programs first. Each test starts its server on
port 18087, with all its files in a temporary folder (`--throwaway-dir`). The tests stop if the
server reports a markdown folder outside that temporary folder (see ADR-002). They are skipped when
`PETA_E2E_SERVER` is not set.

```powershell
$env:PETA_E2E_SERVER = "<peta-journal>\journal\target\debug\journal-server.exe"
emacs -Q --batch -L lisp -L lisp/test -l lisp/test/peta-journal-e2e.el -f ert-run-tests-batch-and-exit
```

On Windows, `emacs` must be the real `emacs.exe`, not an `emacsclient` wrapper.

### Formatting

The formatter is [elisp-formatter](https://github.com/sosuisen/elisp-formatter), shared with
other projects. It indents the files as `emacs-lisp-mode` does, with spaces and no tabs, and
removes the spaces at the end of a line. So TAB in Emacs gives the same indent. CI fetches the
tag `v0.1.0` and runs the check.

- A macro is indented by its `declare (indent ...)` form. Give a new macro one.
- For a quoted list of symbols, put the first symbol alone on the first line. The rest then go
  under it.

Clone elisp-formatter once, anywhere, and run it from this folder:

```powershell
emacs -Q --batch -l <elisp-formatter>/elisp-formatter.el -f elisp-formatter-write lisp   # formats the files
emacs -Q --batch -l <elisp-formatter>/elisp-formatter.el -f elisp-formatter-check lisp   # changes nothing, fails on a change
```
