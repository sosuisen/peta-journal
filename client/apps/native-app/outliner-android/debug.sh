#!/bin/sh
# Builds the app (the debug build), installs it on the device or the emulator that adb sees,
# and starts it.
#
#   ./debug.sh
#   ./debug.sh -Pabi=arm64-v8a -PdropboxKey=<App key> -PdropboxFolder=/PetaJournal
#   ./debug.sh --generate ...     makes generated/ again first, after a change in Rust
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
./gradlew :app:installDebug "$@"
"$ANDROID_HOME/platform-tools/adb" shell am start -n net.sosuisen.petajournal/.MainActivity
