[CmdletBinding()]
param(
    [string]$Commit = "HEAD",
    [string]$Destination,
    [switch]$SkipChecks,
    [switch]$SkipInstaller,
    [string]$GitleaksPath,
    [string]$ExpectedSignerSubject,
    [string]$SigningCertificateThumbprint,
    [string]$TimestampUrl,
    [string]$SignToolPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$repositoryRootForGit = $repositoryRoot.Replace("\", "/")
$exportScript = Join-Path $PSScriptRoot "export-public-alpha.ps1"
$buildScript = Join-Path $PSScriptRoot "build-alpha-windows.ps1"
$signatureTestScript = Join-Path $PSScriptRoot "test-windows-release-signature.ps1"

function Invoke-RepositoryGit {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$GitArguments
    )

    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = @(
            & git.exe `
                -c "safe.directory=$repositoryRootForGit" `
                -c core.excludesFile=NUL `
                -c core.quotepath=false `
                -C $repositoryRoot `
                @GitArguments 2>&1
        )
        $gitExitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }

    if ($gitExitCode -ne 0) {
        throw "git $($GitArguments -join ' ') failed:`n$($output -join [Environment]::NewLine)"
    }

    return $output | ForEach-Object { $_.ToString() }
}

function Invoke-CheckedCommand {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Command,
        [string[]]$Arguments = @()
    )

    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Command failed with exit code $LASTEXITCODE."
    }
}

function Assert-NoTrackedChanges {
    & git.exe `
        -c "safe.directory=$repositoryRootForGit" `
        -c core.excludesFile=NUL `
        -C $repositoryRoot `
        diff `
        --quiet `
        --
    if ($LASTEXITCODE -eq 1) {
        throw "Tracked working-tree changes exist. Commit them before preparing a public release."
    }
    if ($LASTEXITCODE -ne 0) {
        throw "git diff failed with exit code $LASTEXITCODE."
    }

    & git.exe `
        -c "safe.directory=$repositoryRootForGit" `
        -c core.excludesFile=NUL `
        -C $repositoryRoot `
        diff `
        --cached `
        --quiet `
        --
    if ($LASTEXITCODE -eq 1) {
        throw "Staged changes exist. Commit them before preparing a public release."
    }
    if ($LASTEXITCODE -ne 0) {
        throw "git diff --cached failed with exit code $LASTEXITCODE."
    }
}

function Get-CommittedFileContent {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ResolvedCommit,
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    return @(
        Invoke-RepositoryGit -GitArguments @(
            "show"
            "$ResolvedCommit`:$Path"
        )
    ) -join [Environment]::NewLine
}

function Resolve-CargoExecutable {
    $cargoCommand = Get-Command cargo.exe -ErrorAction SilentlyContinue
    if ($null -ne $cargoCommand) {
        return $cargoCommand.Source
    }

    $userProfile = [Environment]::GetFolderPath([Environment+SpecialFolder]::UserProfile)
    $standardCargo = Join-Path $userProfile ".cargo\bin\cargo.exe"
    if (Test-Path -LiteralPath $standardCargo -PathType Leaf) {
        return $standardCargo
    }

    throw "cargo.exe was not found in PATH or the standard Rustup user directory."
}

Assert-NoTrackedChanges

$resolvedCommit = @(
    Invoke-RepositoryGit -GitArguments @(
        "rev-parse"
        "--verify"
        "--end-of-options"
        "$Commit^{commit}"
    )
) | Where-Object { $_ -match "^[0-9a-fA-F]{40}$" } | Select-Object -Last 1
if (-not $resolvedCommit) {
    throw "Could not resolve a 40-character commit ID for '$Commit'."
}
$resolvedCommit = $resolvedCommit.ToLowerInvariant()
$shortCommit = $resolvedCommit.Substring(0, 12)

$originMain = @(
    Invoke-RepositoryGit -GitArguments @(
        "rev-parse"
        "--verify"
        "origin/main^{commit}"
    )
) | Where-Object { $_ -match "^[0-9a-fA-F]{40}$" } | Select-Object -Last 1
if ($originMain.ToLowerInvariant() -cne $resolvedCommit) {
    throw "The release commit must match the locally known origin/main. Push and recheck the private repository first."
}

