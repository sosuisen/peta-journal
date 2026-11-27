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
#
# When tests on the device fail, it runs only those tests once more, and passes when they pass
# then (the developer decided, 2026-11-28). On the emulator of the CI, a test sometimes fails for
# the emulator alone. The log keeps the first failure, and names each test that passed only the
# second time. Why not longer waits: they would make every run slower.
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

results=app/build/outputs/androidTest-results/connected/debug
# The counts of the lines of the first try. The coverage task of app/build.gradle.kts reads them
# with those of the second try: the second try runs only a few tests
first_try=app/build/coverage-first-try

# Gradle names only the report of the tests on the device. These are the tests that failed, each
# with the start of its failure, and what Android said about the clipboard and which window has
# the focus: Android lets only the app with the focus read the clipboard. The whole log of the
# device goes to build/logcat-<try>.txt, which the CI keeps (#1279)
show_failures() {
    mkdir -p build
    "$adb" logcat -d >"build/logcat-$1.txt" 2>/dev/null || true
    for report in "$results"/*.xml; do
        [ -f "$report" ] || continue
        awk '/<testcase / { test = $0 } /<failure|<error/ { print test; left = 6 } left > 0 { print; left-- }' "$report"
    done
    "$adb" logcat -d 2>/dev/null | grep -i "clipboard" | tail -n 20 || true
    "$adb" shell dumpsys window 2>/dev/null | grep -E "mCurrentFocus|mFocusedApp" || true
}

# The tests on the device that failed, one `class#method` on each line
failed_tests() {
    for report in "$results"/*.xml; do
        [ -f "$report" ] || continue
        awk '/<testcase / {
                match($0, / name="[^"]*"/); name = substr($0, RSTART + 7, RLENGTH - 8)
                match($0, / classname="[^"]*"/); class = substr($0, RSTART + 12, RLENGTH - 13)
            }
            /<failure|<error/ { print class "#" name }' "$report"
    done | sort -u
}

# Packs the core only for the kind of the device: each one is about 70MB
abi=$("$adb" shell getprop ro.product.cpu.abi | tr -d '\r')
# A report or counts of an earlier run would be taken for this one
rm -rf "$results" app/build/outputs/code_coverage "$first_try" build/logcat-*.txt
if ./gradlew :app:coverage "-Pabi=$abi"; then
    exit 0
fi
echo "=== The first try failed ==="
show_failures first-try
failed=$(failed_tests)
if [ -z "$failed" ]; then
    # No test on the device failed: a test of the JVM, the build or the floor did. A second try
    # would end the same way
    exit 1
fi
count=$(echo "$failed" | wc -l | tr -d ' ')
echo "=== Second try: only the $count tests on the device that failed, once more ==="
echo "$failed"
if [ ! -d app/build/outputs/code_coverage ]; then
    # The second try alone would cover too few lines, and fail on the floor
    echo "The first try wrote no counts of the lines, and is not tried again" >&2
    exit 1
fi
mkdir -p "$first_try"
cp -R app/build/outputs/code_coverage "$first_try/"
cp -R "$results" "$first_try/results"
if ! ./gradlew :app:coverage "-Pabi=$abi" \
    "-Pandroid.testInstrumentationRunnerArguments.class=$(echo "$failed" | paste -sd , -)"; then
    echo "=== The second try failed too ==="
    show_failures second-try
    exit 1
fi
echo "=== Flaky: these $count tests failed on the first try, and passed on the second ==="
echo "$failed"
# A note on the page of the run in GitHub Actions, so that a flaky test is seen without the log
if [ "${GITHUB_ACTIONS:-}" = true ]; then
    echo "$failed" | while read -r test; do
        echo "::warning title=Flaky test on the device::$test failed on the first try, and passed on the second"
    done
fi
