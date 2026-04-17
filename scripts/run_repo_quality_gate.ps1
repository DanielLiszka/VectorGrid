param(
  [switch]$SkipRust,
  [switch]$SkipWeb,
  [switch]$SkipClippy,
  [switch]$StrictClippy,
  [switch]$IncludeDesktopTypecheck,
  [switch]$IncludeWorkspaceTests,
  [switch]$IncludeA11ySmoke,
  [switch]$IncludeBaselineUi
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

$previousRustcWrapper = $env:RUSTC_WRAPPER
$previousCargoBuildRustcWrapper = $env:CARGO_BUILD_RUSTC_WRAPPER
$previousSccacheNoDaemon = $env:SCCACHE_NO_DAEMON

try {
  Write-Host "Temporarily clearing Rust compiler wrappers for the repo quality gate to avoid sccache daemon startup failures."
  $env:RUSTC_WRAPPER = ""
  $env:CARGO_BUILD_RUSTC_WRAPPER = ""
  $env:SCCACHE_NO_DAEMON = "1"

  if (-not $SkipRust) {
    Invoke-Step "Rust format" { cargo fmt --all -- --check }
    Invoke-Step "Rust check" { cargo check --workspace }

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
    }

    if ($IncludeWorkspaceTests) {
      # Keep the optional full-workspace test leg deterministic on Windows.
      # The Tauri/bin/example surface is large enough that default parallelism
      # can trip linker/pagefile instability even when the serial workspace suite is green.
      Invoke-Step "Rust workspace tests" { cargo test --workspace -j 1 }
    }
  }

  if (-not $SkipWeb) {
    Invoke-Step "Web lint" { & npm.cmd --prefix web run lint }
    Invoke-Step "Web typecheck" { & npm.cmd --prefix web run typecheck }

    if ($IncludeDesktopTypecheck) {
      Invoke-Step "Web desktop typecheck" { & npm.cmd --prefix web run typecheck:desktop }
    }

    if ($IncludeA11ySmoke) {
      Invoke-Step "Web accessibility smoke" { & npm.cmd --prefix web run a11y:smoke:managed }
    }

    if ($IncludeBaselineUi) {
      Invoke-Step "Web baseline UI capture" { & npm.cmd --prefix web run baseline:ui:managed }
    }
  }

  Write-Host "Repo quality gate completed successfully."
  Write-Host "Default scope: rust fmt/check/clippy + web lint/typecheck."
  Write-Host "Optional scope: -IncludeDesktopTypecheck, -IncludeWorkspaceTests, -IncludeA11ySmoke, and -IncludeBaselineUi."
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
