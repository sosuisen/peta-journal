#!/bin/sh
# release.ps1 for Git Bash. The options are the same:
#
#   ./release.sh
#   ./release.sh -Run -url /pages/notes
set -eu
here=$(cd "$(dirname "$0")" && pwd)
script=$(cygpath -w "$here/release.ps1")
# Git Bash would turn an argument such as /pages/notes into a path of Windows
MSYS_NO_PATHCONV=1 exec pwsh -NoProfile -File "$script" "$@"
