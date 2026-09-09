#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Binary,
    [Parameter(Mandatory)][string]$Output,
    [string]$SourceRoot = (Split-Path -Parent $PSScriptRoot),
    [switch]$RequireClean
)
$ErrorActionPreference = 'Stop'
$packageSource = (Resolve-Path -LiteralPath $SourceRoot).Path
$packageBinary = (Resolve-Path -LiteralPath $Binary).Path
$packageOutput = [IO.Path]::GetFullPath($Output)
if (Test-Path -LiteralPath $packageOutput) { throw 'Package destination already exists' }
$distribution = Get-Content -LiteralPath (Join-Path $packageSource 'release/distribution.json') -Raw | ConvertFrom-Json
if ($distribution.profile -ne 'project-ledger-distribution/1' -or
    $distribution.package_profile -ne 'project-ledger-windows-zip/2' -or
    $distribution.product_version -notmatch '^\d+\.\d+\.\d+-rc\.\d+$' -or
    $distribution.operational_acceptance -ne 'NOT_GRANTED' -or
    $distribution.release_status -ne 'ENGINEERING_RELEASE_CANDIDATE' -or
    $distribution.certified_compatibility -ne $false -or $distribution.prerelease -ne $true -or
    $distribution.release_notes -ne "docs/RELEASE_NOTES_v$($distribution.product_version).md") { throw 'Invalid distribution classification' }
