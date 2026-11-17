#!/bin/sh
# Makes the icon of the app from images/peta-journal.svg: the notebook on a white plate with
# rounded corners, as the icons of macOS have. The icon set is kept in the repository, so a
# build does not run this. Run it again after a change of the SVG.
#
# It needs only Rust: the crate client/tools/app-icon makes the icon.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
svg="$here/../../../../images/peta-journal.svg"
icons="$here/App/Assets.xcassets/AppIcon.appiconset"

cd "$here/../../.."
cargo run --release -q -p app-icon -- macos "$svg" "$icons"
