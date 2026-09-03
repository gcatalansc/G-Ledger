[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string]$Executable,

    [Parameter(Mandatory)]
    [string[]]$ProcessArguments,

    [Parameter(Mandatory)]
    [string]$RequestsFile,

    [ValidateRange(1000, 120000)]
    [int]$TimeoutMilliseconds = 30000
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$resolvedExecutable = (Resolve-Path -LiteralPath $Executable).Path
$resolvedRequests = (Resolve-Path -LiteralPath $RequestsFile).Path
$requests = @(Get-Content -Raw -LiteralPath $resolvedRequests | ConvertFrom-Json)
if ($requests.Count -eq 0) {
    throw 'RECORD_SINK_RPC_REQUESTS_EMPTY'
}

$input = [IO.MemoryStream]::new()
foreach ($request in $requests) {
    $body = $request | ConvertTo-Json -Depth 30 -Compress
    $bodyBytes = [Text.Encoding]::UTF8.GetBytes($body)
    $headerBytes = [Text.Encoding]::ASCII.GetBytes(
        "Content-Length: $($bodyBytes.Length)`r`n`r`n"
    )
    $input.Write($headerBytes, 0, $headerBytes.Length)
    $input.Write($bodyBytes, 0, $bodyBytes.Length)
}
$inputBytes = $input.ToArray()
$input.Dispose()

$startInfo = [Diagnostics.ProcessStartInfo]::new()
$startInfo.FileName = $resolvedExecutable
$startInfo.UseShellExecute = $false
$startInfo.CreateNoWindow = $true
$startInfo.RedirectStandardInput = $true
$startInfo.RedirectStandardOutput = $true
$startInfo.RedirectStandardError = $true
foreach ($argument in $ProcessArguments) {
    $startInfo.ArgumentList.Add($argument)
}

$process = [Diagnostics.Process]::new()
$process.StartInfo = $startInfo
if (-not $process.Start()) {
    throw 'RECORD_SINK_RPC_PROCESS_NOT_STARTED'
}

$stdout = [IO.MemoryStream]::new()
$stdoutTask = $process.StandardOutput.BaseStream.CopyToAsync($stdout)
$stderrTask = $process.StandardError.ReadToEndAsync()
$process.StandardInput.BaseStream.Write($inputBytes, 0, $inputBytes.Length)
$process.StandardInput.BaseStream.Flush()
$process.StandardInput.Close()

if (-not $process.WaitForExit($TimeoutMilliseconds)) {
    $process.Kill($true)
    $process.WaitForExit()
    throw "RECORD_SINK_RPC_TIMEOUT: $TimeoutMilliseconds"
}
$null = $stdoutTask.GetAwaiter().GetResult()
$stderr = $stderrTask.GetAwaiter().GetResult()
$exitCode = $process.ExitCode
$process.Dispose()
if ($exitCode -ne 0) {
    throw "RECORD_SINK_RPC_PROCESS_FAILED: exit=$exitCode stderr=$stderr"
}

$bytes = $stdout.ToArray()
$stdout.Dispose()
$responses = @()
$offset = 0
while ($offset -lt $bytes.Length) {
    $headerEnd = -1
    for ($index = $offset; $index -le $bytes.Length - 4; $index++) {
        if ($bytes[$index] -eq 13 -and $bytes[$index + 1] -eq 10 -and
            $bytes[$index + 2] -eq 13 -and $bytes[$index + 3] -eq 10) {
            $headerEnd = $index
            break
        }
    }
    if ($headerEnd -lt 0) {
        throw 'RECORD_SINK_RPC_HEADER_TRUNCATED'
    }
    $header = [Text.Encoding]::ASCII.GetString($bytes, $offset, $headerEnd - $offset)
    if ($header -notmatch '\AContent-Length: ([0-9]+)\z') {
        throw "RECORD_SINK_RPC_HEADER_INVALID: $header"
    }
    $length = [int]$Matches[1]
    $bodyStart = $headerEnd + 4
    $bodyEnd = $bodyStart + $length
    if ($length -lt 0 -or $bodyEnd -gt $bytes.Length) {
        throw 'RECORD_SINK_RPC_BODY_TRUNCATED'
    }
    $body = [Text.Encoding]::UTF8.GetString($bytes, $bodyStart, $length)
    $responses += $body | ConvertFrom-Json
    $offset = $bodyEnd
}
if ($offset -ne $bytes.Length) {
    throw 'RECORD_SINK_RPC_TRAILING_BYTES'
}

$responses | ConvertTo-Json -Depth 30 -Compress -AsArray
