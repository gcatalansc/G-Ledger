#requires -Version 7.0
[CmdletBinding()]
param([Parameter(Mandatory)][string]$Binary)
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$source = Get-Content -LiteralPath (Join-Path $repoRoot 'release/distribution.json') -Raw
$tempBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$fixtureRoot = Join-Path $tempBase ('ledger-distribution-' + [guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory -Path (Join-Path $fixtureRoot 'release')
try {
    foreach ($case in @('version', 'acceptance')) {
        $manifest = $source | ConvertFrom-Json
        if ($case -eq 'version') {
            $manifest.product_version = '0.3.0-rc.999'
            $manifest.release_notes = 'docs/RELEASE_NOTES_v0.3.0-rc.999.md'
            $expected = 'Distribution binary version mismatch'
        }
        else { $manifest.operational_acceptance = 'OPERATIONALLY_ACCEPTED'; $expected = 'Invalid distribution classification' }
        $manifest | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $fixtureRoot 'release/distribution.json') -Encoding utf8NoBOM
        $output = Join-Path $fixtureRoot ($case + '.zip')
        $errorMessage = $null
        try { & (Join-Path $PSScriptRoot 'package-windows.ps1') -Binary $Binary -SourceRoot $fixtureRoot -Output $output | Out-Null }
        catch { $errorMessage = $_.Exception.Message }
        if ($errorMessage -ne $expected) { throw "${case}: expected '$expected', got '$errorMessage'" }
        if (Test-Path -LiteralPath $output) { throw 'Rejected candidate created an archive' }
    }
    'PASS: mismatched binary and inherited acceptance rejected before packaging (2 cases)'
} finally {
    $resolved = [IO.Path]::GetFullPath($fixtureRoot)
    if ([IO.Path]::GetDirectoryName($resolved) -ne $tempBase.TrimEnd([IO.Path]::DirectorySeparatorChar) -or [IO.Path]::GetFileName($resolved) -notmatch '^ledger-distribution-[0-9a-f]{32}$') { throw 'Unsafe test cleanup target' }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
