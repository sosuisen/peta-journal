#!/bin/sh
# debug.ps1 for Git Bash. The options are the same:
#
#   ./debug.sh -url /pages/notes
#   ./debug.sh -Generate -server http://localhost:8087
set -eu
here=$(cd "$(dirname "$0")" && pwd)
script=$(cygpath -w "$here/debug.ps1")
# Git Bash would turn an argument such as /pages/notes into a path of Windows
MSYS_NO_PATHCONV=1 exec pwsh -NoProfile -File "$script" "$@"
