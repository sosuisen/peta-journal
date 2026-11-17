#!/bin/sh
# Makes the icon of the app from images/peta-journal.svg: the notebook in the middle of a white
# square, as in the icon of the Android shell. The image is kept in the repository, so a build
# does not run this. Run it again after a change of the SVG.
#
# It needs only Rust: the crate client/tools/app-icon makes the icon.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
svg="$here/../../../../images/peta-journal.svg"
icon="$here/App/Assets.xcassets/AppIcon.appiconset/icon-1024.png"

cd "$here/../../.."
cargo run --release -q -p app-icon -- ios "$svg" "$icon"
