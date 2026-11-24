# Builds the app into dist/, checked by the TypeScript compiler first.
$ErrorActionPreference = 'Stop'

Push-Location $PSScriptRoot
try {
    # Every time: a pull may add packages, which an old node_modules lacks
    npm install
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    npm run build
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
