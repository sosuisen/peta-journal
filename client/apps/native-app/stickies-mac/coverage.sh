#!/bin/sh
# Runs the tests of stickies-mac with code coverage, prints the lines covered of each file, and
# fails when less than 90% of the lines are covered. The lines are counted over the whole module
# (Sources/), not file by file.
#
#   ./coverage.sh
#
# Run generate.sh first. It builds journal-server first: most of the tests need it.
#
# Left out of the measurement, with the consent of the developer:
# - App/: the entry point of the app (@main). `swift test` does not build it.
# - generated/ and ../swift-shared: generate.sh makes generated/ from the Rust crates, and the Rust
#   tests check it. swift-shared is a package of its own, shared with outliner-ios.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
floor=90

(cd "$here/../../../../journal" && cargo build -p journal-server)
PETA_E2E_SERVER=$(cd "$here/../../../../journal/target/debug" && pwd)/journal-server
export PETA_E2E_SERVER

cd "$here"
swift test --enable-code-coverage

# The name of the test bundle depends on the version of SwiftPM: PetaBorderTests.xctest with
# Xcode 27, PetaBorderPackageTests.xctest with Xcode 26. So the one bundle of the build is used
bundles=$(find "$(swift build --show-bin-path)" -maxdepth 1 -name '*.xctest')
if [ "$(echo "$bundles" | grep -c .)" -ne 1 ]; then
    echo "Expected one test bundle, found: $bundles" >&2
    exit 1
fi
tests="$bundles/Contents/MacOS/$(basename "$bundles" .xctest)"
profile="$(dirname "$(swift test --show-codecov-path)")/default.profdata"
report=$(xcrun llvm-cov report "$tests" -instr-profile="$profile" Sources/)
echo "$report"

# The row of the total: the lines are the 8th column, the missed lines the 9th
echo "$report" | awk -v floor="$floor" '
    $1 == "TOTAL" {
        found = 1
        covered = $8 - $9
        printf "Lines covered: %d of %d (%.2f%%), the floor is %d%%\n", covered, $8, covered * 100 / $8, floor
        if (covered * 100 < floor * $8) {
            print "The coverage is under the floor." > "/dev/stderr"
            exit 1
        }
    }
    END { if (!found) { print "No total in the report." > "/dev/stderr"; exit 1 } }
'
