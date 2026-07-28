$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$runStartedAtUtc = [DateTime]::UtcNow
$isGitHubActions = [string]$env:GITHUB_ACTIONS -eq "true"
$runAttempt = if ([string]::IsNullOrWhiteSpace($env:GITHUB_RUN_ATTEMPT)) { "1" } else { $env:GITHUB_RUN_ATTEMPT }
$evidenceRunId = if ($isGitHubActions -and -not [string]::IsNullOrWhiteSpace($env:GITHUB_RUN_ID)) {
  "github-$($env:GITHUB_RUN_ID)-$runAttempt"
} else {
  "local-$($runStartedAtUtc.ToString('yyyyMMddTHHmmssZ'))-$([guid]::NewGuid().ToString('N'))"
}
if ($evidenceRunId -notmatch '^[a-zA-Z0-9][a-zA-Z0-9._-]{0,119}$') {
  throw "The derived CPU CI evidence run id is unsafe."
}
$currentEvidenceRoot = [System.IO.Path]::GetFullPath(
  (Join-Path $repoRoot "output\qa\required-cpu-quality\$evidenceRunId")
)
$currentEvidenceParent = [System.IO.Path]::GetFullPath(
  (Join-Path $repoRoot "output\qa\required-cpu-quality")
)
$currentEvidencePrefix = $currentEvidenceParent.TrimEnd([char]92, [char]47) + [System.IO.Path]::DirectorySeparatorChar
if (-not $currentEvidenceRoot.StartsWith($currentEvidencePrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw "The current-run evidence directory escaped its fixed parent."
}
if (Test-Path -LiteralPath $currentEvidenceRoot) {
  throw "The current-run evidence directory already exists: $currentEvidenceRoot"
}
New-Item -ItemType Directory -Force -Path $currentEvidenceRoot | Out-Null

function Write-Utf8NoBom {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$Value
  )
  [System.IO.File]::WriteAllText(
    $Path,
    $Value,
    (New-Object System.Text.UTF8Encoding($false))
  )
}

function Copy-CurrentRunReport {
  param(
    [Parameter(Mandatory = $true)][string]$Source,
    [Parameter(Mandatory = $true)][string]$Destination,
    [Parameter(Mandatory = $true)][DateTime]$RunStartedAtUtc,
    [switch]$ValidateGeneratedAt
  )

  if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) {
    throw "Expected current-run report is missing: $Source"
  }
  $sourceItem = Get-Item -LiteralPath $Source -Force
  if (($sourceItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "Current-run reports may not be copied through reparse points."
  }
  if ($sourceItem.LastWriteTimeUtc -lt $RunStartedAtUtc.AddSeconds(-2)) {
    throw "Refusing stale report from before this CPU CI run: $Source"
  }
  if ($ValidateGeneratedAt) {
    $report = Get-Content -Raw -LiteralPath $Source | ConvertFrom-Json -ErrorAction Stop
    $generatedAtProperty = $report.PSObject.Properties["generatedAt"]
    $generatedAtUtcProperty = $report.PSObject.Properties["generated_at_utc"]
    $generatedAtText = if ($null -ne $generatedAtProperty) {
      [string]$generatedAtProperty.Value
    } elseif ($null -ne $generatedAtUtcProperty) {
      [string]$generatedAtUtcProperty.Value
    } else {
      ""
    }
    $generatedAt = [DateTime]::MinValue
    if (
      [string]::IsNullOrWhiteSpace($generatedAtText) -or
      -not [DateTime]::TryParse(
        $generatedAtText,
        [System.Globalization.CultureInfo]::InvariantCulture,
        [System.Globalization.DateTimeStyles]::AdjustToUniversal,
        [ref]$generatedAt
      ) -or
      $generatedAt.ToUniversalTime() -lt $RunStartedAtUtc.AddSeconds(-2)
    ) {
      throw "Refusing report whose generated timestamp does not belong to this CPU CI run: $Source"
    }
  }
  Copy-Item -LiteralPath $Source -Destination $Destination
}

function Invoke-Step {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Name,
    [Parameter(Mandatory = $true)]
    [scriptblock]$Action
  )

  Write-Host "==> $Name"
  & $Action
  if ($LASTEXITCODE -ne 0) {
    throw "$Name failed with exit code $LASTEXITCODE"
  }
  Write-Host "[PASS] $Name"
}

$credentialEnvNames = @(
  "ALPACA_API_KEY",
  "ALPACA_API_SECRET",
  "ALPACA_KEY_ID",
  "ALPACA_SECRET_KEY",
  "APCA_API_KEY_ID",
  "APCA_API_SECRET_KEY",
  "POLYGON_KEY",
  "POLYGON_API_KEY"
)

