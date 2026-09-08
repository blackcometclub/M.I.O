[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Plan,
    [Parameter(Mandatory = $true)]
    [string]$PublicRepositoryPath,
    [switch]$CreateDraft,
    [switch]$PublishDraft,
    [string]$ConfirmSourceCommit,
    [string]$ConfirmTag
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$signatureTestScript = Join-Path $PSScriptRoot "test-windows-release-signature.ps1"

if ($CreateDraft -and $PublishDraft) {
    throw "Choose either -CreateDraft or -PublishDraft, not both."
}

$releasePlanPath = (Resolve-Path -LiteralPath $Plan).Path
$releasePlan = Get-Content -LiteralPath $releasePlanPath -Raw | ConvertFrom-Json
if ([int]$releasePlan.schemaVersion -ne 1) {
    throw "Unsupported release plan schema: $($releasePlan.schemaVersion)"
}
if ([string]$releasePlan.publicRepository -cne "blackcometclub/M.I.O") {
    throw "The release plan does not target blackcometclub/M.I.O."
}

$publicRoot = (Resolve-Path -LiteralPath $PublicRepositoryPath).Path
$publicRootForGit = $publicRoot.Replace("\", "/")
$gitDirectory = Join-Path $publicRoot ".git"
if (-not (Test-Path -LiteralPath $gitDirectory)) {
    throw "The public repository path is not a Git working tree: $publicRoot"
}

function Invoke-PublicGit {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$GitArguments,
        [switch]$AllowFailure
    )

    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = @(
            & git.exe `
                -c "safe.directory=$publicRootForGit" `
                -c core.autocrlf=false `
                -c core.quotepath=false `
                -C $publicRoot `
                @GitArguments 2>&1
        )
        $gitExitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }

    if (-not $AllowFailure -and $gitExitCode -ne 0) {
        throw "git $($GitArguments -join ' ') failed:`n$($output -join [Environment]::NewLine)"
    }

    return [pscustomobject]@{
        ExitCode = $gitExitCode
        Output = @($output | ForEach-Object { $_.ToString() })
    }
}

function Invoke-Gh {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments
    )

    & gh.exe @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "gh $($Arguments -join ' ') failed with exit code $LASTEXITCODE."
    }
}

function Assert-FileHash {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedHash
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Release artifact was not found: $Path"
    }
    $actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
    if ($actualHash -cne $ExpectedHash.ToLowerInvariant()) {
        throw "SHA-256 mismatch for $Path. Expected $ExpectedHash, got $actualHash."
    }
}

function Get-GitBlobHash {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $content = [System.IO.File]::ReadAllBytes($Path)
    $crlfExtensions = @(".bat", ".cmd", ".ps1")
    $requiresLineEndingNormalization = `
        $crlfExtensions -contains [System.IO.Path]::GetExtension($Path).ToLowerInvariant()
    if ($requiresLineEndingNormalization) {
        $normalized = New-Object "System.Collections.Generic.List[byte]" $content.Length
        for ($index = 0; $index -lt $content.Length; $index++) {
            if ($content[$index] -eq 13) {
                if ($index + 1 -lt $content.Length -and $content[$index + 1] -eq 10) {
                    $index++
                }
                $normalized.Add(10)
                continue
            }
            $normalized.Add($content[$index])
        }
        $content = $normalized.ToArray()
    }
    $header = [System.Text.Encoding]::ASCII.GetBytes("blob $($content.Length)`0")
    $payload = New-Object byte[] ($header.Length + $content.Length)
    [System.Buffer]::BlockCopy($header, 0, $payload, 0, $header.Length)
    [System.Buffer]::BlockCopy($content, 0, $payload, $header.Length, $content.Length)

    $sha1 = [System.Security.Cryptography.SHA1]::Create()
    try {
        $hash = $sha1.ComputeHash($payload)
    }
    finally {
        $sha1.Dispose()
    }

    return ([System.BitConverter]::ToString($hash)).Replace("-", "").ToLowerInvariant()
}

