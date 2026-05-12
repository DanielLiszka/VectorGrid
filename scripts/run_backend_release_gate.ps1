param(
  [ValidateSet("cpu", "gpu-ma", "gpu-kernel")]
  [string]$Variant = "cpu",
  [switch]$SkipFormat,
  [switch]$SkipClippy,
  [switch]$StrictClippy,
  [switch]$SkipWorkspaceTests,
  [switch]$SkipReleaseBuilds,
  [switch]$IncludeBenchBuilds,
  [switch]$IncludeCpuPerf,
  [switch]$IncludeGpuPerf,
  [switch]$IncludeGpuValidationPerf
)

$ErrorActionPreference = "Stop"

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))

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

function Get-AppCoreFeatureList {
  $features = @("cli")
  if ($Variant -eq "gpu-ma") {
    $features += "cuda"
  } elseif ($Variant -eq "gpu-kernel") {
    $features += "cuda-backtest-kernel"
  }
  return ($features -join ",")
}

function Get-DesktopFeatureArgs {
  if ($Variant -eq "gpu-ma") {
    return @("--features", "cuda")
  }
  if ($Variant -eq "gpu-kernel") {
    return @("--features", "cuda-backtest-kernel")
  }
  return @()
}

function Get-StrictClippyArgs {
  if ($StrictClippy) {
    return @("--", "-D", "warnings")
  }
  return @()
}

$previousRustcWrapper = $env:RUSTC_WRAPPER
$previousCargoBuildRustcWrapper = $env:CARGO_BUILD_RUSTC_WRAPPER
$previousSccacheNoDaemon = $env:SCCACHE_NO_DAEMON