$savedEnvironment = @{}
$environmentNames = @(
  "CARGO_BUILD_JOBS",
  "CARGO_INCREMENTAL",
  "CI",
  "RUST_TEST_THREADS",
  "RUSTC_WRAPPER",
  "CARGO_BUILD_RUSTC_WRAPPER",
  "SCCACHE_NO_DAEMON"
) + $credentialEnvNames

foreach ($name in $environmentNames) {
  $savedEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}

$locationPushed = $false
try {
  Push-Location $repoRoot
  $locationPushed = $true

  # Keep the shared target directory deterministic and avoid concurrent rustc work
  # within this required CPU lane.
  $env:CARGO_BUILD_JOBS = "1"
  $env:CARGO_INCREMENTAL = "0"
  $env:CI = "true"
  $env:RUST_TEST_THREADS = "1"
  $env:RUSTC_WRAPPER = ""
  $env:CARGO_BUILD_RUSTC_WRAPPER = ""
  $env:SCCACHE_NO_DAEMON = "1"

  # This lane is intentionally credential-free. Provider and broker integration
  # require their separate sandbox/manual evidence.
  foreach ($name in $credentialEnvNames) {
    Remove-Item "Env:$name" -ErrorAction SilentlyContinue
  }

  Invoke-Step "Cargo lockfile contract" {
    cargo metadata --locked --no-deps --format-version 1 | Out-Null
  }

  Invoke-Step "Strict Clippy: ta_strategies" {
    cargo clippy --locked -p ta_strategies --all-targets -- -D warnings
  }

  Invoke-Step "Strict Clippy: ta_optimizer" {
    cargo clippy --locked -p ta_optimizer --all-targets -- -D warnings
  }

  Invoke-Step "Node/npm reproducibility contract" {
    node (Join-Path $repoRoot "scripts\check_node_reproducibility.mjs") `
      --out (Join-Path $currentEvidenceRoot "node-reproducibility.json")
  }

  Invoke-Step "Release secret hygiene" {
    node (Join-Path $repoRoot "scripts\check_release_secret_hygiene.mjs") `
      --out (Join-Path $currentEvidenceRoot "release-secret-hygiene.json")
  }

  Invoke-Step "Provider sandbox offline contracts" {
    node --test (Join-Path $repoRoot "scripts\provider_sandbox_preflight.test.mjs")
  }

  Invoke-Step "Canonical CPU repo quality and workspace tests" {
    powershell -NoProfile -ExecutionPolicy Bypass `
      -File (Join-Path $repoRoot "scripts\run_repo_quality_gate.ps1") `
      -IncludeDesktopTypecheck `
      -IncludeWorkspaceTests
  }

  Invoke-Step "Desktop boundary contracts" {
    & npm.cmd --prefix web run security:desktop-boundary
  }

  Invoke-Step "Browser harness typecheck" {
    & npm.cmd --prefix web run typecheck:e2e
  }

  Invoke-Step "Frontend unit and component tests (serialized)" {
    & npm.cmd --prefix web run test -- --maxWorkers=1 --fileParallelism=false
  }

  Invoke-Step "Full headless Chromium QA" {
    powershell -NoProfile -ExecutionPolicy Bypass `
      -File (Join-Path $repoRoot "scripts\run_frontend_e2e_gate.ps1") `
      -IncludeFullWorkflow `
      -IncludeWcag22Smoke `
      -IncludeQaReport
  }

  Invoke-Step "Headless JSON/CSV/XLSX output parity" {
    node (Join-Path $repoRoot "scripts\run_output_parity_check.mjs")
  }

  Invoke-Step "Stage explicit current-run QA reports" {
    $dashboardSource = Join-Path $repoRoot "output\qa\frontend-qa-dashboard.json"
    Copy-CurrentRunReport `
      -Source $dashboardSource `
      -Destination (Join-Path $currentEvidenceRoot "frontend-qa-dashboard.json") `
      -RunStartedAtUtc $runStartedAtUtc `
      -ValidateGeneratedAt
    Copy-CurrentRunReport `
      -Source (Join-Path $repoRoot "output\qa\frontend-qa-dashboard.md") `
      -Destination (Join-Path $currentEvidenceRoot "frontend-qa-dashboard.md") `
      -RunStartedAtUtc $runStartedAtUtc
    Copy-CurrentRunReport `
      -Source (Join-Path $repoRoot "output\qa\manifest-toolchain-contract.json") `
      -Destination (Join-Path $currentEvidenceRoot "manifest-toolchain-contract.json") `
      -RunStartedAtUtc $runStartedAtUtc `
      -ValidateGeneratedAt
    Copy-CurrentRunReport `
      -Source (Join-Path $repoRoot "output\qa\release-authority-matrix-contract.json") `
      -Destination (Join-Path $currentEvidenceRoot "release-authority-matrix-contract.json") `
      -RunStartedAtUtc $runStartedAtUtc
    Copy-CurrentRunReport `
      -Source (Join-Path $repoRoot "_write_test\headless_reports\vectorgrid_synthetic_headless_current.parity_manifest.json") `
      -Destination (Join-Path $currentEvidenceRoot "headless-output-parity-manifest.json") `
      -RunStartedAtUtc $runStartedAtUtc `
      -ValidateGeneratedAt

    $knownReportNames = @(
      "frontend-qa-dashboard.json",
      "frontend-qa-dashboard.md",
      "headless-output-parity-manifest.json",
      "manifest-toolchain-contract.json",
      "node-reproducibility.json",
      "release-authority-matrix-contract.json",
      "release-secret-hygiene.json"
    )
    $evidenceFiles = @(
      foreach ($name in $knownReportNames) {
        $path = Join-Path $currentEvidenceRoot $name
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
          throw "Required current-run evidence file is missing: $name"
        }
        $item = Get-Item -LiteralPath $path
        [ordered]@{
          name = $name
          size_bytes = [int64]$item.Length
          sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
        }
      }
    )
    $summary = [ordered]@{
      schema_version = 1
      report_kind = "required-cpu-quality"
      run_id = $evidenceRunId
      generated_at_utc = [DateTime]::UtcNow.ToString("o")
      started_at_utc = $runStartedAtUtc.ToString("o")
      status = "passed"
      caller_context = if ($isGitHubActions) { "github-actions-required-cpu-quality" } else { "local-required-cpu-quality" }
      caller_automatic_upload = $isGitHubActions
      checks = @(
        "cargo-lockfile",
        "strict-clippy-ta-strategies",
        "strict-clippy-ta-optimizer",
        "node-npm-reproducibility",
        "release-secret-hygiene",
        "provider-sandbox-offline-contracts",
        "canonical-cpu-quality-and-workspace-tests",
        "release-authority-matrix-contract",
        "desktop-boundary",
        "browser-harness-typecheck",
        "frontend-tests-serialized",
        "full-headless-chromium-qa",
        "headless-output-parity"
      )
      evidence_files = $evidenceFiles
      excluded = @(
        "cuda-gpu",
        "performance-profiling",
        "native-computer-use",
        "packaging-signing",
        "provider-credentials",
        "clean-machine-install"
      )
    }
    Write-Utf8NoBom `
      -Path (Join-Path $currentEvidenceRoot "required-cpu-quality-summary.json") `
      -Value (($summary | ConvertTo-Json -Depth 20) + [Environment]::NewLine)
  }

  $evidenceOutDir = Join-Path $repoRoot "output\support-evidence\ci\$evidenceRunId"
  Invoke-Step "Build redacted CI support evidence" {
    if ($isGitHubActions) {
      powershell -NoProfile -ExecutionPolicy Bypass `
        -File (Join-Path $repoRoot "scripts\build_support_evidence_bundle.ps1") `
        -EvidenceRoot $currentEvidenceRoot `
        -OutDir $evidenceOutDir `
        -CallerContext "github-actions-required-cpu-quality" `
        -AutomaticUpload
    } else {
      powershell -NoProfile -ExecutionPolicy Bypass `
        -File (Join-Path $repoRoot "scripts\build_support_evidence_bundle.ps1") `
        -EvidenceRoot $currentEvidenceRoot `
        -OutDir $evidenceOutDir `
        -CallerContext "local-required-cpu-quality"
    }
  }

  Write-Host "Required CPU CI gate completed successfully."
  Write-Host "Excluded by design: CUDA/GPU, benchmarks/profiling, native WebDriver/Computer Use, packaging/signing, provider credentials, and clean-machine install testing."
} finally {
  if ($locationPushed) {
    Pop-Location -ErrorAction SilentlyContinue
  }

  foreach ($name in $environmentNames) {
    $value = $savedEnvironment[$name]
    if ($null -ne $value) {
      [Environment]::SetEnvironmentVariable($name, $value, "Process")
    } else {
      Remove-Item "Env:$name" -ErrorAction SilentlyContinue
    }
  }
}