function Get-PathComparison {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$ExpectedFiles,
        [Parameter(Mandatory = $true)]
        [string[]]$TrackedFiles,
        [Parameter(Mandatory = $true)]
        [hashtable]$TrackedBlobByPath,
        [Parameter(Mandatory = $true)]
        [string]$SourceRoot
    )

    $expectedSet = New-Object "System.Collections.Generic.HashSet[string]" ([System.StringComparer]::Ordinal)
    foreach ($path in $ExpectedFiles) {
        [void]$expectedSet.Add($path)
    }
    $trackedSet = New-Object "System.Collections.Generic.HashSet[string]" ([System.StringComparer]::Ordinal)
    foreach ($path in $TrackedFiles) {
        [void]$trackedSet.Add($path)
    }

    $added = @($ExpectedFiles | Where-Object { -not $trackedSet.Contains($_) })
    $deleted = @($TrackedFiles | Where-Object { -not $expectedSet.Contains($_) })
    $modified = @()
    foreach ($path in $ExpectedFiles) {
        if (-not $trackedSet.Contains($path)) {
            continue
        }

        $sourcePath = Join-Path $SourceRoot $path
        if (-not $TrackedBlobByPath.ContainsKey($path)) {
            $modified += $path
            continue
        }

        $sourceBlobHash = Get-GitBlobHash -Path $sourcePath
        if ($sourceBlobHash -cne $TrackedBlobByPath[$path]) {
            $modified += $path
        }
    }

    return [pscustomobject]@{
        Added = @($added)
        Modified = @($modified)
        Deleted = @($deleted)
    }
}

function Invoke-GitPathChunks {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Prefix,
        [Parameter(Mandatory = $true)]
        [string[]]$Paths
    )

    $chunkSize = 40
    for ($offset = 0; $offset -lt $Paths.Count; $offset += $chunkSize) {
        $lastIndex = [Math]::Min($offset + $chunkSize - 1, $Paths.Count - 1)
        $chunk = @($Paths[$offset..$lastIndex])
        $result = Invoke-PublicGit -GitArguments (@($Prefix) + $chunk)
        if ($result.Output.Count -gt 0) {
            $result.Output | Write-Host
        }
    }
}

$sourceRoot = (Resolve-Path -LiteralPath ([string]$releasePlan.sourceRoot)).Path
$sourceFileListPath = (Resolve-Path -LiteralPath ([string]$releasePlan.sourceFileList)).Path
$releaseNotesPath = (Resolve-Path -LiteralPath ([string]$releasePlan.releaseNotes)).Path
$snapshotManifestPath = (Resolve-Path -LiteralPath ([string]$releasePlan.snapshotManifest)).Path
$checksumsPath = (Resolve-Path -LiteralPath ([string]$releasePlan.checksums)).Path
$sourceArchivePath = (Resolve-Path -LiteralPath ([string]$releasePlan.sourceArchive)).Path

Assert-FileHash -Path $sourceArchivePath -ExpectedHash ([string]$releasePlan.sourceArchiveSha256)
$installerPath = $null
if ($null -ne $releasePlan.installer) {
    $installerPath = (Resolve-Path -LiteralPath ([string]$releasePlan.installer.path)).Path
    Assert-FileHash -Path $installerPath -ExpectedHash ([string]$releasePlan.installer.sha256)
}

$isStableRelease = [string]$releasePlan.version -match '^\d+\.\d+\.\d+$'
if ($isStableRelease) {
    if ($null -eq $releasePlan.installer) {
        throw "A stable release plan must include a signed Windows installer."
    }
    if ([string]$releasePlan.installer.signatureStatus -cne "Valid") {
        throw "A stable release plan requires a Valid Authenticode signature."
    }
    if ([string]::IsNullOrWhiteSpace([string]$releasePlan.installer.signerSubject)) {
        throw "A stable release plan is missing its verified signer subject."
    }
    if ([string]::IsNullOrWhiteSpace([string]$releasePlan.installer.timestampSubject)) {
        throw "A stable release plan is missing its verified timestamp subject."
    }

    & $signatureTestScript `
        -Path @($installerPath) `
        -ExpectedSignerSubject ([string]$releasePlan.installer.signerSubject) |
        Out-Host
}

$remoteResult = Invoke-PublicGit -GitArguments @("remote", "get-url", "origin")
$originUrl = $remoteResult.Output | Select-Object -Last 1
if ($originUrl -notmatch '^https://github\.com/blackcometclub/M\.I\.O(?:\.git)?$') {
    throw "Unexpected public repository origin: $originUrl"
}

$branchResult = Invoke-PublicGit -GitArguments @("branch", "--show-current")
$branch = $branchResult.Output | Select-Object -Last 1
if ($branch -cne "main") {
    throw "The public repository must be on main, not '$branch'."
}

