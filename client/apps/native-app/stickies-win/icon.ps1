# Makes the icon of the app from images/peta-journal.svg: the notebook in the middle of a
# clear square. The icon is kept in the repository, so a build does not run this.
# Run it again after a change of the SVG. It needs only Rust: the crate client/tools/app-icon
# makes the icon.
$ErrorActionPreference = 'Stop'

$here = $PSScriptRoot
$svg = Join-Path $here '../../../../images/peta-journal.svg'
$icon = Join-Path $here 'PetaStickies/PetaStickies.ico'

Push-Location (Join-Path $here '../../..')
try {
    cargo run --release -q -p app-icon -- windows $svg $icon
    exit $LASTEXITCODE
}
finally { Pop-Location }
