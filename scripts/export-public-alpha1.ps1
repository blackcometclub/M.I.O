[CmdletBinding()]
param(
    [string]$Commit = "HEAD",
    [string]$Destination
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$repositoryRootForGit = $repositoryRoot.Replace("\", "/")
$excludedPaths = @(
    "MOE_Implementation_Plan.md"
    "docs/FINAL-READINESS-2026-08-12.md"
    "docs/HANDOFF-2026-08-12.md"
    "docs/HANDOFF_2026-08-12.md"
    "docs/HANDOFF_2026-08-12_RELAY_CLIENT.md"
    "docs/HANDOFF_2026-08-18_PUBLIC_ALPHA_NO_CLI.md"
)

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

function Assert-NoTrackedChanges {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$DiffArguments,
        [Parameter(Mandatory = $true)]
        [string]$FailureMessage
    )

    & git.exe `
        -c "safe.directory=$repositoryRootForGit" `
        -c core.excludesFile=NUL `
        -C $repositoryRoot `
        @DiffArguments `
        --quiet `
        --
    $diffExitCode = $LASTEXITCODE
    if ($diffExitCode -eq 1) {
        throw $FailureMessage
    }
    if ($diffExitCode -ne 0) {
        throw "git $($DiffArguments -join ' ') failed with exit code $diffExitCode."
    }
}

Assert-NoTrackedChanges `
    -DiffArguments @("diff") `
    -FailureMessage "Tracked working-tree changes exist. Commit or discard them before exporting a public snapshot."
Assert-NoTrackedChanges `
    -DiffArguments @("diff", "--cached") `
    -FailureMessage "Staged changes exist. Commit or unstage them before exporting a public snapshot."

$commitOutput = @(
    Invoke-RepositoryGit -GitArguments @(
        "rev-parse"
        "--verify"
        "--end-of-options"
        "$Commit^{commit}"
    )
)
$resolvedCommit = $commitOutput |
    Where-Object { $_ -match "^[0-9a-fA-F]{40}$" } |
    Select-Object -Last 1
if (-not $resolvedCommit) {
    throw "Could not resolve a 40-character commit ID for '$Commit'."
}
$resolvedCommit = $resolvedCommit.ToLowerInvariant()
$shortCommit = $resolvedCommit.Substring(0, 12)

if ([string]::IsNullOrWhiteSpace($Destination)) {
    $destinationRoot = Join-Path $repositoryRoot ".tools\public-alpha1\$shortCommit"
}
elseif ([System.IO.Path]::IsPathRooted($Destination)) {
    $destinationRoot = [System.IO.Path]::GetFullPath($Destination)
}
else {
    $destinationRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot $Destination))
}

if (Test-Path -LiteralPath $destinationRoot) {
    throw "Destination already exists; refusing to overwrite it: $destinationRoot"
}

$destinationParent = Split-Path -Parent $destinationRoot
New-Item -ItemType Directory -Path $destinationParent -Force | Out-Null
New-Item -ItemType Directory -Path $destinationRoot | Out-Null

$archivePath = Join-Path $destinationRoot "mio-v0.1.0-alpha.1-source.zip"
$sourceRoot = Join-Path $destinationRoot "source"
$manifestPath = Join-Path $destinationRoot "snapshot-manifest.json"
$fileListPath = Join-Path $destinationRoot "source-files.txt"

$archiveArguments = @(
    "archive"
    "--format=zip"
    "--output=$archivePath"
    $resolvedCommit
    "--"
    "."
)
foreach ($excludedPath in $excludedPaths) {
    $archiveArguments += ":(exclude)$excludedPath"
}
Invoke-RepositoryGit -GitArguments $archiveArguments | Out-Null

New-Item -ItemType Directory -Path $sourceRoot | Out-Null
Expand-Archive -LiteralPath $archivePath -DestinationPath $sourceRoot

$excludedSet = New-Object "System.Collections.Generic.HashSet[string]" ([System.StringComparer]::Ordinal)
foreach ($excludedPath in $excludedPaths) {
    [void]$excludedSet.Add($excludedPath)
}

$expectedFiles = @(
    Invoke-RepositoryGit -GitArguments @(
        "ls-tree"
        "-r"
        "--name-only"
        $resolvedCommit
        "--"
    ) |
        Where-Object { -not $excludedSet.Contains($_) } |
        Sort-Object
)
$actualFiles = @(
    Get-ChildItem -LiteralPath $sourceRoot -Recurse -File |
        ForEach-Object {
            $_.FullName.Substring($sourceRoot.Length + 1).Replace("\", "/")
        } |
        Sort-Object
)

$fileListDifference = @(Compare-Object -ReferenceObject $expectedFiles -DifferenceObject $actualFiles)
if ($fileListDifference.Count -ne 0) {
    throw "The exported file list does not match the committed public snapshot boundary."
}

foreach ($excludedPath in $excludedPaths) {
    if ($actualFiles -contains $excludedPath) {
        throw "An excluded private-development document was exported: $excludedPath"
    }
}

$actualFiles | Set-Content -LiteralPath $fileListPath -Encoding UTF8
$archiveHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $archivePath).Hash.ToLowerInvariant()
$manifest = [ordered]@{
    product = "M.I.O."
    version = "0.1.0-alpha.1"
    sourceCommit = $resolvedCommit
    sourceFileCount = $actualFiles.Count
    archiveFile = [System.IO.Path]::GetFileName($archivePath)
    archiveSha256 = $archiveHash
    historyIncluded = $false
    untrackedFilesIncluded = $false
    excludedPrivateDevelopmentDocuments = $excludedPaths
}
$manifest |
    ConvertTo-Json -Depth 4 |
    Set-Content -LiteralPath $manifestPath -Encoding UTF8

Write-Host "M.I.O. public alpha source snapshot created."
Write-Host "Source commit: $resolvedCommit"
Write-Host "Source files: $($actualFiles.Count)"
Write-Host "Archive: $archivePath"
Write-Host "SHA-256: $archiveHash"
Write-Host "Review directory: $sourceRoot"
