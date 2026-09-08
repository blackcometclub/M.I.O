[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string[]]$Path,
    [Parameter(Mandatory = $true)]
    [string]$ExpectedSignerSubject,
    [string]$SignToolPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Resolve-SignTool {
    if (-not [string]::IsNullOrWhiteSpace($SignToolPath)) {
        return (Resolve-Path -LiteralPath $SignToolPath).Path
    }

    $signToolCommand = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if ($null -ne $signToolCommand) {
        return $signToolCommand.Source
    }

    $kitsRoot = Join-Path ${env:ProgramFiles(x86)} "Windows Kits\10\bin"
    $candidates = @(
        Get-ChildItem -LiteralPath $kitsRoot -Filter signtool.exe -Recurse -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
            Sort-Object FullName -Descending
    )
    if ($candidates.Count -eq 0) {
        throw "signtool.exe was not found. Install the Windows SDK or supply -SignToolPath."
    }

    return $candidates[0].FullName
}

if ([string]::IsNullOrWhiteSpace($ExpectedSignerSubject)) {
    throw "-ExpectedSignerSubject must contain the exact certificate subject recorded for the release."
}

$resolvedSignTool = Resolve-SignTool
$results = foreach ($candidatePath in $Path) {
    $resolvedPath = (Resolve-Path -LiteralPath $candidatePath).Path
    $signature = Get-AuthenticodeSignature -LiteralPath $resolvedPath

    if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid) {
        throw "Authenticode signature is not valid for $resolvedPath. Status: $($signature.Status)."
    }
    if ($null -eq $signature.SignerCertificate) {
        throw "The signer certificate is missing for $resolvedPath."
    }
    if ($signature.SignerCertificate.Subject -cne $ExpectedSignerSubject) {
        throw "Unexpected signer for $resolvedPath. Expected '$ExpectedSignerSubject', got '$($signature.SignerCertificate.Subject)'."
    }
    if ($null -eq $signature.TimeStamperCertificate) {
        throw "The RFC 3161 timestamp is missing for $resolvedPath."
    }

    $verificationOutput = @(& $resolvedSignTool verify /pa /all /v $resolvedPath 2>&1)
    $verificationExitCode = $LASTEXITCODE
    if ($verificationExitCode -ne 0) {
        throw "SignTool verification failed for $resolvedPath with exit code $verificationExitCode.`n$($verificationOutput -join [Environment]::NewLine)"
    }
    $verificationOutput | ForEach-Object { Write-Verbose $_.ToString() }

    [pscustomobject]@{
        Path = $resolvedPath
        Status = [string]$signature.Status
        SignerSubject = $signature.SignerCertificate.Subject
        SignerThumbprint = $signature.SignerCertificate.Thumbprint
        TimestampSubject = $signature.TimeStamperCertificate.Subject
        TimestampThumbprint = $signature.TimeStamperCertificate.Thumbprint
    }
}

$results