$packageMetadata = (
    Get-CommittedFileContent -ResolvedCommit $resolvedCommit -Path "package.json"
) | ConvertFrom-Json
$releaseVersion = [string]$packageMetadata.version
if ($releaseVersion -notmatch '^\d+\.\d+\.\d+(?:-(?:alpha|rc)\.\d+)?$') {
    throw "The committed package.json version is not a supported release version: $releaseVersion"
}
$installerMetadata = (
    Get-CommittedFileContent `
        -ResolvedCommit $resolvedCommit `
        -Path "apps/desktop/src-tauri/tauri.installer.conf.json"
) | ConvertFrom-Json
$installerProductName = [string]$installerMetadata.productName
if ([string]::IsNullOrWhiteSpace($installerProductName) -or $installerProductName.EndsWith(".")) {
    throw "The committed Windows installer productName must be present and must not end with a period."
}
$releaseTag = "v$releaseVersion"
$isStableRelease = $releaseVersion -match '^\d+\.\d+\.\d+$'
if ($isStableRelease -and $SkipInstaller) {
    throw "A stable release must include a signed Windows installer. Remove -SkipInstaller."
}
if ($isStableRelease -and [string]::IsNullOrWhiteSpace($ExpectedSignerSubject)) {
    throw "A stable release requires -ExpectedSignerSubject for Authenticode verification."
}
if ($isStableRelease -and (
    [string]::IsNullOrWhiteSpace($SigningCertificateThumbprint) -or
    [string]::IsNullOrWhiteSpace($TimestampUrl)
)) {
    throw "A stable release requires -SigningCertificateThumbprint and -TimestampUrl."
}

$changelog = Get-CommittedFileContent -ResolvedCommit $resolvedCommit -Path "CHANGELOG.md"
$escapedTag = [regex]::Escape($releaseTag)
$releaseNotesMatch = [regex]::Match(
    $changelog,
    "(?ms)^## $escapedTag(?:\s+[^\r\n]*)?\r?\n.*?(?=^## v|\z)"
)
if (-not $releaseNotesMatch.Success) {
    throw "CHANGELOG.md does not contain a release section for $releaseTag."
}
$releaseNotes = $releaseNotesMatch.Value.Trim()

if ([string]::IsNullOrWhiteSpace($Destination)) {
    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $preparationRoot = Join-Path `
        $repositoryRoot `
        ".tools\public-release-prep\$releaseVersion\$shortCommit-$timestamp"
}
elseif ([System.IO.Path]::IsPathRooted($Destination)) {
    $preparationRoot = [System.IO.Path]::GetFullPath($Destination)
}
else {
    $preparationRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot $Destination))
}
if (Test-Path -LiteralPath $preparationRoot) {
    throw "Destination already exists; refusing to overwrite it: $preparationRoot"
}

if (-not $SkipChecks) {
    Push-Location -LiteralPath $repositoryRoot
    try {
        Invoke-CheckedCommand -Command "npm.cmd" -Arguments @("run", "typecheck")
        Invoke-CheckedCommand -Command "npm.cmd" -Arguments @("run", "build")

        $cargoExecutable = Resolve-CargoExecutable
        Invoke-CheckedCommand -Command $cargoExecutable -Arguments @("fmt", "--all", "--", "--check")
        Invoke-CheckedCommand -Command $cargoExecutable -Arguments @("test", "--workspace", "--locked")
    }
    finally {
        Pop-Location
    }
}

