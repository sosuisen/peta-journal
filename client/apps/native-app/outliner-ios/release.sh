#!/bin/sh
# Builds the app (the Release build) for the simulator of an iPhone. It starts nothing.
#
#   ./release.sh                   for the simulator "iPhone 18 Pro"
#   ./release.sh --device <name>   another simulator, as `xcrun simctl list devices` names it
#   ./release.sh --generate        makes generated/ again first, after a change in Rust
#
# Why not for a device: that build needs a team for signing, which is chosen in Xcode (see
# README.md). The App key of Dropbox comes from Dropbox.xcconfig, as in debug.sh.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
cd "$here"

device="iPhone 18 Pro"
generate=no
while [ $# -gt 0 ]; do
    case "$1" in
        --generate) generate=yes ;;
        --device)
            shift
            device=$1
            ;;
        -h | --help)
            sed -n '2,9p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 2
            ;;
    esac
    shift
done

if [ "$generate" = yes ] || [ ! -d generated ]; then
    ./generate.sh
fi
# The project asks for the file, also when the app is built without the sync
[ -f Dropbox.xcconfig ] || : >Dropbox.xcconfig
xcodegen generate --quiet
# Only what went wrong is shown: the whole log of xcodebuild is thousands of lines
if ! xcodebuild -project PetaJournal.xcodeproj -scheme PetaJournal -configuration Release \
    -destination "platform=iOS Simulator,name=$device" -derivedDataPath build build >build.log 2>&1; then
    grep -E "error:|BUILD FAILED" build.log || tail -n 30 build.log
    echo "The build failed. The whole log is in $here/build.log" >&2
    exit 1
fi
echo "Built $here/build/Build/Products/Release-iphonesimulator/PetaJournal.app"
