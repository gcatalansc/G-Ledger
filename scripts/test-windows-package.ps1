#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Zip,
    [Parameter(Mandatory)][string]$ExpectedSha256,
    [Parameter(Mandatory)][string]$CodexTest,
    [Parameter(Mandatory)][string]$PortableTest,
    [Parameter(Mandatory)][string]$HistoryTest,
    [string]$Report
)
$ErrorActionPreference = 'Stop'
if ($Report -and (Test-Path -LiteralPath $Report)) { throw 'Verification report destination already exists' }
$zipPath = (Resolve-Path -LiteralPath $Zip).Path
if ((Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash -ne $ExpectedSha256) { throw 'ZIP checksum mismatch' }
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ('Ledger prueba ZIP á ' + [guid]::NewGuid().ToString('N'))
$install = Join-Path $testRoot 'instalación con espacios'
$null = New-Item -ItemType Directory -Path $install
$archive = [IO.Compression.ZipFile]::OpenRead($zipPath)
try {
    $names = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    $total = 0L
    foreach ($entry in $archive.Entries) {
        $name = $entry.FullName.Replace('\','/')
        if ($name -match '(^/|^[A-Za-z]:|(^|/)\.\.(/|$)|:)' -or -not $names.Add($name)) { throw 'Unsafe or duplicate ZIP entry' }
        if ($name -match '(^|/)(?:\.git|target|dist|\.tooling)(/|$)' -or $name -match '\.(?:db|db-shm|db-wal)$') { throw "Forbidden delivery entry: $name" }
        $total += $entry.Length
        if ($total -gt 536870912 -or $names.Count -gt 2048) { throw 'ZIP extraction budget exceeded' }
    }
    foreach ($required in @(
        'START_HERE.md',
        'project-ledger.exe',
        'LICENSE',
        'CHANGELOG.md',
        'SECURITY.md',
        'RELEASE_NOTES.md',
        'SBOM.spdx.json',
        'CHECKSUMS.sha256',
        'package.json',
        'documentation/README.md',
        'documentation/getting-started/QUICKSTART.md',
        'documentation/getting-started/TUTORIAL.md',
        'documentation/operations/USER_GUIDE.md',
        'documentation/CLI_OPERATIONS.md',
        'documentation/WINDOWS_PORTABLE.md',
        'assurance/README.md',
        'assurance/operational-pilot-report.json',
        'assurance/acceptance-report.json',
        'assurance/compatibility-report.json',
        'assurance/release-manifest.json',
        'documentation/pilot/results/G_LEDGER_OPERATIONAL_ACCEPTANCE_FINAL_REPORT_v1.0.0.md',
        'documentation/pilot/G_LEDGER_OPERATIONAL_ACCEPTANCE_MATRIX.md',
        'documentation/pilot/G_LEDGER_OPERATIONAL_ACCEPTANCE_PILOT_SPEC_v1.0.0.md',
        'SPEC.md',
        'BITACORA_IMPLEMENTACION.md',
        'specs/PROJECT_LEDGER_SPEC_v0.2.0.md',
        'release/release-manifest.json'
        'release/distribution.json'
        'documentation/history/LLM_USAGE.md'
    )) {
        if (-not $names.Contains($required)) { throw "Official delivery file missing: $required" }
    }
    [IO.Compression.ZipFileExtensions]::ExtractToDirectory($archive, $install)
} finally { $archive.Dispose() }
$manifest = Get-Content -LiteralPath (Join-Path $install 'package.json') -Raw | ConvertFrom-Json
if ($manifest.profile -ne 'project-ledger-windows-zip/2' -or $manifest.certified_compatibility -ne $false) { throw 'Invalid package profile' }
if ($manifest.documentation_entrypoint -ne 'START_HERE.md' -or $manifest.release_status -ne 'ENGINEERING_RELEASE_CANDIDATE' -or $manifest.operational_acceptance -ne 'NOT_GRANTED') { throw 'Invalid delivery classification' }
$distribution = Get-Content -LiteralPath (Join-Path $install 'release/distribution.json') -Raw | ConvertFrom-Json
if ($distribution.product_version -ne $manifest.product_version -or $distribution.package_profile -ne $manifest.profile -or $distribution.operational_acceptance -ne 'NOT_GRANTED' -or $distribution.certified_compatibility -ne $false) { throw 'Package and distribution differ' }
$listed = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($entry in $manifest.files) {
    $name = $entry.path
    if ([IO.Path]::IsPathRooted($name) -or $name -match '(^|/)\.\.(/|$)|:|\\' -or -not $listed.Add($name)) { throw 'Unsafe or duplicate manifest path' }
    $path = Join-Path $install $name
    $item = Get-Item -LiteralPath $path
    if ($item.LinkType -or $item.Length -ne $entry.size -or (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ne $entry.sha256) { throw "Package integrity failure: $name" }
}
$null = $listed.Add('package.json')
if (-not $listed.Contains('LICENSE')) { throw 'Project license missing from distribution' }
foreach ($item in Get-ChildItem -LiteralPath $install -File -Recurse -Force) {
    $name = [IO.Path]::GetRelativePath($install, $item.FullName).Replace('\','/')
    if (-not $listed.Contains($name)) { throw 'Unlisted package file' }
}
$markdownEntrypoints = @(
    'START_HERE.md',
    'documentation/README.md',
    'documentation/getting-started/QUICKSTART.md',
    'documentation/getting-started/TUTORIAL.md',
    'documentation/operations/USER_GUIDE.md',
    'documentation/SOURCE_AND_BUILD.md',
    'assurance/README.md'
)
foreach ($relative in $markdownEntrypoints) {
    $document = Join-Path $install $relative
    $content = Get-Content -LiteralPath $document -Raw
    foreach ($match in [regex]::Matches($content, '\[[^\]]+\]\((?<target>[^)]+)\)')) {
        $target = $match.Groups['target'].Value.Trim('<', '>')
        if ($target -match '^(?:https?://|mailto:|#)' -or [string]::IsNullOrWhiteSpace($target)) { continue }
        $pathOnly = ($target -split '#', 2)[0]
        $resolved = [IO.Path]::GetFullPath((Join-Path (Split-Path -Parent $document) $pathOnly))
        if (-not (Test-Path -LiteralPath $resolved)) { throw "Broken package documentation link: $relative -> $target" }
    }
}
$checksumLines = @(Get-Content -LiteralPath (Join-Path $install 'CHECKSUMS.sha256') | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
$checksumPaths = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($line in $checksumLines) {
    if ($line -notmatch '^([0-9a-f]{64})  ([A-Za-z0-9_.\-/]+)$') { throw 'Invalid delivery checksum line' }
    $expected = $Matches[1]
    $name = $Matches[2]
    if (-not $checksumPaths.Add($name) -or $name -eq 'package.json' -or $name -eq 'CHECKSUMS.sha256') { throw 'Invalid delivery checksum scope' }
    $path = Join-Path $install $name
    if (-not (Test-Path -LiteralPath $path -PathType Leaf) -or (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected) { throw "Delivery checksum mismatch: $name" }
}
$expectedChecksumPaths = @($listed | Where-Object { $_ -notin @('package.json','CHECKSUMS.sha256') })
if ($checksumPaths.Count -ne $expectedChecksumPaths.Count) { throw 'Delivery checksum inventory is incomplete' }
foreach ($name in $expectedChecksumPaths) {
    if (-not $checksumPaths.Contains($name)) { throw "Delivery checksum missing: $name" }
}
$binary = Join-Path $install 'project-ledger.exe'
# Parse the PE import table: the portable core may import Windows system DLLs, not an MSVC redistributable.
$pe = [IO.File]::ReadAllBytes($binary)
$header = [BitConverter]::ToInt32($pe, 0x3c)
if ([BitConverter]::ToUInt32($pe,$header) -ne 0x4550 -or [BitConverter]::ToUInt16($pe,$header+4) -ne 0x8664) { throw 'Expected a Windows x86_64 PE' }
$optional = $header + 24
if ([BitConverter]::ToUInt16($pe,$optional) -ne 0x20b) { throw 'Expected a PE32+ image' }
$sections = $optional + [BitConverter]::ToUInt16($pe,$header+20)
$sectionCount = [BitConverter]::ToUInt16($pe,$header+6)
function Convert-Rva([uint32]$Rva) {
    for ($n=0; $n -lt $sectionCount; $n++) {
        $offset = $sections + 40*$n
        $start = [BitConverter]::ToUInt32($pe,$offset+12)
        $length = [Math]::Max([BitConverter]::ToUInt32($pe,$offset+8),[BitConverter]::ToUInt32($pe,$offset+16))
        if ($Rva -ge $start -and $Rva -lt $start+$length) { return [int]($Rva-$start+[BitConverter]::ToUInt32($pe,$offset+20)) }
    }
    throw 'Invalid PE import address'
}
$imports = [Collections.Generic.List[string]]::new()
$descriptor = Convert-Rva ([BitConverter]::ToUInt32($pe,$optional+120))
for ($n=0; $n -lt 256; $n++) {
    $rva = [BitConverter]::ToUInt32($pe,$descriptor+20*$n+12)
    if ($rva -eq 0) { break }
    $offset = Convert-Rva $rva
    $end = $offset
    while ($end -lt $pe.Length -and $end-$offset -lt 256 -and $pe[$end] -ne 0) { $end++ }
    $dll = [Text.Encoding]::ASCII.GetString($pe,$offset,$end-$offset)
    if ($dll -match '^(VCRUNTIME|MSVCP|MSVCR|ucrtbase)' -or $dll -notmatch '^(api-ms-win-[\w.-]+|[A-Za-z0-9_-]+)\.dll$') { throw "Non-portable runtime import: $dll" }
    $imports.Add($dll)
}
function Invoke-Isolated([string[]]$Arguments, [int]$ExpectedExit=0) {
    $start = [Diagnostics.ProcessStartInfo]::new($binary)
    $start.WorkingDirectory = $testRoot
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.Environment.Clear()
    foreach ($key in @('SystemRoot','TEMP','TMP')) { $start.Environment[$key] = [Environment]::GetEnvironmentVariable($key) }
    foreach ($argument in $Arguments) { $start.ArgumentList.Add($argument) }
    $process = [Diagnostics.Process]::Start($start)
    $stdout = $process.StandardOutput.ReadToEndAsync()
    $stderr = $process.StandardError.ReadToEndAsync()
    if (-not $process.WaitForExit(60000)) { $process.Kill($true); $process.WaitForExit(); throw 'Package command timed out' }
    if ($process.ExitCode -ne $ExpectedExit) { throw "Package command failed: $($Arguments -join ' ') / $($stdout.Result) / $($stderr.Result)" }
    return $stdout.Result
}
if ((Invoke-Isolated @('--version')).Trim() -ne "project-ledger $($manifest.product_version)") { throw 'Installed binary version mismatch' }
foreach ($family in @('resources','specs','contracts')) { $null = Invoke-Isolated @($family,'verify','--json') }
$releaseReport = Invoke-Isolated @('release','verify','--json') | ConvertFrom-Json
if ($releaseReport.distribution.product_version -ne $manifest.product_version -or $releaseReport.distribution.operational_acceptance -ne 'NOT_GRANTED' -or $releaseReport.verified_evidence_subject -ne 'v0.2.0-baseline') { throw 'Installed release verification conflates subjects' }
$harnessResults = [Collections.Generic.List[object]]::new()
foreach ($test in @(
    @($CodexTest,'published_codex_argv_runs_the_full_governed_flow_without_direct_storage_access'),
    @($PortableTest,'portable_artifact_operates_recovers_and_rejects_private_exports_without_toolchains'),
    @($PortableTest,'portable_artifact_operates_and_recovers_beyond_windows_max_path'),
    @($HistoryTest,'trace_reconstructs_supersession_attempts_and_evidence_invalidation')
)) {
    $start = [Diagnostics.ProcessStartInfo]::new((Resolve-Path -LiteralPath $test[0]).Path)
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.WorkingDirectory = $testRoot
    $start.Environment['PROJECT_LEDGER_PACKAGE_BINARY'] = $binary
    $start.Environment['CARGO_BIN_EXE_project-ledger'] = $binary
    foreach ($argument in @('--exact',$test[1],'--nocapture')) { $start.ArgumentList.Add($argument) }
    $process = [Diagnostics.Process]::Start($start)
    $stdout = $process.StandardOutput.ReadToEndAsync()
    $stderr = $process.StandardError.ReadToEndAsync()
    if (-not $process.WaitForExit(120000)) { $process.Kill($true); $process.WaitForExit(); throw 'Package scenario timed out' }
    if ($process.ExitCode -ne 0 -or $stdout.Result -notmatch 'test result: ok\. 1 passed; 0 failed') { throw "Extracted package scenario missing or failed: $($stdout.Result) / $($stderr.Result)" }
    $harnessResults.Add(@{test=$test[1]; passed=1; failed=0})
    Write-Host $stdout.Result
}
# Missing optional GSP must be bounded, fail without a result, and leave core operation available.
$missing = Join-Path $testRoot 'missing-python.exe'
$policy = Join-Path $testRoot 'optional-policy.json'
@{allowed_programs=@($missing)} | ConvertTo-Json | Set-Content -LiteralPath $policy -Encoding utf8NoBOM
$null = Invoke-Isolated @('integration','gsp','compile','--python',$missing,'--source-root',$testRoot,'--input',(Join-Path $testRoot 'missing-corpus.json'),'--output',(Join-Path $testRoot 'must-not-exist.json'),'--policy',$policy,'--cwd',$testRoot,'--json') 12
if (Test-Path -LiteralPath (Join-Path $testRoot 'must-not-exist.json')) { throw 'Missing runtime published a result' }
# Tampering does not fall back to the development checkout or accidental cwd.
$modified = Join-Path $install 'resources/specs/PROJECT_LEDGER_SPEC_v0.2.0.md'
$original = [IO.File]::ReadAllBytes($modified)
try {
    [IO.File]::WriteAllText($modified, 'tampered disposable package')
    $null = Invoke-Isolated @('resources','verify','--json') 15
} finally { [IO.File]::WriteAllBytes($modified,$original) }
$null = Invoke-Isolated @('resources','verify','--json')
$result = [ordered]@{
    profile='project-ledger-package-verification/1'; status='PASS'; zip_sha256=$ExpectedSha256.ToLowerInvariant()
    source_commit=$manifest.source_commit; source_tree_dirty=$manifest.source_tree_dirty; extracted_directory=$install
    product_version=$manifest.product_version; operational_acceptance=$manifest.operational_acceptance
    scenarios=@('official-document-layout','documentation-links','delivery-checksums','compiled-resource-hashes','specs-contracts-release','distribution-identity','Codex-full-flow','portable-backup-restore-replay','windows-long-data-paths','history-decision-trace','privacy-and-error-codes','missing-optional-GSP','resource-tamper-rejected')
    runtime_environment='PATH absent; only SystemRoot/TEMP/TMP passed to Ledger; no Cargo/Git/Python/checkout required by core'
    imported_system_dlls=$imports.ToArray(); certified_compatibility=$false
    harness_results=$harnessResults.ToArray()
}
$json = $result | ConvertTo-Json -Depth 8
if ($Report) { $json | Set-Content -LiteralPath $Report -Encoding utf8NoBOM }
$json
