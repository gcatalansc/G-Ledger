#requires -Version 7.0
[CmdletBinding()]
param([Parameter(Mandatory)][string]$Output, [switch]$RequireClean)
$ErrorActionPreference = 'Stop'
$packageRoot = Split-Path -Parent $PSScriptRoot
$zipPath = [IO.Path]::GetFullPath($Output)
$verificationPath = $zipPath + '.verification.json'
$checksumPath = $zipPath + '.sha256'
foreach ($destination in @($zipPath,$verificationPath,$checksumPath)) {
    if (Test-Path -LiteralPath $destination) { throw "Delivery destination already exists: $destination" }
}
Push-Location -LiteralPath $packageRoot
try {
    $previousFlags = $env:RUSTFLAGS
    try {
        $env:RUSTFLAGS = ($previousFlags + ' -C target-feature=+crt-static').Trim()
        & cargo build --locked --release --target x86_64-pc-windows-msvc --target-dir target/windows-portable -p project-ledger-cli
        if ($LASTEXITCODE -ne 0) { throw 'Static-CRT Windows build failed' }
    } finally { $env:RUSTFLAGS = $previousFlags }
    $messages = & cargo test --locked -p project-ledger-cli --test codex_readiness --test package_flow --test history --no-run --message-format=json
    if ($LASTEXITCODE -ne 0) { throw 'Could not build external test harnesses' }
    $artifacts = @($messages | ForEach-Object { $_ | ConvertFrom-Json } | Where-Object { $_.reason -eq 'compiler-artifact' -and $_.executable })
    $codex = ($artifacts | Where-Object { $_.target.name -eq 'codex_readiness' }).executable
    $portable = ($artifacts | Where-Object { $_.target.name -eq 'package_flow' }).executable
    $history = ($artifacts | Where-Object { $_.target.name -eq 'history' }).executable
    if (-not $codex -or -not $portable -or -not $history) { throw 'Expected test executable missing' }
    $binary = Join-Path $packageRoot 'target/windows-portable/x86_64-pc-windows-msvc/release/project-ledger.exe'
    & (Join-Path $PSScriptRoot 'test-distribution-preflight.ps1') -Binary $binary
    & (Join-Path $PSScriptRoot 'package-windows.ps1') -Binary $binary -Output $Output -SourceRoot $packageRoot -RequireClean:$RequireClean
    $hash = (Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash
    "$($hash.ToLowerInvariant())  $([IO.Path]::GetFileName($zipPath))" | Set-Content -LiteralPath $checksumPath -Encoding ascii
    & (Join-Path $PSScriptRoot 'test-windows-package.ps1') -Zip $zipPath -ExpectedSha256 $hash -CodexTest $codex -PortableTest $portable -HistoryTest $history -Report $verificationPath
    if ($LASTEXITCODE -ne 0) { throw 'Official Windows package verification failed' }
} finally { Pop-Location }
