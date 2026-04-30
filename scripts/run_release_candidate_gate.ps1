param(
  [ValidateSet("cpu", "gpu-ma", "gpu-kernel")]
  [string]$Variant = "cpu",
  [switch]$IncludePerf,
  [switch]$IncludeGpuValidationPerf,
  [switch]$SkipBrowserQa,
  [switch]$IncludeExpandedBrowserQa,
  [switch]$IncludeBaselineUi,
  [switch]$IncludeDesktopSmoke,
  [string]$OutDir = "",
  [switch]$SkipPackagedSmoke,
  [switch]$SkipWorkspaceTests,
  [switch]$IncludeLiveSafetyHarness,
  [switch]$IncludeOutputParity,
  [switch]$UnsignedBeta,
  [string]$PostBuildSignScript = ""
)

$ErrorActionPreference = "Stop"

function Write-JsonUtf8 {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    $Value,
    [int]$Depth = 16
  )

  $dir = Split-Path -Parent $Path
  if ($dir) {
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
  }
  $Value | ConvertTo-Json -Depth $Depth | Set-Content -Encoding UTF8 $Path
}

function Resolve-OutDir {
  param([string]$Value)
  if (-not [string]::IsNullOrWhiteSpace($Value)) {
    return [System.IO.Path]::GetFullPath($Value)
  }
  $stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
  return Join-Path (Join-Path $repoRoot "output\\release_candidates") "${stamp}-${Variant}"
}

function Read-JsonFile {
  param([string]$Path)
  if (-not [string]::IsNullOrWhiteSpace($Path) -and (Test-Path $Path)) {
    return Get-Content -Raw $Path | ConvertFrom-Json
  }
  return $null
}

function Resolve-ReleaseBinaryPath {
  $candidates = @(
    (Join-Path $repoRoot "src-tauri\\target\\release\\ta_desktop_demo_app.exe"),
    (Join-Path $repoRoot "target\\release\\ta_desktop_demo_app.exe")
  )
  foreach ($candidate in $candidates) {
    if (Test-Path $candidate) {
      return (Resolve-Path $candidate).Path
    }
  }
  return $null
}

function Resolve-ReleaseBundleRoot {
  $candidates = @(
    (Join-Path $repoRoot "src-tauri\\target\\release\\bundle"),
    (Join-Path $repoRoot "target\\release\\bundle")
  )
  foreach ($candidate in $candidates) {
    if (Test-Path $candidate) {
      return (Resolve-Path $candidate).Path
    }
  }
  return $null
}

function Invoke-ReleaseStep {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Name,
    [Parameter(Mandatory = $true)]
    [string]$CommandText,
    [Parameter(Mandatory = $true)]
    [scriptblock]$Action
  )

  $startedAt = (Get-Date).ToUniversalTime()
  Write-Host "==> $Name"
  try {
    & $Action
    $step = [ordered]@{
      name = $Name
      command = $CommandText
      started_at_utc = $startedAt.ToString("o")
      finished_at_utc = (Get-Date).ToUniversalTime().ToString("o")
      passed = $true
    }
    $script:StepResults += $step
    Write-Host "[PASS] $Name"
  } catch {
    $step = [ordered]@{
      name = $Name
      command = $CommandText
      started_at_utc = $startedAt.ToString("o")
      finished_at_utc = (Get-Date).ToUniversalTime().ToString("o")
      passed = $false
      error = $_.Exception.Message
    }
    $script:StepResults += $step
    throw
  }
}

