#!/bin/sh
# icon.ps1 for Git Bash.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
script=$(cygpath -w "$here/icon.ps1")
exec pwsh -NoProfile -File "$script" "$@"