$statusResult = Invoke-PublicGit -GitArguments @("status", "--porcelain=v1", "--untracked-files=all")
if ($statusResult.Output.Count -ne 0) {
    throw "The public repository working tree is not clean. Nothing was changed.`n$($statusResult.Output -join [Environment]::NewLine)"
}

$expectedFiles = @(
    Get-Content -LiteralPath $sourceFileListPath |
        ForEach-Object { $_.Trim() } |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
)
if ($expectedFiles.Count -ne [int]$releasePlan.sourceFileCount) {
    throw "The source file list count does not match the release plan."
}
foreach ($path in $expectedFiles) {
    if ([System.IO.Path]::IsPathRooted($path) -or $path -match '(^|/)\.\.(/|$)' -or $path -match '(^|/)\.git(/|$)') {
        throw "Unsafe source path in release plan: $path"
    }
    if (-not (Test-Path -LiteralPath (Join-Path $sourceRoot $path) -PathType Leaf)) {
        throw "A planned source file is missing: $path"
    }
}

$trackedResult = Invoke-PublicGit -GitArguments @("ls-files")
$trackedFiles = @($trackedResult.Output | Sort-Object)
$trackedBlobByPath = @{}
$trackedStageResult = Invoke-PublicGit -GitArguments @("ls-files", "--stage")
foreach ($line in $trackedStageResult.Output) {
    if ($line -notmatch '^[0-7]{6} ([0-9a-f]{40,64}) [0-3]\t(.+)$') {
        throw "Could not parse the public repository index entry: $line"
    }
    if ($Matches[2] -match '(^|/)\.git(/|$)') {
        throw "The public repository index contains an unsafe path: $($Matches[2])"
    }
    $trackedBlobByPath[$Matches[2]] = $Matches[1].ToLowerInvariant()
}
$comparison = Get-PathComparison `
    -ExpectedFiles @($expectedFiles | Sort-Object) `
    -TrackedFiles $trackedFiles `
    -TrackedBlobByPath $trackedBlobByPath `
    -SourceRoot $sourceRoot

Write-Host "M.I.O. public release preview"
Write-Host "Version: $($releasePlan.version)"
Write-Host "Tag: $($releasePlan.tag)"
Write-Host "Private source commit: $($releasePlan.sourceCommit)"
Write-Host "Public repository HEAD: $((Invoke-PublicGit -GitArguments @('rev-parse', 'HEAD')).Output[-1])"
Write-Host "Files to add: $($comparison.Added.Count)"
Write-Host "Files to modify: $($comparison.Modified.Count)"
Write-Host "Files to delete: $($comparison.Deleted.Count)"
foreach ($path in $comparison.Added) {
    Write-Host "  ADD $path"
}
foreach ($path in $comparison.Modified) {
    Write-Host "  MODIFY $path"
}
foreach ($path in $comparison.Deleted) {
    Write-Host "  DELETE $path"
}
Write-Host "Source asset: $sourceArchivePath"
if ($null -ne $installerPath) {
    Write-Host "Installer asset: $installerPath"
    Write-Host "Installer signature: $($releasePlan.installer.signatureStatus)"
}

if (-not $CreateDraft -and -not $PublishDraft) {
    Write-Host "No file, GitHub repository, tag, or Release was changed."
    Write-Host "After Owner approval, create a draft with:"
    Write-Host "& .\scripts\publish-public-release.ps1 -Plan `"$releasePlanPath`" -PublicRepositoryPath `"$publicRoot`" -CreateDraft -ConfirmSourceCommit $($releasePlan.sourceCommit)"
    exit 0
}

if ($PublishDraft) {
    if ($ConfirmTag -cne [string]$releasePlan.tag) {
        throw "-PublishDraft requires -ConfirmTag $($releasePlan.tag)."
    }

    Invoke-Gh -Arguments @(
        "release"
        "edit"
        ([string]$releasePlan.tag)
        "--repo"
        ([string]$releasePlan.publicRepository)
        "--draft=false"
        "--prerelease"
    )
    Write-Host "Published GitHub Prerelease $($releasePlan.tag)."
    exit 0
}

if ($ConfirmSourceCommit -cne [string]$releasePlan.sourceCommit) {
    throw "-CreateDraft requires -ConfirmSourceCommit $($releasePlan.sourceCommit)."
}

