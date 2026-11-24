# Builds the web frontend into dist/: the wasm of the core and the types of the border first,
# then the app, checked by the TypeScript compiler.
#
# The journal server need not restart: reload the browser after the build.
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
