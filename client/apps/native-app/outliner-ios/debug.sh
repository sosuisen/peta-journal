#!/bin/sh
# Builds the app (the Debug build) and starts it in the simulator of an iPhone.
#
#   ./debug.sh                   the app in the simulator "iPhone 18 Pro"
#   ./debug.sh --device <name>   another simulator, as `xcrun simctl list devices` names it
#   ./debug.sh --generate        makes generated/ again first, after a change in Rust
#   ./debug.sh --test            runs the tests of the screen in the simulator, and starts no app
#
# The App key of Dropbox and the folder in Dropbox come from Dropbox.xcconfig (see README.md).
# Without that file, the app is built without the sync.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
cd "$here"

device="iPhone 18 Pro"
generate=no
action=build
while [ $# -gt 0 ]; do
    case "$1" in
        --generate) generate=yes ;;
        --test) action=test ;;
        --device)
            shift
            device=$1
            ;;
        -h | --help)
            sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'
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
# A Mac that goes to sleep stops the simulator, and the tests then wait in vain
if ! caffeinate -dimsu xcodebuild -project PetaJournal.xcodeproj -scheme PetaJournal \
    -destination "platform=iOS Simulator,name=$device" -derivedDataPath build "$action" >build.log 2>&1; then
    grep -E "error:|BUILD FAILED|TEST FAILED" build.log || tail -n 30 build.log
    echo "It failed. The whole log is in $here/build.log" >&2
    exit 1
fi
if [ "$action" = test ]; then
    grep -E "Executed [0-9]+ tests" build.log | tail -n 1
    exit 0
fi

xcrun simctl boot "$device" 2>/dev/null || true
open -a DeviceHub
xcrun simctl bootstatus "$device" >/dev/null
xcrun simctl install "$device" build/Build/Products/Debug-iphonesimulator/PetaJournal.app
xcrun simctl terminate "$device" com.sosuisha.petajournal 2>/dev/null || true
xcrun simctl launch "$device" com.sosuisha.petajournal
