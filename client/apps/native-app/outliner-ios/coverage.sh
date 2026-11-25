#!/bin/sh
# Runs every test of outliner-ios in the simulator with code coverage: the tests of the packages
# (Tests/), of the app (AppTests/) and of the screen (UITests/). Prints the lines covered of each
# file, and fails when less than 90% of the lines are covered. The lines are counted over the
# whole module, the app (App/) and its packages (Sources/), not file by file.
#
#   ./coverage.sh                  in the simulator "iPhone 18 Pro", or the first iPhone without it
#   ./coverage.sh --device <name>  in another simulator, as `xcrun simctl list devices` names it
#
# Run generate.sh first. The tests of the screen take about 15 minutes.
#
# Left out of the measurement, with the consent of the developer (#1088):
# - Sources/PetaDropbox/SdkDropbox.swift and Sources/PetaDropbox/DropboxSdk.swift: they only pass
#   each call on to the SDK of Dropbox. A test of them needs a real account of Dropbox.
# - generated/ and ../swift-shared: generate.sh makes them from the Rust crates, and the Rust tests
#   check them. They are not in the targets below.
# - The bundles of the tests themselves.
set -eu
# The coverage names each file by its real path, also when a link leads to the folder
here=$(cd "$(dirname "$0")" && pwd -P)
floor=90
measured="App/ Sources/"
excluded="Sources/PetaDropbox/SdkDropbox.swift Sources/PetaDropbox/DropboxSdk.swift"

if [ "${1:-}" = --device ]; then
    device=$2
elif xcrun simctl list devices available | grep -q "iPhone 18 Pro ("; then
    device="iPhone 18 Pro"
else
    # A machine without that simulator, such as a runner of the CI: its first iPhone
    device=$(xcrun simctl list devices available | sed -n 's/^ *\(iPhone[^(]*[^ (]\) *(.*/\1/p' | head -n 1)
fi
echo "Simulator: $device"

cd "$here"
# The project asks for the file, also when the app is built without the sync
[ -f Dropbox.xcconfig ] || : >Dropbox.xcconfig
xcodegen generate --quiet
rm -rf build/coverage.xcresult
# A new checkout, such as on a runner of the CI, has no build/ yet, and the log goes there
mkdir -p build
# A Mac that goes to sleep stops the simulator, and the tests then wait in vain
if ! caffeinate -dimsu xcodebuild -project PetaJournal.xcodeproj -scheme PetaJournal \
    -destination "platform=iOS Simulator,name=$device" -derivedDataPath build \
    -enableCodeCoverage YES -resultBundlePath build/coverage.xcresult test >build/coverage.log 2>&1; then
    # A panic of the core names where it stopped on the line after "panicked at" (#1176)
    awk '/error:|Test Case.*failed|TEST FAILED/ { print; found = 1 } /panicked at/ { print; getline; print; found = 1 }
        END { exit !found }' build/coverage.log || tail -n 30 build/coverage.log
    echo "The tests failed. The whole log is in $here/build/coverage.log" >&2
    exit 1
fi
grep -E "Executed [0-9]+ tests" build/coverage.log | tail -n 1

# One line for each file of the folders measured: "<path> <covered> <lines>". The archive holds
# each file once, with the counts of every binary that runs it: the app, and the bundles of the
# tests that link the packages. The report by target is not used: it left the packages out of one
# run
files=$(xcrun xccov view --archive --file-list build/coverage.xcresult | while read -r file; do
    path=${file#"$here"/}
    for folder in $measured; do
        # The pattern opens with "(": without it, the shell takes ")" for the end of "$("
        case "$path" in ("$folder"*)
            # A line is "<number>: <count>", or "<number>: *" when nothing runs there
            xcrun xccov view --archive --file "$file" build/coverage.xcresult | awk -v path="$path" -F: '
                $1 ~ /^ *[0-9]+$/ { gsub(/ /, "", $2); if ($2 != "*") { lines++; if ($2 + 0 > 0) covered++ } }
                END { print path, covered + 0, lines + 0 }'
            ;;
        esac
    done
done)
if [ -z "$files" ]; then
    echo "No file in the report." >&2
    exit 1
fi

echo "$files" | awk -v excluded="$excluded" -v floor="$floor" '
    BEGIN { split(excluded, left, " "); for (i in left) out[left[i]] = 1 }
    {
        path = $1
        if (path in out) { printf "%-45s left out\n", path; found[path] = 1; next }
        printf "%-45s %5d of %5d\n", path, $2, $3
        covered += $2
        lines += $3
    }
    END {
        # A file that was renamed would be measured again, and the list would not tell
        for (path in out) if (!(path in found)) { print "Not in the report: " path > "/dev/stderr"; exit 1 }
        printf "Lines covered: %d of %d (%.2f%%), the floor is %d%%\n", covered, lines, covered * 100 / lines, floor
        if (covered * 100 < floor * lines) {
            print "The coverage is under the floor." > "/dev/stderr"
            exit 1
        }
    }
'
