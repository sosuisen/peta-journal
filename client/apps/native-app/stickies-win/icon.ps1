# Makes the icon of the app from images/peta-journal.svg: the notebook in the middle of a
# clear square. The icon is kept in the repository, so a build needs neither of the two tools.
# Run it again after a change of the SVG.
#
# It needs resvg and ImageMagick:
#   winget install --id linebender.resvg
#   winget install --id ImageMagick.ImageMagick
$ErrorActionPreference = 'Stop'

$here = $PSScriptRoot
$svg = Join-Path $here '../../../../images/peta-journal.svg'
$icon = Join-Path $here 'PetaStickies/PetaStickies.ico'
$drawn = Join-Path ([IO.Path]::GetTempPath()) 'peta-stickies-icon.png'

try {
    resvg --height 1024 $svg $drawn
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    # The drawing is not in the middle of its page: the empty edges are cut off first.
    # Windows picks the size that fits. Each size is made from the large image
    magick $drawn -trim +repage -background none -gravity center -extent 1024x1024 `
        -define icon:auto-resize=256,128,96,64,48,40,32,24,20,16 $icon
    exit $LASTEXITCODE
}
finally {
    Remove-Item $drawn -ErrorAction SilentlyContinue
}
