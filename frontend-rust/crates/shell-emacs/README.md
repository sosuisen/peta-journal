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

The setting takes effect only when the shell starts. If PetaJournal is already open, kill the
`*PetaJournal*` buffer with `C-x k`. This also stops the shell. Then run `M-x peta-journal` again.

If the shell stops by itself, Emacs shows a warning and `[shell stopped]` in the mode line. Edits are
not saved until it runs again. Run `M-x peta-journal` to start it. This shows the saved text, so
copy what you typed after the warning first. Errors of the shell are in `*peta-journal-log*`.

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
| `C-/`, `C-?` | Undo and redo, through the history of the core |
| `C-x C-s` | Save now |

Text is also saved by itself after a pause. The edit of an item ends when Emacs idles for
`peta-journal-idle-finish-seconds`, as the core does not take in changes to the file during an edit.

These keys go to other places:

| Key | Action |
|---|---|
| `C-c C-j` | Today's journal |
| `C-c C-f` | Open a page by name |
| `C-c C-n`, `C-c C-p` | The next or the previous journal |
| `C-c C-l` | Back to the page before |
| `C-c C-o` | Open the wiki link or the tag at the point |
| `C-c /` | The search box |
| `C-c C-b` | Backlinks of this page |

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

Wiki link completion, images, the atlas, extensions, settings and operations on several selected
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
