# Runs the mutation test of PetaBorder with Stryker.NET. The tests are PetaBorder.Tests, so they
# need a real journal server: set PETA_E2E_SERVER first. The report is in
# PetaBorder.Tests/StrykerOutput/<time>/reports/mutation-report.html.
$ErrorActionPreference = 'Stop'

if (-not $env:PETA_E2E_SERVER) {
    throw 'Set PETA_E2E_SERVER to journal-server.exe. Without it, the tests are skipped and every mutant survives.'
}

# Directory.Build.props reads this. See the comment there
$env:PetaMutation = 'true'
Push-Location (Join-Path $PSScriptRoot 'PetaBorder.Tests')
try {
    dotnet tool restore
    if ($LASTEXITCODE -ne 0) { throw 'dotnet tool restore failed' }
    dotnet stryker @args
    if ($LASTEXITCODE -ne 0) { throw 'Stryker.NET failed' }
}
finally {
    Pop-Location
    Remove-Item Env:PetaMutation
}