$binaryVersion = & $packageBinary --version
if ($LASTEXITCODE -ne 0 -or ($binaryVersion -join "`n").Trim() -ne "project-ledger $($distribution.product_version)") { throw 'Distribution binary version mismatch' }
$sourceCommit = & git -C $packageSource rev-parse HEAD
if ($LASTEXITCODE -ne 0) { throw 'Cannot identify source commit' }
$dirty = [bool](& git -C $packageSource status --porcelain)
if ($LASTEXITCODE -ne 0) { throw 'Cannot verify the source tree status' }
if ($RequireClean -and $dirty) { throw 'Release packaging requires a clean source tree' }
$catalogText = & $packageBinary resources list --json
if ($LASTEXITCODE -ne 0) { throw 'Binary does not expose its resource catalog' }
$catalog = ($catalogText -join "`n") | ConvertFrom-Json
& $packageBinary --resources $packageSource resources verify --json
if ($LASTEXITCODE -ne 0) { throw 'Binary and source resources differ; rebuild the binary' }
$stage = Join-Path ([IO.Path]::GetTempPath()) ('Ledger empaquetado á ' + [guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory -Path $stage
$files = [Collections.Generic.List[object]]::new()
$packagePaths = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
function Add-PackageFile([string]$Source, [string]$Target) {
    $normalized = $Target.Replace('\','/')
    if ($normalized -notmatch '^[A-Za-z0-9_.\-/]+$' -or $normalized -match '(^|/)\.\.(/|$)' -or [IO.Path]::IsPathRooted($normalized) -or -not $packagePaths.Add($normalized)) { throw "Unsafe or duplicate package path: $Target" }
    $sourcePath = (Resolve-Path -LiteralPath $Source).Path
    $destination = Join-Path $stage $normalized
    $null = New-Item -ItemType Directory -Path (Split-Path -Parent $destination) -Force
    Copy-Item -LiteralPath $sourcePath -Destination $destination
    $null = $files.Add([ordered]@{path=$normalized; size=(Get-Item -LiteralPath $destination).Length; sha256=(Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant()})
}

Add-PackageFile $packageBinary 'project-ledger.exe'
foreach ($entry in @(
    @{Source='packaging/windows/START_HERE.md'; Target='START_HERE.md'},
    @{Source='packaging/windows/ASSURANCE.md'; Target='assurance/README.md'},
    @{Source='LICENSE'; Target='LICENSE'},
    @{Source='CHANGELOG.md'; Target='CHANGELOG.md'},
    @{Source='CONTRIBUTING.md'; Target='CONTRIBUTING.md'},
    @{Source='SECURITY.md'; Target='SECURITY.md'},
    @{Source='SPEC.md'; Target='SPEC.md'},
    @{Source='BITACORA_IMPLEMENTACION.md'; Target='BITACORA_IMPLEMENTACION.md'},
    @{Source=$distribution.release_notes; Target='RELEASE_NOTES.md'},
    @{Source='release/sbom.spdx.json'; Target='SBOM.spdx.json'},
    @{Source='docs/pilot/results/operational-pilot-report.json'; Target='assurance/operational-pilot-report.json'},
    @{Source='docs/pilot/results/stage-8-risk-close-operational-acceptance.json'; Target='assurance/stage-8-risk-close-operational-acceptance.json'},
    @{Source='release/acceptance-report.json'; Target='assurance/acceptance-report.json'},
    @{Source='release/compatibility-report.json'; Target='assurance/compatibility-report.json'},
    @{Source='release/release-manifest.json'; Target='assurance/release-manifest.json'}
)) {
    Add-PackageFile (Join-Path $packageSource $entry.Source) $entry.Target
}
foreach ($tree in @(
    @{SourcePrefix='docs/'; TargetPrefix='documentation/'},
    @{SourcePrefix='specs/'; TargetPrefix='specs/'},
    @{SourcePrefix='referencias/PROJECT_LEDGER_v0.2.0_CODEX/'; TargetPrefix='referencias/PROJECT_LEDGER_v0.2.0_CODEX/'},
    @{SourcePrefix='integrations/agent-skills/'; TargetPrefix='integrations/agent-skills/'},
    @{SourcePrefix='release/'; TargetPrefix='release/'}
)) {
    $tracked = @(& git -C $packageSource ls-files -- $tree.SourcePrefix)
    if ($LASTEXITCODE -ne 0 -or $tracked.Count -eq 0) { throw "Could not enumerate reviewed delivery tree: $($tree.SourcePrefix)" }
    foreach ($relative in $tracked) {
        $normalized = $relative.Replace('\','/')
        if (-not $normalized.StartsWith($tree.SourcePrefix, [StringComparison]::Ordinal)) { throw "Unexpected tracked delivery path: $normalized" }
        $suffix = $normalized.Substring($tree.SourcePrefix.Length)
        Add-PackageFile (Join-Path $packageSource $normalized) ($tree.TargetPrefix + $suffix)
    }
}
foreach ($entry in $catalog.files) {
    if ($entry.path -notmatch '^[A-Za-z0-9_.\-/]+$' -or $entry.path -match '(^|/)\.\.(/|$)' -or [IO.Path]::IsPathRooted($entry.path)) { throw 'Unsafe catalog path' }
    $source = Join-Path $packageSource $entry.path
    $target = 'resources/' + $entry.path
    Add-PackageFile $source $target
    $packaged = $files[$files.Count - 1]
    if ($packaged.sha256 -ne $entry.sha256 -or $packaged.size -ne $entry.size) { throw 'Resource changed while packaging' }
}
$checksumPath = Join-Path $stage 'CHECKSUMS.sha256'
@($files | Sort-Object path | ForEach-Object { "$($_.sha256)  $($_.path)" }) | Set-Content -LiteralPath $checksumPath -Encoding ascii
if (-not $packagePaths.Add('CHECKSUMS.sha256')) { throw 'Duplicate package checksum path' }
$null = $files.Add([ordered]@{path='CHECKSUMS.sha256'; size=(Get-Item -LiteralPath $checksumPath).Length; sha256=(Get-FileHash -LiteralPath $checksumPath -Algorithm SHA256).Hash.ToLowerInvariant()})
$manifest = [ordered]@{
    profile=$distribution.package_profile; source_commit=$sourceCommit; source_tree_dirty=$dirty
    product_version=$distribution.product_version; distribution_manifest='release/distribution.json'
    binary='project-ledger.exe'; target='x86_64-pc-windows-msvc'; certified_compatibility=$false
    release_status=$distribution.release_status; operational_acceptance=$distribution.operational_acceptance
    documentation_entrypoint='START_HERE.md'; source_delivery='Git source at source_commit; candidate may not have a release tag'
    core_requires=@('Windows x86_64','PowerShell 7 for documented operator commands'); optional_gsp='See documentation/WINDOWS_PORTABLE.md; never downloaded automatically'
    files=$files.ToArray()
}
$manifest | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $stage 'package.json') -Encoding utf8NoBOM
$null = New-Item -ItemType Directory -Path (Split-Path -Parent $packageOutput) -Force
# Exclusive creation: never replace another release, even if the destination appears after preflight.
$stream = [IO.File]::Open($packageOutput, [IO.FileMode]::CreateNew, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None)
try { [IO.Compression.ZipFile]::CreateFromDirectory($stage, $stream, [IO.Compression.CompressionLevel]::Optimal, $false) }
finally { $stream.Dispose() }
[ordered]@{package=$packageOutput; sha256=(Get-FileHash -LiteralPath $packageOutput -Algorithm SHA256).Hash.ToLowerInvariant(); source_commit=$sourceCommit; source_tree_dirty=$dirty; files=$files.Count; staging_directory=$stage} | ConvertTo-Json
