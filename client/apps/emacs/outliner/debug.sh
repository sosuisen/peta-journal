#!/bin/sh
# Builds the program (the debug build) and opens today's journal in Emacs with it.
#
#   ./debug.sh        with your init file
#   ./debug.sh -Q     without your init file; what is given goes to Emacs
#
# The journal server has to run already. `emacs` may also be emacsclient: all goes in one
# --eval. In an Emacs that already shows PetaJournal, close its buffers first: the program
# is chosen when the shell starts.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
cd "$here"

cargo build -p outliner-emacs

# Emacs on Windows does not know a path of Git Bash
if command -v cygpath >/dev/null; then here=$(cygpath -m "$here"); fi

# The program is named here: the mode alone takes the release build first, when there is one
exec emacs "$@" --eval "(progn
  (add-to-list 'load-path \"$here/lisp\")
  (setq peta-journal-shell-program
        (executable-find \"$here/../../../target/debug/outliner-emacs\"))
  (require 'peta-journal)
  (peta-journal))"
