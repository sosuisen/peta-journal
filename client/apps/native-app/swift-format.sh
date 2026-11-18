#!/bin/sh
# Formats the Swift of the two Apple shells (outliner-ios and stickies-mac) with swift-format,
# and says what its lint finds. The settings are in .swift-format beside this file.
#
#   ./swift-format.sh           formats the files in place, then lints them
#   ./swift-format.sh --check   changes nothing: fails when a file is not formatted or the
#                               lint finds something
#
# swift-format comes with Xcode, so its version is the one of the compiler. generated/ is
# not formatted: generate.sh makes it.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
cd "$here"

check=no
case "${1:-}" in
    "") ;;
    --check) check=yes ;;
    -h | --help)
        sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'
        exit 0
        ;;
    *)
        echo "Unknown option: $1" >&2
        exit 2
        ;;
esac

set -- \
    outliner-ios/Package.swift outliner-ios/Sources outliner-ios/Tests \
    outliner-ios/App outliner-ios/AppTests outliner-ios/UITests \
    stickies-mac/Package.swift stickies-mac/Sources stickies-mac/Tests stickies-mac/App

if [ "$check" = no ]; then
    xcrun swift-format format --in-place --recursive "$@"
fi
# --strict: a finding fails the run. A file that is not formatted is a finding too
xcrun swift-format lint --strict --recursive "$@"
