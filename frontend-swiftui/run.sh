#!/bin/sh
# Builds the app and starts it. A running app is ended first: `open` would only bring it to the
# front, with its old build and its old arguments.
#
#   ./run.sh                          the stickies of the run before, of the server at http://localhost:8087
#   ./run.sh -url /pages/notes        a page; -url /today is today's journal
#   ./run.sh -server http://localhost:8087 -url /pages/notes
#   ./run.sh --generate ...           makes generated/ again first, after a change in Rust
#   ./run.sh --debug ...              the Debug build, which is slower
#
# The journal server has to run already.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
cd "$here"

configuration=Release
generate=no
while [ $# -gt 0 ]; do
    case "$1" in
        --generate) generate=yes ;;
        --debug) configuration=Debug ;;
        -h | --help)
            sed -n '2,11p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *) break ;;
    esac
    shift
done

if [ "$generate" = yes ] || [ ! -d generated ]; then
    ./generate.sh
fi
xcodegen generate --quiet
# Only what went wrong is shown: the whole log of xcodebuild is thousands of lines
if ! xcodebuild -project PetaJournal.xcodeproj -scheme PetaJournal -configuration "$configuration" \
    -derivedDataPath build build >build.log 2>&1; then
    grep -E "error:|BUILD FAILED" build.log || tail -n 30 build.log
    echo "The build failed. The whole log is in $here/build.log" >&2
    exit 1
fi

if pkill -x PetaJournal; then
    while pgrep -x PetaJournal >/dev/null; do sleep 0.1; done
fi
open "build/Build/Products/$configuration/PetaJournal.app" --args "$@"
