#!/bin/sh
# Builds the app (the release build) as a package that is not signed:
# app/build/outputs/apk/release/app-release-unsigned.apk. It installs nothing: a device takes
# only a signed package.
#
#   ./release.sh
#   ./release.sh -PdropboxKey=<App key> -PdropboxFolder=/PetaJournal
#   ./release.sh --generate ...     makes generated/ again first, after a change in Rust
#
# What follows the option goes to Gradle. README.md says what the properties mean.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
cd "$here"

generate=no
if [ "${1:-}" = --generate ]; then
    generate=yes
    shift
fi
if [ "$generate" = yes ] || [ ! -d generated ]; then
    ./generate.sh
fi

# Git Bash would turn a value such as /PetaJournal into a path of Windows
export MSYS_NO_PATHCONV=1
exec ./gradlew :app:assembleRelease "$@"
