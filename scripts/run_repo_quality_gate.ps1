param(
  [switch]$SkipRust,
  [switch]$SkipWeb,
  [switch]$SkipClippy,
  [switch]$StrictClippy,
  [switch]$IncludeDesktopTypecheck,
  [switch]$IncludeWorkspaceTests,
  [switch]$IncludeBrowserWorkflowQa,
  [switch]$IncludeFullBrowserWorkflowQa,
  [switch]$IncludeBrowserResponsiveE2E,
  [switch]$IncludeBrowserRegressionE2E,
  [switch]$IncludeBrowserExploratoryE2E,
  [switch]$IncludeDesktopSmoke,
  [switch]$IncludeA11ySmoke,
  [switch]$IncludeBaselineUi
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

$previousRustcWrapper = $env:RUSTC_WRAPPER
$previousCargoBuildRustcWrapper = $env:CARGO_BUILD_RUSTC_WRAPPER
$previousSccacheNoDaemon = $env:SCCACHE_NO_DAEMON

try {
  Invoke-Step "Release authority matrix contract" {
    node (Join-Path $repoRoot "scripts\\check_release_authority_matrix.mjs") `
      --out (Join-Path $repoRoot "output\\qa\\release-authority-matrix-contract.json")
  }

  Invoke-Step "Canonical benchmark-lock lease contract" {
    node --test (Join-Path $repoRoot "scripts\\benchmark_lock_contract.test.mjs")
  }

  Invoke-Step "CUDA Graph timing boundary contract" {
    node --test (Join-Path $repoRoot "scripts\\cuda_graph_timing_contract.test.mjs")
  }

  Invoke-Step "CUDA Graph multi-tile parity contract" {
    node --test (Join-Path $repoRoot "scripts\\cuda_graph_multitile_parity_contract.test.mjs")
  }

  Invoke-Step "Performance audit preflight contract" {
    node --test `
      (Join-Path $repoRoot "scripts\\performance_audit_preflight_contract.test.mjs") `
      (Join-Path $repoRoot "scripts\\performance_child_process_contract.test.mjs")
  }

  Invoke-Step "CPU reliability soak authority contract" {
    node --test (Join-Path $repoRoot "scripts\\cpu_reliability_soak_contract.test.mjs")
  }

  Invoke-Step "Frontend interaction latency contract" {
    node --test `
      (Join-Path $repoRoot "web\\scripts\\frontend-interaction-authority-lease.test.mjs") `
      (Join-Path $repoRoot "web\\scripts\\frontend-interaction-build-attestation.test.mjs") `
      (Join-Path $repoRoot "web\\scripts\\frontend-interaction-latency-lib.test.mjs") `
      (Join-Path $repoRoot "web\\scripts\\frontend-interaction-static-snapshot.test.mjs") `
      (Join-Path $repoRoot "web\\scripts\\frontend-interaction-wrapper-contract.test.mjs") `
      (Join-Path $repoRoot "web\\scripts\\support\\mockedTauriHarness.test.mjs")
  }

  Invoke-Step "Live execution safety and public authority contracts" {
    node --test `
      (Join-Path $repoRoot "scripts\\live_execution_safety_harness_contract.test.mjs") `
      (Join-Path $repoRoot "scripts\\public_live_safety_evidence_contract.test.mjs") `
      (Join-Path $repoRoot "scripts\\public_live_release_wiring_contract.test.mjs")
  }

  Invoke-Step "Release-proof evidence and packaged-gate contracts" {
    node --test `
      (Join-Path $repoRoot "scripts\\release_proof_evidence_contract.test.mjs") `
      (Join-Path $repoRoot "scripts\\release_proof_packaged_gate_contract.test.mjs")
  }

  Invoke-Step "Support-evidence privacy canaries" {
    powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $repoRoot "scripts\\test_support_evidence_bundle.ps1")
  }

  Write-Host "Temporarily clearing Rust compiler wrappers for the repo quality gate to avoid sccache daemon startup failures."
  $env:RUSTC_WRAPPER = ""
  $env:CARGO_BUILD_RUSTC_WRAPPER = ""
  $env:SCCACHE_NO_DAEMON = "1"

  if (-not $SkipRust) {
    Invoke-Step "Rust format" { cargo fmt --all -- --check }
    Invoke-Step "Rust check" { cargo check --workspace }
    Invoke-Step "Rust maintainer target check" {
      cargo check -p ta_desktop_demo_app --no-default-features --features maintainer-tools --bins
    }

    if (-not $SkipClippy) {
      $clippyArgs = @("clippy", "--workspace", "--all-targets", "--no-deps")
      if ($StrictClippy) {
        $clippyArgs += @("--", "-D", "warnings")
      }
      Write-Host "==> Rust clippy"
      cargo @clippyArgs
      $clippyExit = $LASTEXITCODE
      if ($clippyExit -ne 0) {
        if ($StrictClippy) {
          throw "Rust clippy failed with exit code $clippyExit"
        }
        Write-Host "[WARN] Rust clippy reported actionable warnings; advisory only in default mode."
      } else {
        Write-Host "[PASS] Rust clippy"
      }

      $maintainerClippyArgs = @(
        "clippy",
        "-p", "ta_desktop_demo_app",
        "--no-default-features",
        "--features", "maintainer-tools",
        "--bins",
        "--no-deps"
      )
      if ($StrictClippy) {
        $maintainerClippyArgs += @("--", "-D", "warnings")
      }
      Write-Host "==> Rust maintainer target clippy"
      cargo @maintainerClippyArgs
      $maintainerClippyExit = $LASTEXITCODE
      if ($maintainerClippyExit -ne 0) {
        if ($StrictClippy) {
          throw "Rust maintainer target clippy failed with exit code $maintainerClippyExit"
        }
        Write-Host "[WARN] Rust maintainer target clippy reported actionable warnings; advisory only in default mode."
      } else {
        Write-Host "[PASS] Rust maintainer target clippy"
      }
    }

    if ($IncludeWorkspaceTests) {


      Invoke-Step "Rust workspace tests" { cargo test --workspace -j 1 }
    }
  }

  if (-not $SkipWeb) {
    Invoke-Step "Manifest/toolchain contract" { & npm.cmd --prefix web run check:manifest-toolchain }
    Invoke-Step "Desktop prerequisite contract" { & npm.cmd --prefix web run test:desktop:prereqs }
    Invoke-Step "Web lint" { & npm.cmd --prefix web run lint }
    Invoke-Step "Web typecheck" { & npm.cmd --prefix web run typecheck }

    if ($IncludeDesktopTypecheck) {
      Invoke-Step "Web desktop harness typecheck" { & npm.cmd --prefix web run typecheck:desktop }
    }

    if ($IncludeBrowserWorkflowQa -or $IncludeFullBrowserWorkflowQa -or $IncludeBrowserResponsiveE2E -or $IncludeBrowserRegressionE2E -or $IncludeBrowserExploratoryE2E) {
      $frontendQaArgs = @("-File", (Join-Path $repoRoot "scripts\\run_frontend_e2e_gate.ps1"))
      if ($IncludeFullBrowserWorkflowQa) {
        $frontendQaArgs += "-IncludeFullWorkflow"
      }
      if ($IncludeBrowserResponsiveE2E) {
        $frontendQaArgs += "-IncludeResponsive"
      }
      if ($IncludeBrowserRegressionE2E) {
        $frontendQaArgs += "-IncludeRegressions"
      }
      if ($IncludeA11ySmoke) {
        $frontendQaArgs += "-IncludeA11ySmoke"
      }
      if ($IncludeBaselineUi) {
        $frontendQaArgs += "-IncludeBaselineUi"
      }
      if ($IncludeBrowserExploratoryE2E) {
        $frontendQaArgs += "-IncludeExploratory"
      }
      Invoke-Step "Frontend browser E2E QA" { powershell @frontendQaArgs }
    } else {
      if ($IncludeA11ySmoke) {
        Invoke-Step "Web accessibility smoke" { & npm.cmd --prefix web run a11y:smoke }
      }

      if ($IncludeBaselineUi) {
        Invoke-Step "Web baseline UI capture" { & npm.cmd --prefix web run baseline:ui }
      }
    }

    if ($IncludeDesktopSmoke) {
      Invoke-Step "Desktop smoke" { & npm.cmd --prefix web run test:desktop:smoke }
    }
  }

  Write-Host "Repo quality gate completed successfully."
  Write-Host "Default scope: rust fmt/check/clippy + web lint/typecheck."
  Write-Host "Optional scope: -IncludeWorkspaceTests, -IncludeDesktopTypecheck, -IncludeBrowserWorkflowQa, -IncludeBrowserResponsiveE2E, -IncludeBrowserRegressionE2E, -IncludeA11ySmoke, -IncludeBaselineUi, -IncludeDesktopSmoke."
} finally {
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
