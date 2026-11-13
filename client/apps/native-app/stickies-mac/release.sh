#!/bin/sh
# Builds the app (the Release build).
#
#   ./release.sh                        builds only
#   ./release.sh --run                  also starts it: the stickies of the run before, of the server at http://localhost:8087
#   ./release.sh --run -url /pages/notes    a page; -url /today is today's journal
#   ./release.sh --generate ...         makes generated/ again first, after a change in Rust
#
# --run ends a running app first: `open` would only bring it to the front, with its old build
# and its old arguments. What follows the options goes to the app. The journal server has to
# run already then.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
cd "$here"

generate=no
run=no
while [ $# -gt 0 ]; do
    case "$1" in
        --generate) generate=yes ;;
        --run) run=yes ;;
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
if ! xcodebuild -project PetaStickies.xcodeproj -scheme PetaStickies -configuration Release \
    -derivedDataPath build build >build.log 2>&1; then
    grep -E "error:|BUILD FAILED" build.log || tail -n 30 build.log
    echo "The build failed. The whole log is in $here/build.log" >&2
    exit 1
fi

app=build/Build/Products/Release/PetaStickies.app
if [ "$run" = no ]; then
    echo "Built $here/$app"
    exit 0
fi
if pkill -x PetaStickies; then
    while pgrep -x PetaStickies >/dev/null; do sleep 0.1; done
fi
open "$app" --args "$@"
