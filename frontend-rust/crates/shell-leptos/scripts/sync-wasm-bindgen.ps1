# Trunk pre_build hook (Windows only, see Trunk*.toml).
# On Windows ARM64, Trunk cannot download a prebuilt wasm-bindgen, so install
# wasm-bindgen-cli matching the wasm-bindgen version in the workspace Cargo.lock.
# On x64 Windows this is a no-op: Trunk downloads the binary itself.
if ($env:PROCESSOR_ARCHITECTURE -ne 'ARM64') { exit 0 }

$manifest = (cargo locate-project --workspace --message-format plain)
$lock = Join-Path (Split-Path $manifest) 'Cargo.lock'
$text = Get-Content $lock -Raw
if ($text -notmatch 'name = "wasm-bindgen"\r?\nversion = "([^"]+)"') {
    Write-Host 'sync-wasm-bindgen: wasm-bindgen not found in Cargo.lock, skipping'
    exit 0
}
$want = $Matches[1]

$have = $null
if (Get-Command wasm-bindgen -ErrorAction SilentlyContinue) {
    $have = ((wasm-bindgen --version) -split ' ')[1]
}
if ($have -eq $want) { exit 0 }

Write-Host "sync-wasm-bindgen: installing wasm-bindgen-cli $want (installed: $have)"
cargo install wasm-bindgen-cli --version $want --locked
exit $LASTEXITCODE
