param(
  [switch]$SkipBuild,
  [switch]$IncludeFullWorkflow,
  [switch]$IncludeResponsive,
  [switch]$IncludeRegressions,
  [switch]$IncludeA11ySmoke,
  [switch]$IncludeWcag22Smoke,
  [switch]$IncludeBaselineUi,
  [switch]$IncludeVisualBaselines,
  [switch]$IncludeQaStatic,
  [switch]$IncludeQaReport,
  [switch]$IncludeUiPerfBudget,
  [switch]$IncludeExploratory
)

$ErrorActionPreference = "Stop"

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

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
Push-Location $repoRoot
try {
  if (-not $SkipBuild) {
    Invoke-Step "Web build" { & npm.cmd --prefix web run build }
  }

  if ($IncludeQaStatic) {
    Invoke-Step "Static QA manifest and fixture checks" { & npm.cmd --prefix web run check:qa-static }
  }

  if ($IncludeFullWorkflow) {
    Invoke-Step "Full browser workflow E2E" { & npm.cmd --prefix web run test:e2e:workflow:full:built }
  } else {
    Invoke-Step "Workbench browser E2E" { & npm.cmd --prefix web run test:e2e:workbench:built }
    Invoke-Step "Browser action coverage" { & npm.cmd --prefix web run test:e2e:action-coverage:built }
    Invoke-Step "Core browser workflow E2E" { & npm.cmd --prefix web run test:e2e:workflow:core:built }

    if ($IncludeResponsive) {
      Invoke-Step "Responsive browser E2E" { & npm.cmd --prefix web run test:e2e:responsive:built }
    }

    if ($IncludeRegressions) {
      Invoke-Step "Regression browser E2E" { & npm.cmd --prefix web run test:e2e:regressions:built }
    }

    if ($IncludeExploratory) {
      Invoke-Step "Exploratory browser E2E" { & npm.cmd --prefix web run test:e2e:explore:built }
    }
  }

  if ($IncludeA11ySmoke) {
    Invoke-Step "Web accessibility smoke" { & npm.cmd --prefix web run a11y:smoke:built }
  }

  if ($IncludeWcag22Smoke) {
    Invoke-Step "Web accessibility WCAG 2.2 smoke" { & npm.cmd --prefix web run a11y:smoke:wcag:built }
  }

  if ($IncludeBaselineUi) {
    Invoke-Step "Baseline UI capture" { & npm.cmd --prefix web run baseline:ui:built }
  }

  if ($IncludeVisualBaselines) {
    Invoke-Step "Visual baseline E2E" { & npm.cmd --prefix web run test:e2e:visual:built }
  }

  if ($IncludeUiPerfBudget) {
    Invoke-Step "UI performance budget" { & npm.cmd --prefix web run perf:ui:budget }
  }

  if ($IncludeQaReport) {
    Invoke-Step "Local QA summary report" { & npm.cmd --prefix web run qa:summary }
  }

  Write-Host "Frontend E2E gate completed successfully."
} finally {
  Pop-Location
}
