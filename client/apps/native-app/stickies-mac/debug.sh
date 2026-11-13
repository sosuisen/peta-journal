#!/bin/sh
# Builds the app (the Debug build, which is slower) and starts it. A running app is ended
# first: `open` would only bring it to the front, with its old build and its old arguments.
#
#   ./debug.sh                        the stickies of the run before, of the server at http://localhost:8087
#   ./debug.sh -url /pages/notes      a page; -url /today is today's journal
#   ./debug.sh -server http://localhost:8087 -url /pages/notes
#   ./debug.sh --generate ...         makes generated/ again first, after a change in Rust
#
# The journal server has to run already.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
cd "$here"

generate=no
while [ $# -gt 0 ]; do
    case "$1" in
        --generate) generate=yes ;;
        -h | --help)
            sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'
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
if ! xcodebuild -project PetaStickies.xcodeproj -scheme PetaStickies -configuration Debug \
    -derivedDataPath build build >build.log 2>&1; then
    grep -E "error:|BUILD FAILED" build.log || tail -n 30 build.log
    echo "The build failed. The whole log is in $here/build.log" >&2
    exit 1
fi

if pkill -x PetaStickies; then
    while pgrep -x PetaStickies >/dev/null; do sleep 0.1; done
fi
open build/Build/Products/Debug/PetaStickies.app --args "$@"
