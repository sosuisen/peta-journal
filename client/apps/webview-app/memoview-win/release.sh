#!/bin/sh
# release.ps1 for Git Bash.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
exec pwsh -NoProfile -File "$here/release.ps1"
