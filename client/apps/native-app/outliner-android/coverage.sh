#!/bin/sh
# Runs every test of outliner-android with code coverage: the tests of the JVM (app/src/test) and
# the tests on a device (app/src/androidTest). Prints the lines covered of each file, and fails
# when less than 90% of the lines are covered. app/build.gradle.kts holds the floor and what is
# measured.
#
#   ./coverage.sh          on the device that adb sees, or in the first emulator of this machine
#   ./coverage.sh <avd>    in another emulator, as `emulator -list-avds` names it
#
# Run generate.sh first. It needs ANDROID_HOME. The emulator that it starts has no window: nothing
# on the screen of this machine is touched.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
adb="$ANDROID_HOME/platform-tools/adb"
emulator="$ANDROID_HOME/emulator/emulator"

cd "$here"
if ! "$adb" devices | grep -q "device$"; then
    avd=${1:-$("$emulator" -list-avds | grep -v "^INFO" | head -n 1)}
    if [ -z "$avd" ]; then
        echo "adb sees no device, and this machine has no emulator. README.md says how to make one" >&2
        exit 1
    fi
    echo "Emulator: $avd"
    mkdir -p build
    "$emulator" -avd "$avd" -no-window -no-audio -no-boot-anim -no-snapshot >build/emulator.log 2>&1 &
    started=$!
    trap 'kill "$started" 2>/dev/null || true; wait "$started" 2>/dev/null || true' EXIT
    waited=0
    until [ "$("$adb" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = 1 ]; do
        if ! kill -0 "$started" 2>/dev/null || [ "$waited" -ge 600 ]; then
            tail -n 30 build/emulator.log
            echo "The emulator did not start. The whole log is in $here/build/emulator.log" >&2
            exit 1
        fi
        sleep 2
        waited=$((waited + 2))
    done
    echo "The emulator started in $waited s"
    # A new emulator may show its lock screen, and no app can start under it
    "$adb" shell input keyevent 82
    # A slow start makes the launcher not answer in time, and Android asks about it in a dialog.
    # The dialog takes the focus from the app, and Android lets only the app with the focus read
    # the clipboard: every test of the paste failed on a runner of the CI with that dialog up
    "$adb" shell settings put global hide_error_dialogs 1
    # Keeps the log of the whole run, to read after a test failed
    "$adb" logcat -G 16M
fi

# Packs the core only for the kind of the device: each one is about 70MB
abi=$("$adb" shell getprop ro.product.cpu.abi | tr -d '\r')
if ! ./gradlew :app:coverage "-Pabi=$abi"; then
    # Gradle names only the report of the tests on the device. These are the tests that failed,
    # each with the start of its failure
    for results in app/build/outputs/androidTest-results/connected/debug/*.xml; do
        [ -f "$results" ] || continue
        awk '/<testcase / { test = $0 } /<failure|<error/ { print test; left = 6 } left > 0 { print; left-- }' "$results"
    done
    # What Android said about the clipboard and which window has the focus: Android lets only the
    # app with the focus read the clipboard
    "$adb" logcat -d 2>/dev/null | grep -i "clipboard" | tail -n 20 || true
    "$adb" shell dumpsys window 2>/dev/null | grep -E "mCurrentFocus|mFocusedApp" || true
    exit 1
fi
