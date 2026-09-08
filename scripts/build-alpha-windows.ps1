[CmdletBinding()]
param(
    [switch]$Installer,
    [string]$SigningCertificateThumbprint,
    [string]$TimestampUrl
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$targetTriple = "x86_64-pc-windows-msvc"
$executablePath = Join-Path $repositoryRoot "target\$targetTriple\release\mio-desktop.exe"
$commandHelperPath = Join-Path $repositoryRoot "target\$targetTriple\release\mio-command-helper.exe"
$sidecarDirectory = Join-Path $repositoryRoot ".tools\tauri-sidecars"
$stagedCommandHelperPath = Join-Path $sidecarDirectory "mio-command-helper-$targetTriple.exe"
$desktopRoot = Join-Path $repositoryRoot "apps\desktop"
$tauriExecutable = Join-Path $repositoryRoot "node_modules\.bin\tauri.cmd"
$tauriInstallerConfig = Join-Path $desktopRoot "src-tauri\tauri.installer.conf.json"
$thirdPartyNoticeScript = Join-Path $repositoryRoot "scripts\generate-third-party-notices.ps1"
$rustFlagsVariable = "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS"
$commandHelperHashVariable = "MOE_BUNDLED_COMMAND_HELPER_SHA256"
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
$previousCommandHelperHash = [Environment]::GetEnvironmentVariable($commandHelperHashVariable, "Process")
$previousPath = [Environment]::GetEnvironmentVariable($pathVariable, "Process")
$buildPath = if ([string]::IsNullOrEmpty($previousPath)) {
    $cargoDirectory
}
else {
    "$cargoDirectory$([IO.Path]::PathSeparator)$previousPath"
}
$locationPushed = $false
$temporaryInstallerConfig = $null

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
    throw "The local Tauri CLI was not found. Run npm ci before building the Windows executable."
}
if ($Installer -and -not (Test-Path -LiteralPath $tauriInstallerConfig -PathType Leaf)) {
    throw "The Windows installer configuration was not found: $tauriInstallerConfig"
}
if ($Installer -and -not (Test-Path -LiteralPath $thirdPartyNoticeScript -PathType Leaf)) {
    throw "The third-party notice generator was not found: $thirdPartyNoticeScript"
}

$signingRequested = `
    -not [string]::IsNullOrWhiteSpace($SigningCertificateThumbprint) -or `
    -not [string]::IsNullOrWhiteSpace($TimestampUrl)
if ($signingRequested -and -not $Installer) {
    throw "Windows signing parameters can only be used with -Installer."
}
if ($signingRequested) {
    if ([string]::IsNullOrWhiteSpace($SigningCertificateThumbprint) -or
        [string]::IsNullOrWhiteSpace($TimestampUrl)) {
        throw "Supply both -SigningCertificateThumbprint and -TimestampUrl."
    }

    $normalizedThumbprint = $SigningCertificateThumbprint.Replace(" ", "").ToUpperInvariant()
    if ($normalizedThumbprint -notmatch '^[0-9A-F]{40}$') {
        throw "The signing certificate thumbprint must contain exactly 40 hexadecimal characters."
    }
    $parsedTimestampUrl = $null
    if (-not [Uri]::TryCreate($TimestampUrl, [UriKind]::Absolute, [ref]$parsedTimestampUrl) -or
        $parsedTimestampUrl.Scheme -cne "https") {
        throw "The RFC 3161 timestamp URL must be an absolute HTTPS URL."
    }

    $signingConfig = Get-Content -LiteralPath $tauriInstallerConfig -Raw | ConvertFrom-Json
    $signingConfig.bundle.windows | Add-Member `
        -NotePropertyName certificateThumbprint `
        -NotePropertyValue $normalizedThumbprint `
        -Force
    $signingConfig.bundle.windows | Add-Member `
        -NotePropertyName digestAlgorithm `
        -NotePropertyValue "sha256" `
        -Force
    $signingConfig.bundle.windows | Add-Member `
        -NotePropertyName timestampUrl `
        -NotePropertyValue $parsedTimestampUrl.AbsoluteUri `
        -Force
    $signingConfig.bundle.windows | Add-Member `
        -NotePropertyName tsp `
        -NotePropertyValue $true `
        -Force
    $temporaryInstallerConfig = Join-Path `
        ([IO.Path]::GetTempPath()) `
        "mio-tauri-signing-$PID-$([guid]::NewGuid().ToString('N')).json"
    $signingConfig | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $temporaryInstallerConfig -Encoding UTF8
}

