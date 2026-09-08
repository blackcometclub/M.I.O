[CmdletBinding()]
param(
    [string]$PackageIdentityName = "TINMOON.M.I.O",
    [string]$Publisher = "CN=0D8FEB2D-BBD6-4178-96F3-44C562FA2BA7",
    [string]$PublisherDisplayName = "TINMOON",
    [string]$PackageVersion,
    [string]$OutputDirectory
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$packageJsonPath = Join-Path $repositoryRoot "package.json"
$productVersion = [string](
    Get-Content -LiteralPath $packageJsonPath -Raw |
    ConvertFrom-Json
).version
$targetTriple = "x86_64-pc-windows-msvc"
$desktopExecutable = Join-Path $repositoryRoot "target\$targetTriple\release\mio-desktop.exe"
$commandHelper = Join-Path $repositoryRoot "target\$targetTriple\release\mio-command-helper.exe"
$manifestTemplate = Join-Path `
    $repositoryRoot `
    "apps\desktop\src-tauri\msix\AppxManifest.template.xml"
$iconDirectory = Join-Path $repositoryRoot "apps\desktop\src-tauri\icons"
$windowsKitBin = "C:\Program Files (x86)\Windows Kits\10\bin"

if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path $repositoryRoot ".tools\store-msix"
}

if ($productVersion -notmatch '^\d+\.\d+\.\d+$') {
    throw "package.json version must contain three numeric fields."
}
if ([string]::IsNullOrWhiteSpace($PackageVersion)) {
    $PackageVersion = "$productVersion.0"
}

if ($PackageIdentityName -notmatch '^[A-Za-z0-9.-]{3,50}$') {
    throw "PackageIdentityName must contain 3-50 letters, digits, periods, or hyphens."
}
if ($PackageVersion -notmatch '^\d+\.\d+\.\d+\.\d+$') {
    throw "PackageVersion must contain four numeric fields, for example 1.0.0.0."
}
$storeProductVersion = $PackageVersion.Split('.')[0..2] -join '.'
if ($storeProductVersion -ne $productVersion) {
    throw "PackageVersion $PackageVersion must match product version $productVersion in its first three fields."
}
foreach ($field in $PackageVersion.Split('.')) {
    if ([int]$field -gt 65535) {
        throw "Every PackageVersion field must be between 0 and 65535."
    }
}

$requiredFiles = @(
    $desktopExecutable,
    $commandHelper,
    $manifestTemplate,
    (Join-Path $repositoryRoot "LICENSE"),
    (Join-Path $repositoryRoot "THIRD-PARTY-NOTICES.txt"),
    (Join-Path $iconDirectory "StoreLogo.png"),
    (Join-Path $iconDirectory "Square44x44Logo.png"),
    (Join-Path $iconDirectory "Square71x71Logo.png"),
    (Join-Path $iconDirectory "Square150x150Logo.png")
)
foreach ($path in $requiredFiles) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required MSIX input was not found: $path"
    }
}

