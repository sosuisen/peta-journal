# Builds the app into dist/, checked by the TypeScript compiler first.
$ErrorActionPreference = 'Stop'

Push-Location $PSScriptRoot
try {
    if (-not (Test-Path node_modules)) {
        npm install
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }
    npm run build
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
