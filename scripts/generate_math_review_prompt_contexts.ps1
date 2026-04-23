param(
    [string]$PromptDoc = "Math Review/MATH_REVIEW_PROMPT_PACKETS.md",
    [string]$OutputRoot = "Math Review/Prompt Contexts"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$promptDocPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot $PromptDoc))
$outputRootPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot $OutputRoot))
$repoRootPath = [System.IO.Path]::GetFullPath($repoRoot)

if (-not (Test-Path -LiteralPath $promptDocPath)) {
    throw "Prompt doc not found: $promptDocPath"
}

if (-not $outputRootPath.StartsWith($repoRootPath, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to write outside the repo root: $outputRootPath"
}

function Get-SafeFolderName {
    param(
        [string]$Id,
        [string]$Title
    )

    $name = "$Id $Title"
    $invalidChars = [System.IO.Path]::GetInvalidFileNameChars()
    foreach ($char in $invalidChars) {
        $escaped = [regex]::Escape([string]$char)
        $name = $name -replace $escaped, "-"
    }
    $name = $name -replace "\s+", " "
    return $name.Trim()
}

function Add-UniquePath {
    param(
        [System.Collections.Generic.List[string]]$Paths,
        [string]$Candidate
    )

    if ([string]::IsNullOrWhiteSpace($Candidate)) {
        return
    }

    $trimmed = $Candidate.Trim()
    if (-not ($trimmed -match '[\\/]' -or $trimmed -match '\.(rs|cu|md|toml|ps1)$')) {
        return
    }

    if (-not $Paths.Contains($trimmed)) {
        [void]$Paths.Add($trimmed)
    }
}

function Get-ContextPathsFromSection {
    param(
        [string]$SectionText
    )

    $paths = [System.Collections.Generic.List[string]]::new()
    foreach ($line in ($SectionText -split "`r?`n")) {
        $trimmed = $line.Trim()
        if ($trimmed -match '^\-\s+`(?<path>[^`]+)`$') {
            Add-UniquePath -Paths $paths -Candidate $matches['path']
            continue
        }
        if ($trimmed -match '^\-\s+in\s+`(?<path>[^`]+)`:$') {
            Add-UniquePath -Paths $paths -Candidate $matches['path']
            continue
        }
    }
    return $paths
}

function Get-RelativeDisplayPath {
    param(
        [string]$FullPath
    )

    $relative = $FullPath.Substring($repoRootPath.Length).TrimStart('\', '/')
    return $relative -replace '\\', '/'
}

function Build-CorePromptMarkdown {
    param(
        [pscustomobject]$Item,
        [System.Collections.Generic.List[string]]$Copied,
        [System.Collections.Generic.List[string]]$Missing
    )

    $lines = [System.Collections.Generic.List[string]]::new()
    $lines.Add("# $($Item.Id) $($Item.Title)") | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add(('- package type: `{0}`' -f $Item.Kind)) | Out-Null
    $lines.Add(('- copied context files: `{0}`' -f $Copied.Count)) | Out-Null
    $lines.Add(('- missing context files: `{0}`' -f $Missing.Count)) | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('## Prompt Scope') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add($Item.SectionText.TrimEnd()) | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('## First-Pass Prompt') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add(('Treat `{0}` as an open mathematical problem, not a code-review task. Stay inside the owned problem and respect the non-goals. Use the primary ownership files as the redesign surface and the context files as constraints only. Do not propose implementation yet.' -f $Item.Id)) | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('Use the following prompt scope block exactly:') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('```md') | Out-Null
    $lines.Add($Item.SectionText.TrimEnd()) | Out-Null
    $lines.Add('```') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('Required deliverables:') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('1. Formal problem statement') | Out-Null
    $lines.Add('2. Notation for the search space, state space, path functional, ranking functional, and admissible approximations') | Out-Null
    $lines.Add('3. Fixed invariants and interface constraints') | Out-Null
    $lines.Add('4. Strongest plausible positive result') | Out-Null
    $lines.Add('5. Strongest plausible impossibility result') | Out-Null
    $lines.Add('6. Candidate theorem or lemma list') | Out-Null
    $lines.Add('7. One explicit counterexample family') | Out-Null
    $lines.Add('8. One lower-bound, identifiability, or impossibility angle') | Out-Null
    $lines.Add('9. Unresolved questions that block implementation') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('## Second-Pass Prompt') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add(('Attack the first-pass answer for `{0}` as an adversarial mathematician. Try to break the leading formalization, proposed sufficient statistics, pruning rules, ranking claims, or approximation contracts with edge cases, dependence structure, pathologies, and finite-grid effects. Do not redesign out-of-scope surfaces.' -f $Item.Id)) | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('Use the following prompt scope block exactly:') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('```md') | Out-Null
    $lines.Add($Item.SectionText.TrimEnd()) | Out-Null
    $lines.Add('```') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('Required deliverables:') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('1. Explicit failure modes') | Out-Null
    $lines.Add('2. Adversarial constructions or separating examples') | Out-Null
    $lines.Add('3. Corrected theorem or lemma candidates') | Out-Null
    $lines.Add('4. Weakest assumptions under which the result survives') | Out-Null
    $lines.Add('5. Distinction between exact, approximate, and heuristic claims') | Out-Null
    $lines.Add('6. Revised time, memory, and retained-state costs') | Out-Null
    $lines.Add('7. Dependency burden on other PR prompts') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('Narrow to the strongest surviving repo-fit result.') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('## Third-Pass Prompt') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add(('Produce the strongest implementation-ready mathematical result for `{0}` that survives the second pass. Stay inside the owned problem. Treat listed dependencies as interfaces, not as permission to redesign adjacent prompts. Any non-exact claim must come with an explicit approximation, error, regret, rank-instability, or bias contract.' -f $Item.Id)) | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('Use the following prompt scope block exactly:') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('```md') | Out-Null
    $lines.Add($Item.SectionText.TrimEnd()) | Out-Null
    $lines.Add('```') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('Required deliverables:') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('1. Final formalization') | Out-Null
    $lines.Add('2. Final theorem or lemma list') | Out-Null
    $lines.Add('3. Proof sketch or proof plan') | Out-Null
    $lines.Add('4. Sufficient statistics, or a proof they do not exist under the stated constraints') | Out-Null
    $lines.Add('5. Exactness conditions and equivalence classes') | Out-Null
    $lines.Add('6. Error, regret, or bias bounds for every non-exact claim') | Out-Null
    $lines.Add('7. Time, memory, and retained-state complexity') | Out-Null
    $lines.Add('8. File and symbol impact map') | Out-Null
    $lines.Add('9. Verification obligations and falsifiers') | Out-Null
    $lines.Add('10. Dependencies that must be reconciled during consolidation') | Out-Null
    $lines.Add('11. Kill criteria') | Out-Null
    $lines.Add("") | Out-Null
    return $lines
}

function Build-OverlayPromptMarkdown {
    param(
        [pscustomobject]$Item,
        [System.Collections.Generic.List[string]]$Copied,
        [System.Collections.Generic.List[string]]$Missing
    )

    $lines = [System.Collections.Generic.List[string]]::new()
    $lines.Add("# $($Item.Id) $($Item.Title)") | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add(('- package type: `{0}`' -f $Item.Kind)) | Out-Null
    $lines.Add(('- copied context files: `{0}`' -f $Copied.Count)) | Out-Null
    $lines.Add(('- missing context files: `{0}`' -f $Missing.Count)) | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('## Overlay Scope') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add($Item.SectionText.TrimEnd()) | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('## First-Pass Overlay Prompt') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('Append this overlay to the end of the chosen core `PR-*` first-pass prompt. Do not change the core prompt''s ownership, non-goals, or file boundaries.') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('Use this overlay block exactly:') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('```md') | Out-Null
    $lines.Add($Item.SectionText.TrimEnd()) | Out-Null
    $lines.Add('```') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('Overlay theorem obligations: require at least one new lemma candidate, one new counterexample family, or one new bound that the core prompt would otherwise miss.') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('## Second-Pass Overlay Prompt') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('Append this overlay to the end of the chosen core `PR-*` second-pass prompt. Keep the core prompt''s ownership and comparison structure fixed.') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('Use this overlay block exactly:') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('```md') | Out-Null
    $lines.Add($Item.SectionText.TrimEnd()) | Out-Null
    $lines.Add('```') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('Overlay theorem obligations: require the response to show how the overlay changes exactness assumptions, bounds, or adversarial constructions without changing ownership.') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('## Third-Pass Overlay Prompt') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('Append this overlay to the end of the chosen core `PR-*` third-pass prompt. Keep implementation ownership inside the core prompt''s boundary.') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('Use this overlay block exactly:') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('```md') | Out-Null
    $lines.Add($Item.SectionText.TrimEnd()) | Out-Null
    $lines.Add('```') | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add('Overlay theorem obligations: require implementation-facing implications of the new lemma, counterexample, or bound without redesigning adjacent PR ownership surfaces.') | Out-Null
    $lines.Add("") | Out-Null
    return $lines
}

if (Test-Path -LiteralPath $outputRootPath) {
    Remove-Item -LiteralPath $outputRootPath -Recurse -Force
}
New-Item -ItemType Directory -Path $outputRootPath | Out-Null

$raw = Get-Content -LiteralPath $promptDocPath -Raw

$items = [System.Collections.Generic.List[object]]::new()

$prMatches = [regex]::Matches(
    $raw,
    '(?ms)^## `(?<id>PR-\d+)` (?<title>[^\r\n]+)\r?\n(?<body>.*?)(?=^## `PR-\d+` |^## Consolidation Protocol|\z)'
)
foreach ($match in $prMatches) {
    $sectionText = $match.Value.Trim() + [Environment]::NewLine
    $paths = Get-ContextPathsFromSection -SectionText $sectionText
    $items.Add([pscustomobject]@{
            Id          = $match.Groups['id'].Value
            Title       = $match.Groups['title'].Value.Trim()
            SectionText = $sectionText
            Paths       = $paths
            Kind        = "core"
        }) | Out-Null
}

$oxMatches = [regex]::Matches(
    $raw,
    '(?ms)^### `(?<id>OX-\d+)` (?<title>[^\r\n]+)\r?\n(?<body>.*?)(?=^### `OX-\d+` |\z)'
)
foreach ($match in $oxMatches) {
    $sectionText = $match.Value.Trim() + [Environment]::NewLine
    $paths = Get-ContextPathsFromSection -SectionText $sectionText
    $items.Add([pscustomobject]@{
            Id          = $match.Groups['id'].Value
            Title       = $match.Groups['title'].Value.Trim()
            SectionText = $sectionText
            Paths       = $paths
            Kind        = "overlay"
        }) | Out-Null
}

$items = $items | Sort-Object Id

$indexLines = [System.Collections.Generic.List[string]]::new()
$coreCount = ($items | Where-Object Kind -eq 'core').Count
$overlayCount = ($items | Where-Object Kind -eq 'overlay').Count
$totalCount = $items.Count
$indexLines.Add("# Prompt Context Packages") | Out-Null
$indexLines.Add("") | Out-Null
$indexLines.Add('Generated from `[MATH_REVIEW_PROMPT_PACKETS.md](../MATH_REVIEW_PROMPT_PACKETS.md)`.' ) | Out-Null
$indexLines.Add("") | Out-Null
$indexLines.Add(('- core prompt packages: `{0}`' -f $coreCount)) | Out-Null
$indexLines.Add(('- overlay prompt packages: `{0}`' -f $overlayCount)) | Out-Null
$indexLines.Add(('- total packages: `{0}`' -f $totalCount)) | Out-Null
$indexLines.Add("") | Out-Null

foreach ($item in $items) {
    $folderName = Get-SafeFolderName -Id $item.Id -Title $item.Title
    $itemRoot = Join-Path $outputRootPath $folderName
    New-Item -ItemType Directory -Path $itemRoot -Force | Out-Null

    $copied = [System.Collections.Generic.List[string]]::new()
    $missing = [System.Collections.Generic.List[string]]::new()

    foreach ($relativePath in $item.Paths) {
        $sourcePath = [System.IO.Path]::GetFullPath((Join-Path $repoRootPath $relativePath))
        if (-not $sourcePath.StartsWith($repoRootPath, [System.StringComparison]::OrdinalIgnoreCase)) {
            $missing.Add($relativePath) | Out-Null
            continue
        }

        if (-not (Test-Path -LiteralPath $sourcePath)) {
            $missing.Add($relativePath) | Out-Null
            continue
        }

        $destinationPath = Join-Path $itemRoot $relativePath
        $destinationDir = Split-Path -Parent $destinationPath
        New-Item -ItemType Directory -Path $destinationDir -Force | Out-Null
        Copy-Item -LiteralPath $sourcePath -Destination $destinationPath -Force
        $copied.Add($relativePath) | Out-Null
    }

    $promptLines = [System.Collections.Generic.List[string]]::new()
    if ($item.Kind -eq "core") {
        $generatedLines = Build-CorePromptMarkdown -Item $item -Copied $copied -Missing $missing
    }
    else {
        $generatedLines = Build-OverlayPromptMarkdown -Item $item -Copied $copied -Missing $missing
    }
    foreach ($line in $generatedLines) {
        $promptLines.Add([string]$line) | Out-Null
    }

    $promptLines.Add("## Local Repo Copies") | Out-Null
    $promptLines.Add("") | Out-Null
    $promptLines.Add("The copied files below are stored directly under this prompt folder, preserving repo-relative paths.") | Out-Null
    $promptLines.Add("") | Out-Null
    if ($copied.Count -eq 0) {
        $promptLines.Add("- none copied") | Out-Null
    }
    else {
        foreach ($path in ($copied | Sort-Object)) {
            $promptLines.Add(('- `{0}`' -f $path)) | Out-Null
        }
    }
    if ($missing.Count -gt 0) {
        $promptLines.Add("") | Out-Null
        $promptLines.Add("## Missing Context Files") | Out-Null
        $promptLines.Add("") | Out-Null
        foreach ($path in ($missing | Sort-Object)) {
            $promptLines.Add(('- `{0}`' -f $path)) | Out-Null
        }
    }

    Set-Content -LiteralPath (Join-Path $itemRoot "PROMPT.md") -Value $promptLines -Encoding utf8

    $indexLines.Add(('- [{0} {1}](./{2}/PROMPT.md) - `{3}` copied context files' -f $item.Id, $item.Title, $folderName, $copied.Count)) | Out-Null
}

Set-Content -LiteralPath (Join-Path $outputRootPath "README.md") -Value $indexLines -Encoding utf8

Write-Output "Generated prompt context packages at: $(Get-RelativeDisplayPath -FullPath $outputRootPath)"
Write-Output "Package count: $(($items).Count)"
