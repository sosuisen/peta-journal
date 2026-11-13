#!/bin/sh
# Makes the icon of the app from images/peta-journal.svg: the notebook in the middle of a white
# square, as in the icon of the Android shell. The image is kept in the repository, so a build
# needs neither of the two tools. Run it again after a change of the SVG.
#
# It needs rsvg-convert and ImageMagick:
#   brew install librsvg imagemagick
set -eu
here=$(cd "$(dirname "$0")" && pwd)
svg="$here/../../../../images/peta-journal.svg"
icon="$here/App/Assets.xcassets/AppIcon.appiconset/icon-1024.png"

# The drawing is not in the middle of its page: the empty edges are cut off first.
# iOS takes no icon that has an alpha channel, also when every pixel is opaque
rsvg-convert --height 640 "$svg" |
    magick png:- -trim +repage -background white -gravity center -extent 1024x1024 \
        -alpha remove -alpha off -strip "PNG24:$icon"