function Write-ReleaseSummary {
  param(
    [string]$Status,
    [string]$ErrorMessage = ""
  )

  $manifest = Read-JsonFile -Path (Join-Path $resolvedOutDir "manifest.json")
  $packagedSmokeSummary = Read-JsonFile -Path (Join-Path $resolvedOutDir "packaged-smoke-summary.json")
  $summary = [ordered]@{
    schema_version = 1
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    variant = $Variant
    status = $Status
    out_dir = $resolvedOutDir
    include_perf = [bool]$IncludePerf
    include_gpu_validation_perf = [bool]$IncludeGpuValidationPerf
    skip_browser_qa = [bool]$SkipBrowserQa
    include_expanded_browser_qa = [bool]$IncludeExpandedBrowserQa
    include_baseline_ui = [bool]$IncludeBaselineUi
    include_desktop_smoke = [bool]$IncludeDesktopSmoke
    include_live_safety_harness = [bool]$IncludeLiveSafetyHarness
    include_output_parity = [bool]$IncludeOutputParity
    skip_packaged_smoke = [bool]$SkipPackagedSmoke
    skip_workspace_tests = [bool]$SkipWorkspaceTests
    unsigned_beta = [bool]$UnsignedBeta
    post_build_sign_script = if ([string]::IsNullOrWhiteSpace($PostBuildSignScript)) { $null } else { [System.IO.Path]::GetFullPath($PostBuildSignScript) }
    error = if ([string]::IsNullOrWhiteSpace($ErrorMessage)) { $null } else { $ErrorMessage }
    artifacts = [ordered]@{
      readiness = (Join-Path $resolvedOutDir "readiness.json")
      provenance = (Join-Path $resolvedOutDir "provenance.json")
      manifest = (Join-Path $resolvedOutDir "manifest.json")
      hashes = (Join-Path $resolvedOutDir "SHA256SUMS.txt")
      packaged_smoke = (Join-Path $resolvedOutDir "packaged-smoke-summary.json")
      live_safety = (Join-Path $resolvedOutDir "live-safety-summary.json")
      desktop_smoke_dir = (Join-Path $resolvedOutDir "desktop-smoke")
    }
    release_artifact = [ordered]@{
      source_kind = if ($null -ne $manifest) { [string]$manifest.artifact_source_kind } else { $null }
      release_binary_path = if ($null -ne $manifest) { [string]$manifest.release_binary_path } else { $null }
      bundle_root = if ($null -ne $manifest) { [string]$manifest.bundle_root } else { $null }
    }
    packaged_smoke = [ordered]@{
      status = if ($null -ne $packagedSmokeSummary) { [string]$packagedSmokeSummary.status } else { $null }
      smoke_mode = if ($null -ne $packagedSmokeSummary) { [string]$packagedSmokeSummary.smoke_mode } else { $null }
      diagnosis = if ($null -ne $packagedSmokeSummary) { [string]$packagedSmokeSummary.diagnosis } else { $null }
    }
    steps = $script:StepResults
  }
  Write-JsonUtf8 -Path $summaryJsonPath -Value $summary

  $lines = @(
    "# Release Candidate Gate",
    "",
    "- Status: $Status",
    "- Variant: $Variant",
    "- Output: $resolvedOutDir",
    "- Include perf: $([bool]$IncludePerf)",
    "- Include GPU validation perf: $([bool]$IncludeGpuValidationPerf)",
    "- Skip browser QA: $([bool]$SkipBrowserQa)",
    "- Include expanded browser QA: $([bool]$IncludeExpandedBrowserQa)",
    "- Include baseline UI: $([bool]$IncludeBaselineUi)",
    "- Include desktop smoke: $([bool]$IncludeDesktopSmoke)",
    "- Include live safety harness: $([bool]$IncludeLiveSafetyHarness)",
    "- Include output parity: $([bool]$IncludeOutputParity)",
    "- Skip packaged smoke: $([bool]$SkipPackagedSmoke)",
    "- Skip workspace tests: $([bool]$SkipWorkspaceTests)",
    "- Unsigned beta artifact labeling: $([bool]$UnsignedBeta)",
    "- Post-build signing hook: $(if ([string]::IsNullOrWhiteSpace($PostBuildSignScript)) { 'none' } else { [System.IO.Path]::GetFullPath($PostBuildSignScript) })"
  )
  if ($null -ne $manifest -and -not [string]::IsNullOrWhiteSpace([string]$manifest.artifact_source_kind)) {
    $lines += "- Release artifact source: $([string]$manifest.artifact_source_kind)"
  }
  if ($null -ne $packagedSmokeSummary -and -not [string]::IsNullOrWhiteSpace([string]$packagedSmokeSummary.smoke_mode)) {
    $lines += "- Packaged smoke mode: $([string]$packagedSmokeSummary.smoke_mode)"
  }
  if (-not [string]::IsNullOrWhiteSpace($ErrorMessage)) {
    $lines += ""
    $lines += "## Failure"
    $lines += ""
    $lines += "- $ErrorMessage"
  }
  $lines += ""
  $lines += "## Steps"
  $lines += ""
  foreach ($step in $script:StepResults) {
    $statusLabel = if ($step.passed) { "PASS" } else { "FAIL" }
    $lines += "- [$statusLabel] $($step.name)"
    $lines += "  command: $($step.command)"
    if ($step.error) {
      $lines += "  error: $($step.error)"
    }
  }
  if ($null -ne $packagedSmokeSummary -and -not [string]::IsNullOrWhiteSpace([string]$packagedSmokeSummary.diagnosis)) {
    $lines += ""
    $lines += "## Packaged Smoke Note"
    $lines += ""
    $lines += "- $([string]$packagedSmokeSummary.diagnosis)"
  }
  Set-Content -Path $summaryMdPath -Encoding UTF8 -Value $lines
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$resolvedOutDir = Resolve-OutDir -Value $OutDir
New-Item -ItemType Directory -Force -Path $resolvedOutDir | Out-Null
$summaryJsonPath = Join-Path $resolvedOutDir "summary.json"
$summaryMdPath = Join-Path $resolvedOutDir "summary.md"
$script:StepResults = @()

try {
  if ($IncludeGpuValidationPerf -and $Variant -ne "gpu-kernel") {
    throw "-IncludeGpuValidationPerf requires -Variant gpu-kernel because the validation perf gate exercises cuda-backtest-kernel."
  }

  $doctorArgs = @("-File", (Join-Path $repoRoot "scripts\\run_release_doctor.ps1"), "-Variant", $Variant, "-OutDir", $resolvedOutDir)
  if ($IncludeDesktopSmoke -and -not $SkipPackagedSmoke) {
    $doctorArgs += "-RequirePackagedSmoke"
  }
  Invoke-ReleaseStep -Name "Release doctor" -CommandText ("powershell " + ($doctorArgs -join " ")) -Action {
    powershell @doctorArgs
    if ($LASTEXITCODE -ne 0) {
      throw "release doctor failed with exit code $LASTEXITCODE"
    }
  }

  $qualityArgs = @("-File", (Join-Path $repoRoot "scripts\\run_repo_quality_gate.ps1"))
  if (-not $SkipWorkspaceTests) {
    $qualityArgs += "-IncludeWorkspaceTests"
  }
  Invoke-ReleaseStep -Name "Repo quality gate" -CommandText ("powershell " + ($qualityArgs -join " ")) -Action {
    powershell @qualityArgs
    if ($LASTEXITCODE -ne 0) {
      throw "repo quality gate failed with exit code $LASTEXITCODE"
    }
  }

  Invoke-ReleaseStep -Name "Web parity" -CommandText "npm --prefix web run parity" -Action {
    & npm.cmd --prefix web run parity
    if ($LASTEXITCODE -ne 0) {
      throw "web parity failed with exit code $LASTEXITCODE"
    }
  }

  if (-not $SkipBrowserQa) {
    $browserQaArgs = @("-File", (Join-Path $repoRoot "scripts\\run_frontend_e2e_gate.ps1"), "-IncludeA11ySmoke")
    if ($IncludeExpandedBrowserQa) {
      $browserQaArgs += @("-IncludeResponsive", "-IncludeRegressions")
    }
    if ($IncludeBaselineUi) {
      $browserQaArgs += "-IncludeBaselineUi"
    }
    Invoke-ReleaseStep -Name "Frontend browser QA" -CommandText ("powershell " + ($browserQaArgs -join " ")) -Action {
      powershell @browserQaArgs
      if ($LASTEXITCODE -ne 0) {
        throw "frontend browser QA failed with exit code $LASTEXITCODE"
      }
    }
  }

  if ($IncludeOutputParity) {
    Invoke-ReleaseStep -Name "Output parity" -CommandText "node scripts/run_output_parity_check.mjs" -Action {
      node "$repoRoot\\scripts\\run_output_parity_check.mjs"
      if ($LASTEXITCODE -ne 0) {
        throw "output parity failed with exit code $LASTEXITCODE"
      }
    }
  } else {
    Write-Host "[SKIP] Output parity sample baselines are local-only in the backend-preview public tree."
  }

  $workflowArgs = @("-File", (Join-Path $repoRoot "scripts\\run_workflow_gate.ps1"), "-Mode", "candidate")
  Invoke-ReleaseStep -Name "Workflow gate" -CommandText ("powershell " + ($workflowArgs -join " ")) -Action {
    powershell @workflowArgs
    if ($LASTEXITCODE -ne 0) {
      throw "workflow gate failed with exit code $LASTEXITCODE"
    }
  }

  if ($IncludePerf) {
    $cpuArgs = @("-File", (Join-Path $repoRoot "scripts\\run_cpu_perf_gate.ps1"), "-Mode", "candidate")
    Invoke-ReleaseStep -Name "CPU perf gate" -CommandText ("powershell " + ($cpuArgs -join " ")) -Action {
      powershell @cpuArgs
      if ($LASTEXITCODE -ne 0) {
        throw "CPU perf gate failed with exit code $LASTEXITCODE"
      }
    }

    if ($Variant -ne "cpu") {
      $gpuArgs = @("-File", (Join-Path $repoRoot "scripts\\run_gpu_perf_gate.ps1"), "-Mode", "candidate")
      Invoke-ReleaseStep -Name "GPU perf gate" -CommandText ("powershell " + ($gpuArgs -join " ")) -Action {
        powershell @gpuArgs
        if ($LASTEXITCODE -ne 0) {
          throw "GPU perf gate failed with exit code $LASTEXITCODE"
        }
      }
    }
  }

  if ($IncludeGpuValidationPerf) {
    $gpuValidationArgs = @(
      "-File", (Join-Path $repoRoot "scripts\\run_gpu_validation_perf_gate.ps1"),
      "-Mode", "candidate",
      "-WorkloadProfile", "release-authority"
    )
    Invoke-ReleaseStep -Name "GPU validation perf gate" -CommandText ("powershell " + ($gpuValidationArgs -join " ")) -Action {
      powershell @gpuValidationArgs
      if ($LASTEXITCODE -ne 0) {
        throw "GPU validation perf gate failed with exit code $LASTEXITCODE"
      }
    }
  }

  $buildCommand = switch ($Variant) {
    "cpu" { "Push-Location src-tauri; cargo tauri build --ci --bundles nsis; Pop-Location" }
    "gpu-ma" { "Push-Location src-tauri; cargo tauri build --ci --bundles nsis --features cuda; Pop-Location" }
    "gpu-kernel" { "Push-Location src-tauri; VECTORBT_USE_PREBUILT_PTX=1 cargo tauri build --ci --bundles nsis --features cuda-backtest-kernel; Pop-Location" }
  }
  Invoke-ReleaseStep -Name "Tauri release build" -CommandText $buildCommand -Action {
    $prevPrebuilt = $env:VECTORBT_USE_PREBUILT_PTX
    $pushedLocation = $false
    try {
      Push-Location (Join-Path $repoRoot "src-tauri")
      $pushedLocation = $true
      if ($Variant -eq "gpu-kernel") {
        $env:VECTORBT_USE_PREBUILT_PTX = "1"
        & cargo tauri build --ci --bundles nsis --features cuda-backtest-kernel
      } elseif ($Variant -eq "gpu-ma") {
        & cargo tauri build --ci --bundles nsis --features cuda
      } else {
        & cargo tauri build --ci --bundles nsis
      }
      if ($LASTEXITCODE -ne 0) {
        throw "tauri build failed with exit code $LASTEXITCODE"
      }
    } finally {
      if ($pushedLocation) {
        Pop-Location
      }
      if ($null -ne $prevPrebuilt) {
        $env:VECTORBT_USE_PREBUILT_PTX = $prevPrebuilt
      } else {
        Remove-Item Env:VECTORBT_USE_PREBUILT_PTX -ErrorAction SilentlyContinue
      }
    }
  }

  $releaseBinaryPath = Resolve-ReleaseBinaryPath
  if ($null -eq $releaseBinaryPath) {
    throw "tauri build completed but no release executable was found under src-tauri\\target\\release or target\\release."
  }
  $releaseBundleRoot = Resolve-ReleaseBundleRoot

  $effectivePostBuildSignScript = if ([string]::IsNullOrWhiteSpace($PostBuildSignScript)) {
    [string]$env:VECTORGRID_POST_BUILD_SIGN_SCRIPT
  } else {
    $PostBuildSignScript
  }
  if (-not [string]::IsNullOrWhiteSpace($effectivePostBuildSignScript)) {
    $resolvedSignScript = [System.IO.Path]::GetFullPath($effectivePostBuildSignScript)
    if (-not (Test-Path -LiteralPath $resolvedSignScript -PathType Leaf)) {
      throw "Post-build signing script does not exist: $resolvedSignScript"
    }
    $signArgs = @(
      "-ExecutionPolicy", "Bypass",
      "-File", $resolvedSignScript,
      "-Variant", $Variant,
      "-OutDir", $resolvedOutDir,
      "-ReleaseBinaryPath", $releaseBinaryPath
    )
    if ($null -ne $releaseBundleRoot) {
      $signArgs += @("-BundleRoot", $releaseBundleRoot)
    }
    Invoke-ReleaseStep -Name "Post-build signing hook" -CommandText ("powershell " + ($signArgs -join " ")) -Action {
      powershell @signArgs
      if ($LASTEXITCODE -ne 0) {
        throw "post-build signing hook failed with exit code $LASTEXITCODE"
      }
    }
  }

  $provenanceArgs = @(
    "-File", (Join-Path $repoRoot "scripts\\capture_release_provenance.ps1"),
    "-Variant", $Variant,
    "-OutDir", $resolvedOutDir,
    "-BinaryPath", $releaseBinaryPath
  )
  Invoke-ReleaseStep -Name "Release provenance" -CommandText ("powershell " + ($provenanceArgs -join " ")) -Action {
    powershell @provenanceArgs
    if ($LASTEXITCODE -ne 0) {
      throw "release provenance failed with exit code $LASTEXITCODE"
    }
  }

  $manifestArgs = @(
    "-File", (Join-Path $repoRoot "scripts\\generate_release_manifest.ps1"),
    "-Variant", $Variant,
    "-OutDir", $resolvedOutDir,
    "-ReleaseBinaryPath", $releaseBinaryPath,
    "-ProvenancePath", (Join-Path $resolvedOutDir "provenance.json")
  )
  if ($UnsignedBeta) {
    $manifestArgs += "-UnsignedBeta"
  }
  Invoke-ReleaseStep -Name "Release manifest" -CommandText ("powershell " + ($manifestArgs -join " ")) -Action {
    powershell @manifestArgs
    if ($LASTEXITCODE -ne 0) {
      throw "release manifest generation failed with exit code $LASTEXITCODE"
    }
  }

  if ($IncludeDesktopSmoke -and -not $SkipPackagedSmoke) {
    $smokeArgs = @(
      "-File", (Join-Path $repoRoot "scripts\\run_packaged_smoke.ps1"),
      "-BinaryPath", $releaseBinaryPath,
      "-OutDir", $resolvedOutDir
    )
    Invoke-ReleaseStep -Name "Packaged desktop smoke" -CommandText ("powershell " + ($smokeArgs -join " ")) -Action {
      powershell @smokeArgs
      if ($LASTEXITCODE -ne 0) {
        throw "packaged desktop smoke failed with exit code $LASTEXITCODE"
      }
    }
  }

  if ($IncludeLiveSafetyHarness) {
    $liveArgs = @("-File", (Join-Path $repoRoot "scripts\\run_live_execution_safety_harness.ps1"), "-OutDir", $resolvedOutDir)
    Invoke-ReleaseStep -Name "Live execution safety harness" -CommandText ("powershell " + ($liveArgs -join " ")) -Action {
      powershell @liveArgs
      if ($LASTEXITCODE -ne 0) {
        throw "live execution safety harness failed with exit code $LASTEXITCODE"
      }
    }
  }

  Write-ReleaseSummary -Status "passed"
  Write-Host "Release candidate gate completed successfully."
  Write-Host "Summary written to $summaryJsonPath"
} catch {
  Write-ReleaseSummary -Status "failed" -ErrorMessage $_.Exception.Message
  throw
}
