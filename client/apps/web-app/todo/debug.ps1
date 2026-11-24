# Starts the development server of Vite, at http://localhost:5180. What is given goes to Vite.
#
# The journal server has to run already. Open http://localhost:5180/?port=8087 to call it.
$ErrorActionPreference = 'Stop'

Push-Location $PSScriptRoot
try {
    # Every time: a pull may add packages, which an old node_modules lacks
    npm install
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    npm run dev -- @args
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
