#!/bin/sh
# debug.ps1 for Git Bash. What is given goes to the app.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
script=$(cygpath -w "$here/debug.ps1")
# Git Bash would turn an argument that starts with a slash into a path of Windows
MSYS_NO_PATHCONV=1 exec pwsh -NoProfile -File "$script" "$@"
