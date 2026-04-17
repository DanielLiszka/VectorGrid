param(
  [string[]]$ExtraCargoArgs = @()
)

$ErrorActionPreference = "Stop"

$previousRustcWrapper = $env:RUSTC_WRAPPER
$previousCargoBuildRustcWrapper = $env:CARGO_BUILD_RUSTC_WRAPPER
$previousSccacheNoDaemon = $env:SCCACHE_NO_DAEMON

try {
  Write-Host "Running stable workspace tests with serialized Cargo jobs for the Windows-heavy workspace surface."
  Write-Host "Temporarily clearing Rust compiler wrappers to avoid sccache daemon startup failures."

  $env:RUSTC_WRAPPER = ""
  $env:CARGO_BUILD_RUSTC_WRAPPER = ""
  $env:SCCACHE_NO_DAEMON = "1"

  $cargoArgs = @("test", "--workspace", "-j", "1") + $ExtraCargoArgs
  Write-Host ("==> cargo " + ($cargoArgs -join " "))

  & cargo @cargoArgs
  if ($LASTEXITCODE -ne 0) {
    throw "Cargo workspace test command failed with exit code $LASTEXITCODE"
  }

  Write-Host "[PASS] Cargo workspace test command completed"
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
