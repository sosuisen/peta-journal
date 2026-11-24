#!/bin/sh
# Runs the mutation tests of one Swift package with swift-mutation-testing. The settings of
# each package are in its .swift-mutation-testing.yml.
#
#   ./mutation-test.sh swift-shared
#   ./mutation-test.sh outliner-ios
#   ./mutation-test.sh stickies-mac    builds journal-server first: most of its tests need it
#
# Run generate.sh of the app first. The verdicts are kept in .swift-mutation-testing-cache/
# of the package, and a later run tests again only what changed.
set -eu
here=$(cd "$(dirname "$0")" && pwd)

case "${1:-}" in
    swift-shared | outliner-ios | stickies-mac) package=$1 ;;
    -h | --help)
        sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'
        exit 0
        ;;
    *)
        echo "Usage: $0 swift-shared | outliner-ios | stickies-mac" >&2
        exit 2
        ;;
esac

if ! command -v swift-mutation-testing >/dev/null; then
    echo "swift-mutation-testing is missing. See the README of stickies-mac." >&2
    exit 1
fi

# The tool copies the package into a sandbox beside this folder, and builds it there. The
# packages name ../swift-shared, so it has to be beside the sandbox too
sandboxes="${TMPDIR:-/tmp}"
sandboxes="${sandboxes%/}/swift-mutation-testing"
mkdir -p "$sandboxes"
ln -sfn "$here/swift-shared" "$sandboxes/swift-shared"

if [ "$package" = stickies-mac ]; then
    (cd "$here/../../../journal" && cargo build -p journal-server)
    PETA_E2E_SERVER=$(cd "$here/../../../journal/target/debug" && pwd)/journal-server
    export PETA_E2E_SERVER
fi

cd "$here/$package"
status=0
swift-mutation-testing . || status=$?
if [ "$package" = stickies-mac ]; then
    # The tool kills a run of the tests as soon as a test fails, and the servers of that run
    # go on. Only the servers of this program are stopped, not one that the user runs
    pkill -f "^$PETA_E2E_SERVER --port" || true
fi
exit "$status"
