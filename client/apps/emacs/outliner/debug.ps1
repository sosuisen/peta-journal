# Builds the program (the debug build) and opens today's journal in Emacs with it.
#
#   ./debug.ps1        with your init file
#   ./debug.ps1 -Q     without your init file; what is given goes to Emacs
#
# The journal server has to run already. `emacs` may also be emacsclient: all goes in one
# --eval. In an Emacs that already shows PetaJournal, close its buffers first: the program
# is chosen when the shell starts.
$ErrorActionPreference = 'Stop'

Push-Location $PSScriptRoot
try {
    cargo build -p client-shell-emacs
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
finally {
    Pop-Location
}

# A backslash would start an escape in the string of Emacs Lisp
$here = $PSScriptRoot -replace '\\', '/'
# The program is named here: the mode alone takes the release build first, when there is one.
# One line: a script that stands for `emacs` on Windows (.cmd) loses what follows a line break
$lisp = "(progn (add-to-list 'load-path `"$here/lisp`")" +
    " (setq peta-journal-shell-program (executable-find `"$here/../../../target/debug/client-shell-emacs`"))" +
    " (require 'peta-journal) (peta-journal))"
# PowerShell does not escape the quotes for a .cmd, and the string would lose them
if ((Get-Command emacs).Source -match '\.(cmd|bat)$') { $lisp = $lisp -replace '"', '\"' }
emacs @args --eval $lisp
exit $LASTEXITCODE