try {
  Push-Location $repoRoot

  if (($IncludeGpuPerf -or $IncludeGpuValidationPerf) -and $Variant -eq "cpu") {
    throw "GPU perf gates require -Variant gpu-ma or -Variant gpu-kernel."
  }
  if ($IncludeGpuValidationPerf -and $Variant -ne "gpu-kernel") {
    throw "-IncludeGpuValidationPerf requires -Variant gpu-kernel."
  }

  Write-Host "Backend release gate: Rust-only validation for the backend/CLI preview surface."
  Write-Host "Scope: backend crates, CLI targets, benchmark harnesses, and headless validation."
  Write-Host "This gate intentionally does not call npm, inspect web/, build web/dist, or run Tauri packaging."
  Write-Host "Variant: $Variant"

  $env:RUSTC_WRAPPER = ""
  $env:CARGO_BUILD_RUSTC_WRAPPER = ""
  $env:SCCACHE_NO_DAEMON = "1"

  if (-not $SkipFormat) {
    Invoke-Step "Rust format" { cargo fmt --all -- --check }
  }

  Invoke-Step "Backend CLI check" {
    cargo check -p ta_app_core --features (Get-AppCoreFeatureList)
  }

  Invoke-Step "Backend Rust checks" {
    cargo check -p ta_app_core --all-targets --features (Get-AppCoreFeatureList)
    if ($LASTEXITCODE -ne 0) { throw "ta_app_core check failed with exit code $LASTEXITCODE" }

    cargo check -p ta_optimizer --all-targets
    if ($LASTEXITCODE -ne 0) { throw "ta_optimizer check failed with exit code $LASTEXITCODE" }

    cargo check -p ta_strategies --all-targets
    if ($LASTEXITCODE -ne 0) { throw "ta_strategies check failed with exit code $LASTEXITCODE" }

    cargo check -p ta_desktop_demo_app --no-default-features --lib --tests --bin ta_headless_validate
    if ($LASTEXITCODE -ne 0) { throw "ta_desktop_demo_app backend check failed with exit code $LASTEXITCODE" }
  }

  if (-not $SkipClippy) {
    $clippyFailures = @()
    $clippySteps = @(
      @{
        Name = "ta_app_core"
        Args = @("clippy", "-p", "ta_app_core", "--all-targets", "--features", (Get-AppCoreFeatureList), "--no-deps")
      },
      @{
        Name = "ta_optimizer"
        Args = @("clippy", "-p", "ta_optimizer", "--all-targets", "--no-deps")
      },
      @{
        Name = "ta_strategies"
        Args = @("clippy", "-p", "ta_strategies", "--all-targets", "--no-deps")
      },
      @{
        Name = "ta_desktop_demo_app backend targets"
        Args = @("clippy", "-p", "ta_desktop_demo_app", "--no-default-features", "--lib", "--tests", "--bin", "ta_headless_validate", "--no-deps")
      }
    )
    foreach ($step in $clippySteps) {
      $args = $step.Args + (Get-StrictClippyArgs)
      Write-Host "==> Rust clippy: $($step.Name)"
      cargo @args
      $clippyExit = $LASTEXITCODE
      if ($clippyExit -ne 0) {
        if ($StrictClippy) {
          throw "Rust clippy failed for $($step.Name) with exit code $clippyExit"
        }
        $clippyFailures += "$($step.Name) exit=$clippyExit"
      } else {
        Write-Host "[PASS] Rust clippy: $($step.Name)"
      }
    }
    if ($clippyFailures.Count -gt 0) {
      Write-Host "[WARN] Rust clippy reported actionable warnings; advisory only unless -StrictClippy is set: $($clippyFailures -join '; ')"
    }
  }

  if (-not $SkipWorkspaceTests) {
    Invoke-Step "Backend Rust tests" {
      cargo test -p ta_app_core -j 1
      if ($LASTEXITCODE -ne 0) { throw "ta_app_core tests failed with exit code $LASTEXITCODE" }

      cargo test -p ta_optimizer -j 1
      if ($LASTEXITCODE -ne 0) { throw "ta_optimizer tests failed with exit code $LASTEXITCODE" }

      cargo test -p ta_strategies -j 1
      if ($LASTEXITCODE -ne 0) { throw "ta_strategies tests failed with exit code $LASTEXITCODE" }

      cargo test -p ta_desktop_demo_app --no-default-features --lib --tests -j 1 -- --test-threads=1
      if ($LASTEXITCODE -ne 0) { throw "ta_desktop_demo_app backend tests failed with exit code $LASTEXITCODE" }
    }
  }

  if (-not $SkipReleaseBuilds) {
    Invoke-Step "Backend CLI release build" {
      cargo build -p ta_app_core --release --features (Get-AppCoreFeatureList) --bin ta_cli
    }

    $desktopArgs = @("build", "-p", "ta_desktop_demo_app", "--release", "--no-default-features") + (Get-DesktopFeatureArgs) + @("--bin", "ta_headless_validate")
    Write-Host ("==> Headless validation CLI release build: cargo " + ($desktopArgs -join " "))
    cargo @desktopArgs
    if ($LASTEXITCODE -ne 0) {
      throw "Headless validation CLI release build failed with exit code $LASTEXITCODE"
    }
    Write-Host "[PASS] Headless validation CLI release build"
  }

  if ($IncludeBenchBuilds) {
    Invoke-Step "Backend benchmark build" {
      cargo bench -p ta_app_core --no-run
    }
  }

  if ($IncludeCpuPerf) {
    Invoke-Step "CPU perf gate" {
      powershell -ExecutionPolicy Bypass -File (Join-Path $repoRoot "scripts\\run_cpu_perf_gate.ps1") -Mode candidate
    }
  }

  if ($IncludeGpuPerf) {
    Invoke-Step "GPU perf gate" {
      powershell -ExecutionPolicy Bypass -File (Join-Path $repoRoot "scripts\\run_gpu_perf_gate.ps1") -Mode candidate
    }
  }

  if ($IncludeGpuValidationPerf) {
    Invoke-Step "GPU validation perf gate" {
      powershell -ExecutionPolicy Bypass -File (Join-Path $repoRoot "scripts\\run_gpu_validation_perf_gate.ps1") -Mode candidate -WorkloadProfile release-authority
    }
  }

  Write-Host "Backend release gate completed successfully."
} finally {
  Pop-Location -ErrorAction SilentlyContinue

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
