param(
  [switch]$SkipRust,
  [switch]$SkipWeb,
  [switch]$SkipCommandContract
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

$credentialEnvNames = @(
  "ALPACA_API_KEY",
  "ALPACA_API_KEY_ID",
  "ALPACA_API_SECRET",
  "ALPACA_API_SECRET_KEY",
  "ALPACA_KEY",
  "ALPACA_KEY_ID",
  "ALPACA_SECRET",
  "ALPACA_SECRET_KEY",
  "ALPACA_API_BASE_URL",
  "APCA_API_KEY",
  "APCA_API_KEY_ID",
  "APCA_API_SECRET",
  "APCA_API_SECRET_KEY",
  "APCA_API_BASE_URL",
  "POLYGON_KEY",
  "POLYGON_API_KEY",
  "POLYGON_TOKEN",
  "POLYGON_API_BASE_URL"
)

$previousEnv = @{}
foreach ($name in $credentialEnvNames) {
  $previousEnv[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}

$previousRustcWrapper = $env:RUSTC_WRAPPER
$previousCargoBuildRustcWrapper = $env:CARGO_BUILD_RUSTC_WRAPPER
$previousSccacheNoDaemon = $env:SCCACHE_NO_DAEMON

try {
  Push-Location $repoRoot

  foreach ($name in $credentialEnvNames) {
    Remove-Item "Env:$name" -ErrorAction SilentlyContinue
  }

  $env:RUSTC_WRAPPER = ""
  $env:CARGO_BUILD_RUSTC_WRAPPER = ""
  $env:SCCACHE_NO_DAEMON = "1"

  Write-Host "Execution safety gate: targeted unit/contract coverage only; no benchmarks or live credential use."

  if (-not $SkipRust) {
    Invoke-Step "Rust paper broker unit tests" {
      cargo test -p ta_desktop_demo_app --lib broker_paper::tests:: -- --test-threads=1
    }
    Invoke-Step "Rust live stream state tests" {
      cargo test -p ta_desktop_demo_app --lib state_live_stream_tests -- --test-threads=1
    }
    Invoke-Step "Rust execution mock harness tests" {
      cargo test -p ta_desktop_demo_app --lib execution_mocks::tests:: -- --test-threads=1
    }
    Invoke-Step "Rust execution risk policy settings tests" {
      cargo test -p ta_desktop_demo_app --lib execution_risk_policy -- --test-threads=1
    }
    Invoke-Step "Rust execution command safety tests" {
      cargo test -p ta_desktop_demo_app --lib commands::commands_execution::tests:: -- --test-threads=1
    }
  }

  if (-not $SkipWeb) {
    Invoke-Step "Frontend execution tests" {
      & npm.cmd --prefix web run test -- `
        src/services/executionService.test.ts `
        src/services/executionChartService.test.ts `
        src/stores/executionStore.actions.test.ts `
        src/components/ExecutionWorkspacePanel.test.ts `
        src/components/ExecutionWorkspacePanel.virtualization.test.ts `
        src/utils/executionRisk.test.ts
    }

    if (-not $SkipCommandContract) {
      Invoke-Step "Frontend Tauri command contract check" {
        & npm.cmd --prefix web run check:commands
      }

      Invoke-Step "Frontend Tauri command risk policy check" {
        & npm.cmd --prefix web run check:tauri-command-risk-policy
      }
    }
  }

  Write-Host "Execution safety gate completed successfully."
} finally {
  Pop-Location -ErrorAction SilentlyContinue

  foreach ($name in $credentialEnvNames) {
    $value = $previousEnv[$name]
    if ($null -ne $value) {
      [Environment]::SetEnvironmentVariable($name, $value, "Process")
    } else {
      Remove-Item "Env:$name" -ErrorAction SilentlyContinue
    }
  }

  if ($null -ne $previousRustcWrapper) {
    $env:RUSTC_WRAPPER = $previousRustcWrapper
  } else {
    Remove-Item Env:RUSTC_WRAPPER -ErrorAction SilentlyContinue
  }

  if ($null -ne $previousCargoBuildRustcWrapper) {
    $env:CARGO_BUILD_RUSTC_WRAPPER = $previousCargoBuildRustcWrapper
  } else {
    Remove-Item Env:CARGO_BUILD_RUSTC_WRAPPER -ErrorAction SilentlyContinue
  }

  if ($null -ne $previousSccacheNoDaemon) {
    $env:SCCACHE_NO_DAEMON = $previousSccacheNoDaemon
  } else {
    Remove-Item Env:SCCACHE_NO_DAEMON -ErrorAction SilentlyContinue
  }
}
