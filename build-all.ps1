#!/usr/bin/env pwsh
# PowerShell script to build all PetaJournal components on Windows

$ErrorActionPreference = "Stop"

Write-Host "=====================================" -ForegroundColor Cyan
Write-Host "Building PetaJournal - Windows Build" -ForegroundColor Cyan
Write-Host "=====================================" -ForegroundColor Cyan
Write-Host ""

$ROOT_DIR = $PSScriptRoot

# Function to check if command exists
function Test-CommandExists {
    param($command)
    $null = Get-Command $command -ErrorAction SilentlyContinue
    return $?
}

# Check prerequisites
Write-Host "Checking prerequisites..." -ForegroundColor Yellow

$missing = @()
if (-not (Test-CommandExists "java")) { $missing += "Java 25" }
if (-not (Test-CommandExists "mvn")) { $missing += "Maven" }
if (-not (Test-CommandExists "node")) { $missing += "Node.js" }
if (-not (Test-CommandExists "npm")) { $missing += "npm" }
if (-not (Test-CommandExists "cmake")) { $missing += "CMake" }

if ($missing.Count -gt 0) {
    Write-Host "Missing prerequisites: $($missing -join ', ')" -ForegroundColor Red
    Write-Host "Please install missing components. See INSTALL_WINDOWS.md" -ForegroundColor Red
    exit 1
}

Write-Host "All prerequisites found!" -ForegroundColor Green
Write-Host ""

# Step 1: Build Proto
Write-Host "Step 1/4: Building Proto definitions..." -ForegroundColor Cyan
Set-Location (Join-Path $ROOT_DIR "proto")
& mvn clean install
if ($LASTEXITCODE -ne 0) {
    Write-Host "Proto build failed!" -ForegroundColor Red
    exit 1
}
Write-Host "Proto build complete!" -ForegroundColor Green
Write-Host ""

# Step 2: Build Markdown Parser
Write-Host "Step 2/4: Building Markdown Parser..." -ForegroundColor Cyan
Set-Location (Join-Path $ROOT_DIR "markdown-parser")

$BUILD_DIR = "build"
if (Test-Path $BUILD_DIR) {
    Write-Host "Cleaning previous build..." -ForegroundColor Yellow
    Remove-Item -Recurse -Force $BUILD_DIR
}
New-Item -ItemType Directory -Path $BUILD_DIR | Out-Null
Set-Location $BUILD_DIR

# Auto-detect Visual Studio or use MinGW
if (Test-CommandExists "cl") {
    Write-Host "Using MSVC compiler..." -ForegroundColor Yellow
    & cmake .. -G "Visual Studio 18 2026" -A x64
    if ($LASTEXITCODE -ne 0) {
        Write-Host "CMake configuration failed!" -ForegroundColor Red
        exit 1
    }
    & cmake --build . --config Release
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Markdown Parser build failed!" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "Using MinGW compiler..." -ForegroundColor Yellow
    & cmake .. -G "MinGW Makefiles" -DCMAKE_BUILD_TYPE=Release
    if ($LASTEXITCODE -ne 0) {
        Write-Host "CMake configuration failed!" -ForegroundColor Red
        exit 1
    }
    & cmake --build .
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Markdown Parser build failed!" -ForegroundColor Red
        exit 1
    }
}

# Run tests
Write-Host "Running markdown parser tests..." -ForegroundColor Yellow
if (Test-CommandExists "cl") {
    & ctest -C Release --output-on-failure
} else {
    & ctest --output-on-failure
}
if ($LASTEXITCODE -ne 0) {
    Write-Host "Markdown Parser tests failed!" -ForegroundColor Red
    exit 1
}

Write-Host "Markdown Parser build complete!" -ForegroundColor Green

# Copy DLL to journal-server
Write-Host "Copying markdown parser DLL to journal-server..." -ForegroundColor Yellow
$LIB_DIR = Join-Path $ROOT_DIR "journal-server\lib\windows"
if (-not (Test-Path $LIB_DIR)) {
    New-Item -ItemType Directory -Path $LIB_DIR | Out-Null
}

if (Test-CommandExists "cl") {
    # MSVC build
    $DLL_SRC = "Release\peta_markdown.dll"
} else {
    # MinGW build
    $DLL_SRC = "libpeta_markdown.dll"
}

$DLL_DEST = Join-Path $LIB_DIR "libpeta_markdown.dll"
if (Test-Path $DLL_SRC) {
    Copy-Item $DLL_SRC $DLL_DEST -Force
    Write-Host "DLL copied successfully!" -ForegroundColor Green
} else {
    Write-Host "Warning: DLL not found at $DLL_SRC" -ForegroundColor Yellow
}
Write-Host ""

# Step 3: Build Journal Server
Write-Host "Step 3/4: Building Journal Server..." -ForegroundColor Cyan
Set-Location (Join-Path $ROOT_DIR "journal-server")
& mvn clean package
if ($LASTEXITCODE -ne 0) {
    Write-Host "Journal Server build failed!" -ForegroundColor Red
    exit 1
}
Write-Host "Journal Server build complete!" -ForegroundColor Green
Write-Host ""

# Step 4: Build Frontend
Write-Host "Step 4/4: Building Frontend..." -ForegroundColor Cyan
Set-Location (Join-Path $ROOT_DIR "frontend")

Write-Host "Installing npm dependencies..." -ForegroundColor Yellow
& npm install --legacy-peer-deps
if ($LASTEXITCODE -ne 0) {
    Write-Host "npm install failed!" -ForegroundColor Red
    exit 1
}

Write-Host "Building Angular app..." -ForegroundColor Yellow
& npm run build
if ($LASTEXITCODE -ne 0) {
    Write-Host "Frontend build failed!" -ForegroundColor Red
    exit 1
}

Write-Host "Frontend build complete!" -ForegroundColor Green
Write-Host ""

# Success
Set-Location $ROOT_DIR
Write-Host "=====================================" -ForegroundColor Green
Write-Host "All components built successfully!" -ForegroundColor Green
Write-Host "=====================================" -ForegroundColor Green
Write-Host ""
Write-Host "To run the application:" -ForegroundColor Cyan
Write-Host "  1. Start journal server: cd journal-server && mvn quarkus:dev" -ForegroundColor White
Write-Host "  2. Start frontend: cd frontend && npm start" -ForegroundColor White
Write-Host "  3. Open browser: http://localhost:4200" -ForegroundColor White
