# Starts the development server of Vite, at http://localhost:5180. What is given goes to Vite.
#
# The journal server has to run already.
$ErrorActionPreference = 'Stop'

Push-Location $PSScriptRoot
try {
    if (-not (Test-Path node_modules)) {
        npm install
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }
    npm run dev -- @args
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