$installerSourcePath = $null
$installerHash = $null
$installerSignatureStatus = $null
$installerSignerSubject = $null
$installerSignerThumbprint = $null
$installerTimestampSubject = $null
if (-not $SkipInstaller) {
    $buildArguments = @{ Installer = $true }
    if (-not [string]::IsNullOrWhiteSpace($SigningCertificateThumbprint)) {
        $buildArguments.SigningCertificateThumbprint = $SigningCertificateThumbprint
    }
    if (-not [string]::IsNullOrWhiteSpace($TimestampUrl)) {
        $buildArguments.TimestampUrl = $TimestampUrl
    }
    & $buildScript @buildArguments
    if ($LASTEXITCODE -ne 0) {
        throw "The Windows installer build failed with exit code $LASTEXITCODE."
    }

    $installerSourcePath = Join-Path `
        $repositoryRoot `
        "target\x86_64-pc-windows-msvc\release\bundle\nsis\$($installerProductName)_$($releaseVersion)_x64-setup.exe"
    if (-not (Test-Path -LiteralPath $installerSourcePath -PathType Leaf)) {
        throw "The expected installer was not created: $installerSourcePath"
    }
    $installerHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $installerSourcePath).Hash.ToLowerInvariant()
    $installerSignature = Get-AuthenticodeSignature -LiteralPath $installerSourcePath
    $installerSignatureStatus = [string]$installerSignature.Status

    if ($isStableRelease) {
        $desktopExecutablePath = Join-Path `
            $repositoryRoot `
            "target\x86_64-pc-windows-msvc\release\mio-desktop.exe"
        $signatureArguments = @{
            Path = @($desktopExecutablePath, $installerSourcePath)
            ExpectedSignerSubject = $ExpectedSignerSubject
        }
        if (-not [string]::IsNullOrWhiteSpace($SignToolPath)) {
            $signatureArguments.SignToolPath = $SignToolPath
        }
        & $signatureTestScript @signatureArguments | Out-Host
    }

    if ($null -ne $installerSignature.SignerCertificate) {
        $installerSignerSubject = $installerSignature.SignerCertificate.Subject
        $installerSignerThumbprint = $installerSignature.SignerCertificate.Thumbprint
    }
    if ($null -ne $installerSignature.TimeStamperCertificate) {
        $installerTimestampSubject = $installerSignature.TimeStamperCertificate.Subject
    }
}

New-Item -ItemType Directory -Path $preparationRoot | Out-Null
$snapshotRoot = Join-Path $preparationRoot "snapshot"
& $exportScript -Commit $resolvedCommit -Destination $snapshotRoot
if ($LASTEXITCODE -ne 0) {
    throw "The public source export failed with exit code $LASTEXITCODE."
}

if ([string]::IsNullOrWhiteSpace($GitleaksPath)) {
    $bundledGitleaks = Join-Path $repositoryRoot ".tools\gitleaks-8.30.1\gitleaks.exe"
    if (Test-Path -LiteralPath $bundledGitleaks -PathType Leaf) {
        $GitleaksPath = $bundledGitleaks
    }
    else {
        $gitleaksCommand = Get-Command gitleaks.exe -ErrorAction SilentlyContinue
        if ($null -ne $gitleaksCommand) {
            $GitleaksPath = $gitleaksCommand.Source
        }
    }
}
if ([string]::IsNullOrWhiteSpace($GitleaksPath) -or -not (Test-Path -LiteralPath $GitleaksPath -PathType Leaf)) {
    throw "Gitleaks was not found. Supply its verified executable with -GitleaksPath."
}

$gitleaksReportPath = Join-Path $preparationRoot "gitleaks-report.json"
Invoke-CheckedCommand -Command $GitleaksPath -Arguments @(
    "dir"
    "--no-banner"
    "--no-color"
    "--redact"
    "--report-format"
    "json"
    "--report-path"
    $gitleaksReportPath
    (Join-Path $snapshotRoot "source")
)

$artifactsRoot = Join-Path $preparationRoot "artifacts"
New-Item -ItemType Directory -Path $artifactsRoot | Out-Null

$snapshotManifestPath = Join-Path $snapshotRoot "snapshot-manifest.json"
$snapshotManifest = Get-Content -LiteralPath $snapshotManifestPath -Raw | ConvertFrom-Json
$sourceArchivePath = Join-Path $snapshotRoot ([string]$snapshotManifest.archiveFile)
$preparedSourceArchivePath = Join-Path $artifactsRoot ([System.IO.Path]::GetFileName($sourceArchivePath))
Copy-Item -LiteralPath $sourceArchivePath -Destination $preparedSourceArchivePath

