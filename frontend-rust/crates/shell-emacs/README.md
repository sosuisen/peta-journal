# peta-shell-emacs

English | [日本語](README.ja.md)

The Emacs frontend of PetaJournal. It has two parts:

- `peta-shell-emacs` is a Rust program. It holds the Crux core (`peta-core`) and performs its
  effects: HTTP, the event stream and timers.
- `lisp/peta-journal.el` is an Emacs mode. It draws the rows it is sent and reports what the user
  does.

They talk in JSON lines over stdin and stdout. `src/protocol.rs` defines the messages.

## Setup

Build the program:

```powershell
cd peta-journal/frontend-rust
cargo build --release -p peta-shell-emacs
```

Add this to your Emacs init file:

```elisp
(add-to-list 'load-path "<peta-journal>/frontend-rust/crates/shell-emacs/lisp")
(autoload 'peta-journal "peta-journal" nil t)
```

Then run `M-x peta-journal` while the journal server is running. It opens today's journal.

The mode finds the program in `target/release` or `target/debug`, or on the `exec-path`. To use
another one, set `peta-journal-shell-program`. Emacs 29.1 or newer is needed.

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
| `TAB`, `M-<right>` | Indent |
| `<backtab>`, `M-<left>` | Outdent |
| `M-<up>`, `M-<down>` | Move the item up or down |
| `DEL` at the start of a body | Join the item to the one above |
| `C-d` at the end of a body | Join the item below to this one |
| `C-k` at the end of a body | Join the item below to this one |
| `C-c C-t` | Cycle the task state |
| `C-c C-c` | Hide or show the children |
| `C-c C-d` | Delete the item with its children |
| `C-c C-i` | Insert an image file at the point |
| `C-/`, `C-?` | Undo and redo, through the history of the core |
| `C-x C-s` | Save now |

Text is also saved by itself after a pause. The edit of an item ends when Emacs idles for
`peta-journal-idle-finish-seconds`, as the core does not take in changes to the file during an edit.

These keys go to other places:

| Key | Action |
|---|---|
| `C-c C-j` | Today's journal |
| `C-c C-f` | Open a page by name |
| `C-c C-v` | Open a page by name in a new buffer |
| `C-c C-r` | Rename the page |
| `C-c C-a` | Show or hide the list of page properties |
| `C-c C-s` | Set a page property |
| `C-c C-k` | Remove a page property |
| `C-c C-n`, `C-c C-p` | The next or the previous journal |
| `C-c C-l` | Back to the page before |
| `C-c C-o` | Open the wiki link or the tag at the point |
| `C-c /` | The search box |
| `C-c C-b` | Backlinks of this page |

## The gap between items

`peta-journal-item-gap` sets the gap between items, in pixels. The default is `4`, and `0` removes
the gap. A new value applies to the rows drawn after the change. Only a graphical Emacs shows the gap.

## Rename a page

`C-c C-r` or `M-x peta-journal-rename-page` asks for a new name and offers the path of the page. A
`/` in the name puts the page in a folder, and a name without `/` puts it outside any folder. If
another page has that name, Emacs asks whether to overwrite it.

Only a doc page can be renamed. On a journal, an index page or an atlas, the command stops without
asking.

## Inserting an image

`C-c C-i` or `M-x peta-journal-insert-image` asks for a file. The server keeps a copy in the `assets`
folder of the notes, and a link to it goes in at the point. The buffer shows the link as text: it
does not show the image yet.

The server changes the name of the copy. Each character other than ASCII letters, digits, `.`, `_`
and `-` becomes `_`.

The server refuses a large file: 2MB went through and 3MB did not. Emacs then shows the reason, and
the body stays as it was.

## Page properties

`C-c C-a` shows the properties of the page in another window. The point stays in the page. The same
key closes the window.

`C-c C-s` asks for a key and a value, and `C-c C-k` asks for a key to remove. Both then show the
list. Keys are completed from the property definitions of the server. A new key is defined in the
settings of the web app, not here. Separate the values of a list with commas.

A journal cannot have properties, so these keys stop without asking on a journal.

## Two pages at once

`C-c C-v` or `M-x peta-journal-new-buffer` opens a page in a new buffer, and you can edit both pages.
An empty name opens today's journal. Each buffer has its own shell and its own undo history. Killing
a buffer stops only its shell.

If a buffer already shows the page, this command goes to that buffer. The other keys can still show
one page in two buffers, and Emacs then warns you. Avoid this. What one buffer saves does not appear
in the other, as a core reloads an open page only when its file changes. Edits of one item in two
buffers could overwrite each other.

## The search box

Press `C-c /` and type in the minibuffer. The results appear in a window below and change as you
type. They list the pages, an entry that makes a new page, and the items.

| Key | Action |
|---|---|
| `C-n`, `<down>` | Select the next result |
| `C-p`, `<up>` | Select the previous result |
| `RET` | Open the selected result |
| `C-g` | Close the search box and stay on the page |

You can press these keys right after typing. They wait for the results of what you typed.

To open the search box from any buffer, add this to your Emacs init file:

```elisp
(autoload 'peta-journal-search "peta-journal" nil t)
(global-set-key (kbd "C-c /") #'peta-journal-search)
```

## Not in this version

Wiki link completion, showing images, pasting images from the clipboard, the atlas, extensions, settings and operations on several selected
items.

## Tests

`cargo test -p peta-shell-emacs` tests the shell around the core, with a fake host.

The tests of the buffer run without a process:

```powershell
cd peta-journal/frontend-rust/crates/shell-emacs
emacs -Q --batch -L lisp -l lisp/test/peta-journal-test.el -f ert-run-tests-batch-and-exit
```

The E2E tests press real keys and use the real shell and a real server. They run what the other
tests cannot, such as `src/runtime.rs`. Build both programs first. Each test starts its server on
port 18087, with its files in a temporary folder and its own `--config-dir`. The tests stop if the
server reports a markdown folder outside that temporary folder (see ADR-002). They are skipped when
`PETA_E2E_SERVER` is not set.

```powershell
$env:PETA_E2E_SERVER = "<peta-journal>\journal-server-rust\target\debug\journal-server.exe"
emacs -Q --batch -L lisp -L lisp/test -l lisp/test/peta-journal-e2e.el -f ert-run-tests-batch-and-exit
```

On Windows, `emacs` must be the real `emacs.exe`, not an `emacsclient` wrapper.