$fetchResult = Invoke-PublicGit -GitArguments @("fetch", "origin", "main", "--tags")
if ($fetchResult.Output.Count -gt 0) {
    $fetchResult.Output | Write-Host
}
$headCommit = (Invoke-PublicGit -GitArguments @("rev-parse", "HEAD")).Output[-1]
$originMainCommit = (Invoke-PublicGit -GitArguments @("rev-parse", "origin/main")).Output[-1]
if ($headCommit -cne $originMainCommit) {
    throw "The public clone HEAD does not match origin/main after fetch. Nothing was changed."
}

$existingTag = Invoke-PublicGit -GitArguments @(
    "show-ref"
    "--verify"
    "--quiet"
    "refs/tags/$($releasePlan.tag)"
) -AllowFailure
if ($existingTag.ExitCode -eq 0) {
    throw "Tag $($releasePlan.tag) already exists in the public clone."
}
if ($existingTag.ExitCode -ne 1) {
    throw "Could not verify whether tag $($releasePlan.tag) already exists."
}

if ($comparison.Deleted.Count -gt 0) {
    Invoke-GitPathChunks -Prefix @("rm", "--ignore-unmatch", "--") -Paths $comparison.Deleted
}

$filesToCopy = @($comparison.Added) + @($comparison.Modified)
foreach ($path in $filesToCopy) {
    $sourcePath = Join-Path $sourceRoot $path
    $destinationPath = Join-Path $publicRoot $path
    $destinationParent = Split-Path -Parent $destinationPath
    if (-not (Test-Path -LiteralPath $destinationParent -PathType Container)) {
        New-Item -ItemType Directory -Path $destinationParent -Force | Out-Null
    }
    Copy-Item -LiteralPath $sourcePath -Destination $destinationPath -Force
}
if ($filesToCopy.Count -gt 0) {
    Invoke-GitPathChunks -Prefix @("add", "--force", "--") -Paths $filesToCopy
}

$indexedFiles = @((Invoke-PublicGit -GitArguments @("ls-files")).Output | Sort-Object)
$fileListDifference = @(Compare-Object -ReferenceObject @($expectedFiles | Sort-Object) -DifferenceObject $indexedFiles)
if ($fileListDifference.Count -ne 0) {
    throw "The staged public repository file list does not match the prepared snapshot. Changes were left for inspection."
}

$diffCheck = Invoke-PublicGit -GitArguments @("diff", "--cached", "--check")
if ($diffCheck.Output.Count -gt 0) {
    $diffCheck.Output | Write-Host
}
$stagedNames = @((Invoke-PublicGit -GitArguments @("diff", "--cached", "--name-only")).Output)
if ($stagedNames.Count -eq 0) {
    throw "The prepared snapshot creates no public repository change. Nothing was committed."
}

$commitMessage = "Publish M.I.O. $($releasePlan.tag)"
$commitResult = Invoke-PublicGit -GitArguments @("commit", "-m", $commitMessage)
$commitResult.Output | Write-Host
$publicReleaseCommit = (Invoke-PublicGit -GitArguments @("rev-parse", "HEAD")).Output[-1]

$tagResult = Invoke-PublicGit -GitArguments @(
    "tag"
    "-a"
    ([string]$releasePlan.tag)
    "-m"
    "M.I.O. $($releasePlan.tag)"
    $publicReleaseCommit
)
if ($tagResult.Output.Count -gt 0) {
    $tagResult.Output | Write-Host
}

$pushResult = Invoke-PublicGit -GitArguments @(
    "push"
    "--atomic"
    "origin"
    "main"
    "refs/tags/$($releasePlan.tag)"
)
$pushResult.Output | Write-Host

$releaseAssets = @(
    $sourceArchivePath
    $snapshotManifestPath
    $checksumsPath
)
if ($null -ne $installerPath) {
    $releaseAssets += $installerPath
}

Invoke-Gh -Arguments (@(
    "release"
    "create"
    ([string]$releasePlan.tag)
    "--repo"
    ([string]$releasePlan.publicRepository)
    "--verify-tag"
    "--draft"
    "--prerelease"
    "--title"
    "M.I.O. $($releasePlan.tag)"
    "--notes-file"
    $releaseNotesPath
) + $releaseAssets)

Write-Host "Draft GitHub Prerelease created for $($releasePlan.tag)."
Write-Host "Review the draft, then publish it only after explicit Owner approval:"
Write-Host "& .\scripts\publish-public-release.ps1 -Plan `"$releasePlanPath`" -PublicRepositoryPath `"$publicRoot`" -PublishDraft -ConfirmTag $($releasePlan.tag)"
