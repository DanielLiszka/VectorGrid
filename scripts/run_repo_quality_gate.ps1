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


      Invoke-Step "Rust workspace tests" { cargo test --workspace -j 1 }
    }
  }

  if (-not $SkipWeb) {
    Invoke-Step "Web lint" { & npm.cmd --prefix web run lint }
    Invoke-Step "Web typecheck" { & npm.cmd --prefix web run typecheck }

    if ($IncludeDesktopTypecheck) {
      Write-Host "[SKIP] Web desktop typecheck is local-only in the backend-preview public tree."
    }

    if ($IncludeA11ySmoke) {
      Write-Host "[SKIP] Web accessibility smoke is local-only in the backend-preview public tree."
    }

    if ($IncludeBaselineUi) {
      Write-Host "[SKIP] Web baseline UI capture is local-only in the backend-preview public tree."
    }
  }

  Write-Host "Repo quality gate completed successfully."
  Write-Host "Default scope: rust fmt/check/clippy + web lint/typecheck."
  Write-Host "Optional scope: -IncludeWorkspaceTests. Desktop/a11y/baseline UI switches are accepted but skipped in this backend-preview tree."
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
