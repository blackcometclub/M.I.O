[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$targetTriple = "x86_64-pc-windows-msvc"
$artifactPath = Join-Path $repositoryRoot "target\$targetTriple\release\moe-desktop.exe"
$desktopRoot = Join-Path $repositoryRoot "apps\desktop"
$tauriExecutable = Join-Path $repositoryRoot "node_modules\.bin\tauri.cmd"
$rustFlagsVariable = "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS"
$staticCrtFlags = "-C target-feature=+crt-static"
$pathVariable = "PATH"
$cargoCommand = Get-Command cargo.exe -ErrorAction SilentlyContinue

if ($null -ne $cargoCommand) {
    $cargoExecutable = $cargoCommand.Source
}
else {
    $userProfile = [Environment]::GetFolderPath([Environment+SpecialFolder]::UserProfile)
    $cargoExecutable = Join-Path $userProfile ".cargo\bin\cargo.exe"
    if (-not (Test-Path -LiteralPath $cargoExecutable -PathType Leaf)) {
        throw "cargo.exe was not found in PATH or the standard Rustup user directory."
    }
}

$cargoDirectory = Split-Path -Parent $cargoExecutable
$previousRustFlags = [Environment]::GetEnvironmentVariable($rustFlagsVariable, "Process")
$previousPath = [Environment]::GetEnvironmentVariable($pathVariable, "Process")
$buildPath = if ([string]::IsNullOrEmpty($previousPath)) {
    $cargoDirectory
}
else {
    "$cargoDirectory$([IO.Path]::PathSeparator)$previousPath"
}
$locationPushed = $false

if (-not (Test-Path -LiteralPath $tauriExecutable -PathType Leaf)) {
    throw "The local Tauri CLI was not found. Run npm ci before building the Windows alpha executable."
}

try {
    [Environment]::SetEnvironmentVariable($pathVariable, $buildPath, "Process")
    [Environment]::SetEnvironmentVariable($rustFlagsVariable, $staticCrtFlags, "Process")
    Push-Location -LiteralPath $desktopRoot
    $locationPushed = $true

    & $tauriExecutable `
        build `
        --no-bundle `
        --target $targetTriple `
        --runner $cargoExecutable `
        -- `
        --locked
    if ($LASTEXITCODE -ne 0) {
        throw "The Windows alpha executable build failed with exit code $LASTEXITCODE."
    }

    if (-not (Test-Path -LiteralPath $artifactPath -PathType Leaf)) {
        throw "The expected artifact was not created: $artifactPath"
    }

    $artifactHash = Get-FileHash -Algorithm SHA256 -LiteralPath $artifactPath
    Write-Host "M.I.O. Windows alpha executable: $artifactPath"
    Write-Host "SHA-256: $($artifactHash.Hash)"
}
finally {
    if ($locationPushed) {
        Pop-Location
    }
    [Environment]::SetEnvironmentVariable($rustFlagsVariable, $previousRustFlags, "Process")
    [Environment]::SetEnvironmentVariable($pathVariable, $previousPath, "Process")
}
