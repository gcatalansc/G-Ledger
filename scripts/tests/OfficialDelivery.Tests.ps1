#requires -Version 7.0

Describe 'Official documentation entrypoint' -Tag 'Documentation' {
    $repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
    $documentationFiles = @(
        'docs/README.md',
        'docs/getting-started/QUICKSTART.md',
        'docs/getting-started/TUTORIAL.md',
        'docs/operations/USER_GUIDE.md',
        'docs/assurance/README.md',
        'docs/SOURCE_AND_BUILD.md'
    )

    foreach ($relative in $documentationFiles) {
        It "$relative exists" {
            Test-Path -LiteralPath (Join-Path $repoRoot $relative) | Should Be $true
        }
    }

    It 'the repository README routes readers to the documentation home' {
        $readme = Get-Content -LiteralPath (Join-Path $repoRoot 'README.md') -Raw
        $pattern = [regex]::Escape('(docs/README.md)')
        $readme | Should Match $pattern
    }

    It 'all local links in the curated documentation resolve' {
        $missing = [Collections.Generic.List[string]]::new()
        foreach ($relative in $documentationFiles) {
            $document = Join-Path $repoRoot $relative
            if (-not (Test-Path -LiteralPath $document)) {
                $missing.Add("$relative -> FILE_MISSING")
                continue
            }
            $content = Get-Content -LiteralPath $document -Raw
            foreach ($match in [regex]::Matches($content, '\[[^\]]+\]\((?<target>[^)]+)\)')) {
                $target = $match.Groups['target'].Value.Trim('<', '>')
                if ($target -match '^(?:https?://|mailto:|#)' -or [string]::IsNullOrWhiteSpace($target)) { continue }
                $pathOnly = ($target -split '#', 2)[0]
                $resolved = [IO.Path]::GetFullPath((Join-Path (Split-Path -Parent $document) $pathOnly))
                if (-not (Test-Path -LiteralPath $resolved)) { $missing.Add("$relative -> $target") }
            }
        }
        if ($missing.Count -ne 0) { throw "Broken curated documentation links: $($missing -join '; ')" }
    }
}

Describe 'Operational candidate release policy' -Tag 'ReleaseDocumentation' {
    $repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
    $releaseNotesPath = Join-Path $repoRoot 'docs/RELEASE_NOTES_v0.2.0-rc.1.md'

    It 'publishes explicit bounded release labels' {
        Test-Path -LiteralPath $releaseNotesPath | Should Be $true
        if (Test-Path -LiteralPath $releaseNotesPath) {
            $notes = Get-Content -LiteralPath $releaseNotesPath -Raw
            $notes | Should Match 'OPERATIONALLY_ACCEPTED'
            $notes | Should Match 'ENGINEERING_RELEASE_CANDIDATE'
            $notes | Should Match 'certified_compatibility=false'
            $notes | Should Match '(?i)prerelease'
            $notes | Should Match '(?i)no es General Availability'
        }
    }
}