if (-not (Test-Path -LiteralPath $windowsKitBin -PathType Container)) {
    throw "The Windows SDK bin directory was not found: $windowsKitBin"
}
$makeAppxCandidates = Get-ChildItem `
    -LiteralPath $windowsKitBin `
    -Filter "makeappx.exe" `
    -Recurse `
    -File `
    -ErrorAction SilentlyContinue |
    Where-Object { $_.Directory.Name -eq "x64" } |
    Sort-Object FullName -Descending
$makeAppx = $makeAppxCandidates | Select-Object -First 1
if ($null -eq $makeAppx) {
    throw "The x64 MakeAppx.exe was not found in the Windows SDK."
}

New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$resolvedOutputDirectory = (Resolve-Path -LiteralPath $OutputDirectory).Path
$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$packagePath = Join-Path `
    $resolvedOutputDirectory `
    "M.I.O_$($PackageVersion)_x64_store-unsigned_$timestamp.msix"
$hashPath = "$packagePath.sha256"

$layoutDirectory = Join-Path `
    ([IO.Path]::GetTempPath()) `
    "mio-msix-layout-$PID-$([guid]::NewGuid().ToString('N'))"
$verifyDirectory = Join-Path `
    ([IO.Path]::GetTempPath()) `
    "mio-msix-verify-$PID-$([guid]::NewGuid().ToString('N'))"

function Escape-XmlAttribute {
    param([Parameter(Mandatory = $true)][string]$Value)

    return [Security.SecurityElement]::Escape($Value)
}

try {
    $assetsDirectory = Join-Path $layoutDirectory "Assets"
    New-Item -ItemType Directory -Path $assetsDirectory -Force | Out-Null

    Copy-Item -LiteralPath $desktopExecutable -Destination (Join-Path $layoutDirectory "mio-desktop.exe")
    Copy-Item -LiteralPath $commandHelper -Destination (Join-Path $layoutDirectory "mio-command-helper.exe")
    Copy-Item -LiteralPath (Join-Path $repositoryRoot "LICENSE") -Destination $layoutDirectory
    Copy-Item `
        -LiteralPath (Join-Path $repositoryRoot "THIRD-PARTY-NOTICES.txt") `
        -Destination $layoutDirectory
    foreach ($iconName in @(
        "StoreLogo.png",
        "Square44x44Logo.png",
        "Square71x71Logo.png",
        "Square150x150Logo.png"
    )) {
        Copy-Item -LiteralPath (Join-Path $iconDirectory $iconName) -Destination $assetsDirectory
    }

    $manifest = Get-Content -LiteralPath $manifestTemplate -Raw
    $manifest = $manifest.Replace(
        "__PACKAGE_IDENTITY_NAME__",
        (Escape-XmlAttribute $PackageIdentityName)
    )
    $manifest = $manifest.Replace("__PUBLISHER__", (Escape-XmlAttribute $Publisher))
    $manifest = $manifest.Replace(
        "__PUBLISHER_DISPLAY_NAME__",
        (Escape-XmlAttribute $PublisherDisplayName)
    )
    $manifest = $manifest.Replace("__PACKAGE_VERSION__", $PackageVersion)
    [IO.File]::WriteAllText(
        (Join-Path $layoutDirectory "AppxManifest.xml"),
        $manifest,
        [Text.UTF8Encoding]::new($false)
    )

    & $makeAppx.FullName pack /d $layoutDirectory /p $packagePath
    if ($LASTEXITCODE -ne 0) {
        throw "MakeAppx pack failed with exit code $LASTEXITCODE."
    }

    & $makeAppx.FullName unpack /p $packagePath /d $verifyDirectory
    if ($LASTEXITCODE -ne 0) {
        throw "MakeAppx unpack verification failed with exit code $LASTEXITCODE."
    }
    foreach ($relativePath in @(
        "AppxManifest.xml",
        "LICENSE",
        "THIRD-PARTY-NOTICES.txt",
        "mio-desktop.exe",
        "mio-command-helper.exe",
        "Assets\StoreLogo.png",
        "Assets\Square44x44Logo.png",
        "Assets\Square71x71Logo.png",
        "Assets\Square150x150Logo.png"
    )) {
        $verifiedPath = Join-Path $verifyDirectory $relativePath
        if (-not (Test-Path -LiteralPath $verifiedPath -PathType Leaf)) {
            throw "The packed MSIX is missing a required file: $relativePath"
        }
    }

    $packageHash = Get-FileHash -Algorithm SHA256 -LiteralPath $packagePath
    [IO.File]::WriteAllText(
        $hashPath,
        "$($packageHash.Hash)  $([IO.Path]::GetFileName($packagePath))`r`n",
        [Text.UTF8Encoding]::new($false)
    )

    Write-Host "M.I.O. unsigned Store MSIX: $packagePath"
    Write-Host "SHA-256: $($packageHash.Hash)"
    Write-Host "Identity: $PackageIdentityName"
    Write-Host "Publisher: $Publisher"
    Write-Warning "This package is unsigned. Do not sideload it. It is intended for Microsoft Store submission checks only."
}
finally {
    foreach ($temporaryDirectory in @($layoutDirectory, $verifyDirectory)) {
        if (Test-Path -LiteralPath $temporaryDirectory -PathType Container) {
            Remove-Item -LiteralPath $temporaryDirectory -Recurse -Force
        }
    }
}
