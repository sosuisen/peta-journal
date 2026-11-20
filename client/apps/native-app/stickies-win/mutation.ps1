# Runs the mutation test of PetaBorder with Stryker.NET. The tests are PetaBorder.Tests, so they
# need a real journal server: set PETA_E2E_SERVER first. The report is in
# PetaBorder.Tests/StrykerOutput/<time>/reports/mutation-report.html.
# A test host that crashes leaves its journal server on the port of the tests. Stop it before
# the next run.
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
    # Why --log-to-file: without it, Stryker does not open the standard output and error of a
    # test host at all. The core of a test then writes its errors (eprintln!) to a handle that
    # is not its standard error. When the write fails, eprintln! panics and the host ends with
    # 0xC0000409 in the initial test run. The output of each host goes to
    # StrykerOutput/<time>/logs/test-servers instead.
    dotnet stryker --log-to-file @args
    if ($LASTEXITCODE -ne 0) { throw 'Stryker.NET failed' }
}
finally {
    Pop-Location
    Remove-Item Env:PetaMutation
}
