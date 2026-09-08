[CmdletBinding()]
param(
    [switch]$Check
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$outputPath = Join-Path $repositoryRoot "THIRD-PARTY-NOTICES.txt"
$targetTriple = "x86_64-pc-windows-msvc"
$workspacePrefix = "$repositoryRoot\"
$cargoExecutable = (Get-Command cargo.exe -ErrorAction SilentlyContinue).Source

if ([string]::IsNullOrWhiteSpace($cargoExecutable)) {
    $userProfile = [Environment]::GetFolderPath([Environment+SpecialFolder]::UserProfile)
    $cargoExecutable = Join-Path $userProfile ".cargo\bin\cargo.exe"
}
if (-not (Test-Path -LiteralPath $cargoExecutable -PathType Leaf)) {
    throw "cargo.exe was not found."
}

function Get-NormalizedText {
    param([Parameter(Mandatory = $true)][string]$Path)

    $text = Get-Content -LiteralPath $Path -Raw
    $normalizedText = $text -replace "`r`n", "`n" -replace "`r", "`n"
    $trimmedLines = @(
        $normalizedText -split "`n" | ForEach-Object { $_.TrimEnd() }
    )
    return ($trimmedLines -join "`n").TrimEnd() + "`n"
}

function Get-TextHash {
    param([Parameter(Mandatory = $true)][string]$Text)

    $bytes = [Text.Encoding]::UTF8.GetBytes($Text)
    return [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant()
}

function Get-LicenseFiles {
    param([Parameter(Mandatory = $true)][string]$Directory)

    return @(
        Get-ChildItem -LiteralPath $Directory -File |
            Where-Object { $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE)(\.|-|_|$)' } |
            Sort-Object Name
    )
}

function Add-LicenseDocuments {
    param(
        [Parameter(Mandatory = $true)][hashtable]$Documents,
        [Parameter(Mandatory = $true)][string]$PackageLabel,
        [Parameter(Mandatory = $true)][string]$Directory
    )

    $files = @(Get-LicenseFiles -Directory $Directory)
    foreach ($file in $files) {
        $text = Get-NormalizedText -Path $file.FullName
        $hash = Get-TextHash -Text $text
        if (-not $Documents.ContainsKey($hash)) {
            $Documents[$hash] = [pscustomobject]@{
                Hash = $hash
                Text = $text
                Sources = [System.Collections.Generic.SortedSet[string]]::new(
                    [StringComparer]::Ordinal
                )
            }
        }
        [void]$Documents[$hash].Sources.Add("$PackageLabel / $($file.Name)")
    }
    return $files.Count
}

function Escape-TableCell {
    param([AllowEmptyString()][string]$Value)

    if ([string]::IsNullOrWhiteSpace($Value)) {
        return "-"
    }
    return $Value.Replace("|", "\|").Replace("`r", " ").Replace("`n", " ")
}

$metadataJson = & $cargoExecutable @(
    "metadata"
    "--offline"
    "--locked"
    "--filter-platform"
    $targetTriple
    "--format-version"
    "1"
)
if ($LASTEXITCODE -ne 0) {
    throw "cargo metadata failed with exit code $LASTEXITCODE."
}
$metadata = $metadataJson | ConvertFrom-Json

$nodesById = @{}
foreach ($node in $metadata.resolve.nodes) {
    $nodesById[[string]$node.id] = $node
}
$packagesById = @{}
foreach ($package in $metadata.packages) {
    $packagesById[[string]$package.id] = $package
}

$rootIds = @(
    $metadata.packages |
        Where-Object { $_.name -in @("moe-command-helper", "moe-desktop") } |
        ForEach-Object { [string]$_.id }
)
if ($rootIds.Count -ne 2) {
    throw "The two Windows native package roots were not resolved."
}

$resolvedIds = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
$pendingIds = [System.Collections.Generic.Queue[string]]::new()
foreach ($rootId in $rootIds) {
    $pendingIds.Enqueue($rootId)
}
while ($pendingIds.Count -gt 0) {
    $packageId = $pendingIds.Dequeue()
    if (-not $resolvedIds.Add($packageId)) {
        continue
    }
    foreach ($dependency in $nodesById[$packageId].deps) {
        $isNormalDependency = @(
            $dependency.dep_kinds | Where-Object { $null -eq $_.kind }
        ).Count -gt 0
        if ($isNormalDependency) {
            $pendingIds.Enqueue([string]$dependency.pkg)
        }
    }
}

$nativePackages = @(
    $resolvedIds |
        ForEach-Object { $packagesById[$_] } |
        Where-Object { -not ([string]$_.manifest_path).StartsWith($workspacePrefix) } |
        Sort-Object name, version
)
$nativeMissingLicense = @(
    $nativePackages |
        Where-Object {
            [string]::IsNullOrWhiteSpace([string]$_.license) -and
            [string]::IsNullOrWhiteSpace([string]$_.license_file)
        }
)
if ($nativeMissingLicense.Count -gt 0) {
    throw "Native packages without license metadata: $($nativeMissingLicense.name -join ', ')"
}

$packageLockPath = Join-Path $repositoryRoot "package-lock.json"
$packageLock = Get-Content -LiteralPath $packageLockPath -Raw | ConvertFrom-Json -AsHashtable
$desktopLockEntry = $packageLock.packages["apps/desktop"]
if ($null -eq $desktopLockEntry) {
    throw "apps/desktop was not found in package-lock.json."
}

$pendingNpmNames = [System.Collections.Generic.Queue[string]]::new()
foreach ($dependencyName in $desktopLockEntry.dependencies.Keys) {
    $pendingNpmNames.Enqueue([string]$dependencyName)
}
$seenNpmNames = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
$frontendPackages = [System.Collections.Generic.List[object]]::new()
while ($pendingNpmNames.Count -gt 0) {
    $dependencyName = $pendingNpmNames.Dequeue()
    if (-not $seenNpmNames.Add($dependencyName)) {
        continue
    }
    $lockPath = "node_modules/$dependencyName"
    $entry = $packageLock.packages[$lockPath]
    if ($null -eq $entry) {
        throw "The production npm dependency is missing from package-lock.json: $dependencyName"
    }
    if ([string]::IsNullOrWhiteSpace([string]$entry.license)) {
        throw "The production npm dependency has no license metadata: $dependencyName"
    }
    $frontendPackages.Add([pscustomobject]@{
        Name = $dependencyName
        Version = [string]$entry.version
        License = [string]$entry.license
        Directory = Join-Path $repositoryRoot ($lockPath.Replace("/", "\"))
    })
    if ($entry.ContainsKey("dependencies") -and $null -ne $entry.dependencies) {
        foreach ($transitiveName in $entry.dependencies.Keys) {
            $pendingNpmNames.Enqueue([string]$transitiveName)
        }
    }
}
$frontendPackages = @($frontendPackages | Sort-Object Name, Version)

$licenseDocuments = @{}
$packagesWithoutRootLicenseFile = [System.Collections.Generic.List[string]]::new()
foreach ($package in $nativePackages) {
    $packageLabel = "$($package.name) $($package.version)"
    $directory = Split-Path -Parent ([string]$package.manifest_path)
    if ((Add-LicenseDocuments -Documents $licenseDocuments -PackageLabel $packageLabel -Directory $directory) -eq 0) {
        $packagesWithoutRootLicenseFile.Add("$packageLabel ($($package.license))")
    }
}
foreach ($package in $frontendPackages) {
    $packageLabel = "$($package.Name) $($package.Version)"
    if ((Add-LicenseDocuments -Documents $licenseDocuments -PackageLabel $packageLabel -Directory $package.Directory) -eq 0) {
        $packagesWithoutRootLicenseFile.Add("$packageLabel ($($package.License))")
    }
}

$fontLicensePath = Join-Path $repositoryRoot "apps\desktop\src\assets\licenses\Pixelify-Sans-OFL.txt"
if (-not (Test-Path -LiteralPath $fontLicensePath -PathType Leaf)) {
    throw "The bundled Pixelify Sans license file is missing."
}
$fontLicenseText = Get-NormalizedText -Path $fontLicensePath
$fontLicenseHash = Get-TextHash -Text $fontLicenseText
if (-not $licenseDocuments.ContainsKey($fontLicenseHash)) {
    $licenseDocuments[$fontLicenseHash] = [pscustomobject]@{
        Hash = $fontLicenseHash
        Text = $fontLicenseText
        Sources = [System.Collections.Generic.SortedSet[string]]::new([StringComparer]::Ordinal)
    }
}
[void]$licenseDocuments[$fontLicenseHash].Sources.Add("Pixelify Sans / Pixelify-Sans-OFL.txt")

$lines = [System.Collections.Generic.List[string]]::new()
$lines.Add("M.I.O. THIRD-PARTY NOTICES")
$lines.Add("============================")
$lines.Add("")
$lines.Add("Generated by scripts/generate-third-party-notices.ps1 from Cargo.lock,")
$lines.Add("package-lock.json, locally cached package metadata, and bundled asset licenses.")
$lines.Add("Target: Windows x64 ($targetTriple)")
$lines.Add("Native roots: moe-desktop and moe-command-helper (normal dependencies only)")
$lines.Add("Do not edit this file manually.")
$lines.Add("")
$lines.Add("RUST NATIVE DEPENDENCIES ($($nativePackages.Count))")
$lines.Add("--------------------------------------")
foreach ($package in $nativePackages) {
    $repository = if ([string]::IsNullOrWhiteSpace([string]$package.repository)) {
        [string]$package.homepage
    }
    else {
        [string]$package.repository
    }
    $lines.Add("$($package.name) $($package.version) | $($package.license) | $repository")
}
$lines.Add("")
$lines.Add("WEB FRONTEND DEPENDENCIES ($($frontendPackages.Count))")
$lines.Add("-------------------------------------")
foreach ($package in $frontendPackages) {
    $lines.Add("$($package.Name) $($package.Version) | $($package.License)")
}
$lines.Add("")
$lines.Add("BUNDLED ASSET")
$lines.Add("-------------")
$lines.Add("Pixelify Sans | OFL-1.1")
$lines.Add("")
$lines.Add("PACKAGES WITHOUT A ROOT LICENSE OR NOTICE FILE")
$lines.Add("----------------------------------------------")
$lines.Add("These packages have declared license metadata but did not include a root-level")
$lines.Add("LICENSE, LICENCE, COPYING, or NOTICE file in the locally resolved package.")
foreach ($packageLabel in ($packagesWithoutRootLicenseFile | Sort-Object)) {
    $lines.Add($packageLabel)
}
$lines.Add("")
$lines.Add("LICENSE AND NOTICE TEXTS ($($licenseDocuments.Count) UNIQUE DOCUMENTS)")
$lines.Add("--------------------------------------------------")
foreach ($document in ($licenseDocuments.Values | Sort-Object Hash)) {
    $lines.Add("")
    $lines.Add("DOCUMENT SHA-256: $($document.Hash)")
    $lines.Add("Used by:")
    foreach ($source in $document.Sources) {
        $lines.Add("- $source")
    }
    $lines.Add("----- BEGIN LICENSE OR NOTICE TEXT -----")
    foreach ($textLine in ($document.Text.TrimEnd("`n") -split "`n", 0, "SimpleMatch")) {
        $lines.Add($textLine)
    }
    $lines.Add("----- END LICENSE OR NOTICE TEXT -----")
}

$generatedText = ($lines -join "`n") + "`n"
if ($Check) {
    if (-not (Test-Path -LiteralPath $outputPath -PathType Leaf)) {
        throw "THIRD-PARTY-NOTICES.txt is missing. Run scripts/generate-third-party-notices.ps1."
    }
    $existingText = Get-NormalizedText -Path $outputPath
    if ($existingText -cne $generatedText) {
        throw "THIRD-PARTY-NOTICES.txt is stale. Run scripts/generate-third-party-notices.ps1."
    }
    Write-Host "THIRD-PARTY-NOTICES.txt is current."
    exit 0
}

[IO.File]::WriteAllText($outputPath, $generatedText, [Text.UTF8Encoding]::new($false))
Write-Host "Generated: $outputPath"
Write-Host "Rust native dependencies: $($nativePackages.Count)"
Write-Host "Web frontend dependencies: $($frontendPackages.Count)"
Write-Host "Unique license and notice documents: $($licenseDocuments.Count)"
