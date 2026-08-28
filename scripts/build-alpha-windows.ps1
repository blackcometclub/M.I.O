[CmdletBinding()]
param(
    [switch]$Installer
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$targetTriple = "x86_64-pc-windows-msvc"
$executablePath = Join-Path $repositoryRoot "target\$targetTriple\release\moe-desktop.exe"
$desktopRoot = Join-Path $repositoryRoot "apps\desktop"
$tauriExecutable = Join-Path $repositoryRoot "node_modules\.bin\tauri.cmd"
$tauriInstallerConfig = Join-Path $desktopRoot "src-tauri\tauri.installer.conf.json"
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

function Restore-ProcessEnvironmentVariable {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [AllowNull()]
        $Value
    )

    if ($null -eq $Value) {
        Remove-Item -LiteralPath "Env:$Name" -ErrorAction SilentlyContinue
        return
    }

    [Environment]::SetEnvironmentVariable($Name, $Value, "Process")
}

if (-not (Test-Path -LiteralPath $tauriExecutable -PathType Leaf)) {
    throw "The local Tauri CLI was not found. Run npm ci before building the Windows alpha executable."
}
if ($Installer -and -not (Test-Path -LiteralPath $tauriInstallerConfig -PathType Leaf)) {
    throw "The Windows installer configuration was not found: $tauriInstallerConfig"
}

$tauriArguments = @("build")
if ($Installer) {
    $tauriArguments += @(
        "--bundles"
        "nsis"
        "--config"
        $tauriInstallerConfig
        "--no-sign"
    )
}
else {
    $tauriArguments += "--no-bundle"
}
$tauriArguments += @(
    "--target"
    $targetTriple
    "--runner"
    $cargoExecutable
    "--"
    "--locked"
)

try {
    [Environment]::SetEnvironmentVariable($pathVariable, $buildPath, "Process")
    [Environment]::SetEnvironmentVariable($rustFlagsVariable, $staticCrtFlags, "Process")
    Push-Location -LiteralPath $desktopRoot
    $locationPushed = $true

    & $tauriExecutable @tauriArguments
    if ($LASTEXITCODE -ne 0) {
        $artifactKind = if ($Installer) { "installer" } else { "executable" }
        throw "The Windows alpha $artifactKind build failed with exit code $LASTEXITCODE."
    }

    if (-not (Test-Path -LiteralPath $executablePath -PathType Leaf)) {
        throw "The expected executable was not created: $executablePath"
    }

    if ($Installer) {
        $releaseVersion = [string](
            Get-Content -LiteralPath (Join-Path $desktopRoot "src-tauri\tauri.conf.json") -Raw |
                ConvertFrom-Json
        ).version
        $artifactPath = Join-Path `
            $repositoryRoot `
            "target\$targetTriple\release\bundle\nsis\M.I.O._$($releaseVersion)_x64-setup.exe"
    }
    else {
        $artifactPath = $executablePath
    }

    if (-not (Test-Path -LiteralPath $artifactPath -PathType Leaf)) {
        throw "The expected artifact was not created: $artifactPath"
    }

    $artifactHash = Get-FileHash -Algorithm SHA256 -LiteralPath $artifactPath
    $artifactLabel = if ($Installer) { "installer" } else { "executable" }
    Write-Host "M.I.O. Windows alpha $artifactLabel`: $artifactPath"
    if ($Installer) {
        $signature = Get-AuthenticodeSignature -LiteralPath $artifactPath
        Write-Host "Signature status: $($signature.Status)"
    }
    Write-Host "SHA-256: $($artifactHash.Hash)"
}
finally {
    if ($locationPushed) {
        Pop-Location
    }
    Restore-ProcessEnvironmentVariable -Name $rustFlagsVariable -Value $previousRustFlags
    Restore-ProcessEnvironmentVariable -Name $pathVariable -Value $previousPath
}