$preparedInstallerPath = $null
if ($null -ne $installerSourcePath) {
    $preparedInstallerPath = Join-Path $artifactsRoot ([System.IO.Path]::GetFileName($installerSourcePath))
    Copy-Item -LiteralPath $installerSourcePath -Destination $preparedInstallerPath
}

$releaseNotesPath = Join-Path $preparationRoot "release-notes.md"
$releaseNotes | Set-Content -LiteralPath $releaseNotesPath -Encoding UTF8

$checksumsPath = Join-Path $artifactsRoot "SHA256SUMS.txt"
$checksumLines = @(
    "$($snapshotManifest.archiveSha256)  $([System.IO.Path]::GetFileName($preparedSourceArchivePath))"
)
if ($null -ne $preparedInstallerPath) {
    $checksumLines += "$installerHash  $([System.IO.Path]::GetFileName($preparedInstallerPath))"
}
$checksumLines | Set-Content -LiteralPath $checksumsPath -Encoding ASCII

$preparedSnapshotManifestPath = Join-Path $artifactsRoot "snapshot-manifest.json"
Copy-Item -LiteralPath $snapshotManifestPath -Destination $preparedSnapshotManifestPath

$releasePlan = [ordered]@{
    schemaVersion = 1
    product = "M.I.O."
    version = $releaseVersion
    tag = $releaseTag
    sourceCommit = $resolvedCommit
    preparedAt = (Get-Date).ToUniversalTime().ToString("o")
    privateRepository = "blackcometclub/M.I.O-dev"
    publicRepository = "blackcometclub/M.I.O"
    preparationRoot = $preparationRoot
    sourceRoot = (Join-Path $snapshotRoot "source")
    sourceFileList = (Join-Path $snapshotRoot "source-files.txt")
    sourceArchive = $preparedSourceArchivePath
    sourceArchiveSha256 = [string]$snapshotManifest.archiveSha256
    sourceFileCount = [int]$snapshotManifest.sourceFileCount
    installer = if ($null -eq $preparedInstallerPath) {
        $null
    }
    else {
        [ordered]@{
            path = $preparedInstallerPath
            sha256 = $installerHash
            signatureStatus = $installerSignatureStatus
            signerSubject = $installerSignerSubject
            signerThumbprint = $installerSignerThumbprint
            timestampSubject = $installerTimestampSubject
        }
    }
    snapshotManifest = $preparedSnapshotManifestPath
    checksums = $checksumsPath
    releaseNotes = $releaseNotesPath
    gitleaksReport = $gitleaksReportPath
    checksRun = (-not $SkipChecks)
    installerBuilt = (-not $SkipInstaller)
}

$releasePlanPath = Join-Path $preparationRoot "release-plan.json"
$releasePlan | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $releasePlanPath -Encoding UTF8

$summaryPath = Join-Path $preparationRoot "REVIEW.txt"
$summaryLines = @(
    "M.I.O. public release preparation"
    "Version: $releaseVersion"
    "Tag: $releaseTag"
    "Private source commit: $resolvedCommit"
    "Source files: $($snapshotManifest.sourceFileCount)"
    "Source ZIP SHA-256: $($snapshotManifest.archiveSha256)"
    "Validation checks run: $(-not $SkipChecks)"
    "Installer built: $(-not $SkipInstaller)"
)
if ($null -ne $preparedInstallerPath) {
    $summaryLines += "Installer signature: $installerSignatureStatus"
    $summaryLines += "Installer SHA-256: $installerHash"
}
$summaryLines += "Gitleaks: PASS (0 findings)"
$summaryLines += ""
$summaryLines += "No GitHub repository, tag, or Release was changed."
$summaryLines += "Preview the next step with:"
$summaryLines += "& .\scripts\publish-public-release.ps1 -Plan `"$releasePlanPath`" -PublicRepositoryPath <public-clone-path>"
$summaryLines | Set-Content -LiteralPath $summaryPath -Encoding UTF8

Write-Host "M.I.O. public release preparation completed."
Write-Host "Version: $releaseVersion"
Write-Host "Source commit: $resolvedCommit"
Write-Host "Review: $summaryPath"
Write-Host "Plan: $releasePlanPath"
Write-Host "No external publication was performed."