$tauriArguments = @("build")
if ($Installer) {
    $effectiveInstallerConfig = if ($null -ne $temporaryInstallerConfig) {
        $temporaryInstallerConfig
    }
    else {
        $tauriInstallerConfig
    }
    $tauriArguments += @(
        "--bundles"
        "nsis"
        "--config"
        $effectiveInstallerConfig
    )
    if (-not $signingRequested) {
        $tauriArguments += "--no-sign"
    }
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
    [Environment]::SetEnvironmentVariable($commandHelperHashVariable, $null, "Process")
    if ($Installer) {
        & $thirdPartyNoticeScript -Check
        & $cargoExecutable @(
            "build"
            "--package"
            "moe-command-helper"
            "--bin"
            "mio-command-helper"
            "--release"
            "--target"
            $targetTriple
            "--locked"
        )
        if ($LASTEXITCODE -ne 0) {
            throw "The M.I.O. command helper build failed with exit code $LASTEXITCODE."
        }
        if (-not (Test-Path -LiteralPath $commandHelperPath -PathType Leaf)) {
            throw "The expected command helper was not created: $commandHelperPath"
        }
        New-Item -ItemType Directory -Path $sidecarDirectory -Force | Out-Null
        Copy-Item -LiteralPath $commandHelperPath -Destination $stagedCommandHelperPath -Force
        $stagedHelperHash = Get-FileHash -Algorithm SHA256 -LiteralPath $stagedCommandHelperPath
        [Environment]::SetEnvironmentVariable(
            $commandHelperHashVariable,
            $stagedHelperHash.Hash,
            "Process"
        )
        Write-Host "M.I.O. command helper sidecar: $stagedCommandHelperPath"
        Write-Host "Command helper SHA-256: $($stagedHelperHash.Hash)"
    }
    Push-Location -LiteralPath $desktopRoot
    $locationPushed = $true

    & $tauriExecutable @tauriArguments
    if ($LASTEXITCODE -ne 0) {
        $artifactKind = if ($Installer) { "installer" } else { "executable" }
        throw "The Windows $artifactKind build failed with exit code $LASTEXITCODE."
    }

    if (-not (Test-Path -LiteralPath $executablePath -PathType Leaf)) {
        throw "The expected executable was not created: $executablePath"
    }

    if ($Installer) {
        $baseTauriConfig = Get-Content -LiteralPath `
            (Join-Path $desktopRoot "src-tauri\tauri.conf.json") -Raw | ConvertFrom-Json
        $installerTauriConfig = Get-Content -LiteralPath $tauriInstallerConfig -Raw | ConvertFrom-Json
        $releaseVersion = [string](
            $baseTauriConfig.version
        )
        $installerProductName = [string]$installerTauriConfig.productName
        if ([string]::IsNullOrWhiteSpace($installerProductName) -or $installerProductName.EndsWith(".")) {
            throw "The Windows installer productName must be present and must not end with a period."
        }
        $artifactPath = Join-Path `
            $repositoryRoot `
            "target\$targetTriple\release\bundle\nsis\$($installerProductName)_$($releaseVersion)_x64-setup.exe"
    }
    else {
        $artifactPath = $executablePath
    }

    if (-not (Test-Path -LiteralPath $artifactPath -PathType Leaf)) {
        throw "The expected artifact was not created: $artifactPath"
    }

    $artifactHash = Get-FileHash -Algorithm SHA256 -LiteralPath $artifactPath
    $artifactLabel = if ($Installer) { "installer" } else { "executable" }
    Write-Host "M.I.O. Windows $artifactLabel`: $artifactPath"
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
    Restore-ProcessEnvironmentVariable `
        -Name $commandHelperHashVariable `
        -Value $previousCommandHelperHash
    Restore-ProcessEnvironmentVariable -Name $pathVariable -Value $previousPath
    if ($null -ne $temporaryInstallerConfig) {
        Remove-Item -LiteralPath $temporaryInstallerConfig -Force -ErrorAction SilentlyContinue
    }
}
