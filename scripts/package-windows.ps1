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
Copy-Item -LiteralPath $packageBinary -Destination (Join-Path $stage 'project-ledger.exe')
Copy-Item -LiteralPath (Join-Path $packageSource 'LICENSE') -Destination (Join-Path $stage 'LICENSE')
$files = [Collections.Generic.List[object]]::new()
foreach ($entry in $catalog.files) {
    if ($entry.path -notmatch '^[A-Za-z0-9_.\-/]+$' -or $entry.path -match '(^|/)\.\.(/|$)' -or [IO.Path]::IsPathRooted($entry.path)) { throw 'Unsafe catalog path' }
    $source = Join-Path $packageSource $entry.path
    $destination = Join-Path $stage ('resources/' + $entry.path)
    $null = New-Item -ItemType Directory -Path (Split-Path -Parent $destination) -Force
    Copy-Item -LiteralPath $source -Destination $destination
    if ((Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant() -ne $entry.sha256) { throw 'Resource changed while packaging' }
    $files.Add(@{path=('resources/' + $entry.path); size=$entry.size; sha256=$entry.sha256})
}
foreach ($name in @('project-ledger.exe','LICENSE')) {
    $path = Join-Path $stage $name
    $files.Add(@{path=$name; size=(Get-Item -LiteralPath $path).Length; sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()})
}
$manifest = [ordered]@{
    profile='project-ledger-windows-zip/1'; source_commit=$sourceCommit; source_tree_dirty=$dirty
    binary='project-ledger.exe'; target='x86_64-pc-windows-msvc'; certified_compatibility=$false
    core_requires=@('Windows x86_64'); optional_gsp='See resources/docs/WINDOWS_PORTABLE.md; never downloaded automatically'
    files=$files.ToArray()
}
$manifest | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $stage 'package.json') -Encoding utf8NoBOM
$null = New-Item -ItemType Directory -Path (Split-Path -Parent $packageOutput) -Force
# Exclusive creation: never replace another release, even if the destination appears after preflight.
$stream = [IO.File]::Open($packageOutput, [IO.FileMode]::CreateNew, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None)
try { [IO.Compression.ZipFile]::CreateFromDirectory($stage, $stream, [IO.Compression.CompressionLevel]::Optimal, $false) }
finally { $stream.Dispose() }
[ordered]@{package=$packageOutput; sha256=(Get-FileHash -LiteralPath $packageOutput -Algorithm SHA256).Hash.ToLowerInvariant(); source_commit=$sourceCommit; source_tree_dirty=$dirty; files=$files.Count; staging_directory=$stage} | ConvertTo-Json
