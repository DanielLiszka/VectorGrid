param(
  [switch]$SkipBuild,
  [string]$OutPath = "",
  [string]$BuildAttestationPath = "output/qa/frontend-interaction-latency/latest-build-attestation.json"
)

$ErrorActionPreference = "Stop"

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$outputRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "output"))
$runner = Join-Path $repoRoot "web\scripts\measure-frontend-interaction-latency.mjs"
$attestor = Join-Path $repoRoot "web\scripts\frontend-interaction-build-attestation.mjs"
$leaseVerifier = Join-Path $repoRoot "web\scripts\frontend-interaction-authority-lease.mjs"
$reportValidator = Join-Path $repoRoot "web\scripts\frontend-interaction-report-validator.mjs"
$policyPath = Join-Path $repoRoot "scripts\frontend_interaction_latency_policy.json"
$processPreflight = Join-Path $repoRoot "scripts\performance_audit_preflight.ps1"
$benchmarkContext = "scoped warm mocked-browser click-to-painted-stable latency"
$benchmarkMutexName = "Local\ta_desktop_demo_benchmark_gate"
$authorityMutexName = "Local\ta_desktop_demo_frontend_interaction_authority"
$defaultBuildAttestationPath = "output/qa/frontend-interaction-latency/latest-build-attestation.json"
. (Join-Path $PSScriptRoot "benchmark_lock.ps1")
. $processPreflight

function Invoke-Step {
  param(
    [Parameter(Mandatory = $true)][string]$Name,
    [Parameter(Mandatory = $true)][scriptblock]$Action
  )

  Write-Host "==> $Name"
  & $Action
  if ($LASTEXITCODE -ne 0) {
    throw "$Name failed with exit code $LASTEXITCODE"
  }
  Write-Host "[PASS] $Name"
}

function Resolve-BoundedOutputPath {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [switch]$MustNotExist
  )

  if ($Path -match '^(\\\\|//)[?.](\\|/)') {
    throw "Device-namespace paths are forbidden: $Path"
  }
  $resolved = if ([System.IO.Path]::IsPathRooted($Path)) {
    [System.IO.Path]::GetFullPath($Path)
  } else {
    [System.IO.Path]::GetFullPath((Join-Path $repoRoot $Path))
  }
  $outputPrefix = $outputRoot.TrimEnd("\") + "\"
  if (
    $resolved -eq $outputRoot -or
    -not $resolved.StartsWith($outputPrefix, [System.StringComparison]::OrdinalIgnoreCase)
  ) {
    throw "Path must remain below ${outputRoot}: $resolved"
  }

  New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null
  $outputItem = Get-Item -LiteralPath $outputRoot -Force
  if (
    -not $outputItem.PSIsContainer -or
    ($outputItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0
  ) {
    throw "Output root must be an ordinary directory: $outputRoot"
  }

  $relative = [System.IO.Path]::GetRelativePath($outputRoot, $resolved)
  $segments = @($relative -split '[\\/]')
  foreach ($segment in $segments) {
    if (
      [string]::IsNullOrWhiteSpace($segment) -or
      $segment -in @(".", "..") -or
      $segment.Contains(":") -or
      $segment.EndsWith(".") -or
      $segment.EndsWith(" ") -or
      $segment -match '^(?i:con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)'
    ) {
      throw "Path contains an unsafe output component: $resolved"
    }
  }

  $cursor = $outputRoot
  foreach ($segment in $segments) {
    $cursor = Join-Path $cursor $segment
    if (-not (Test-Path -LiteralPath $cursor)) {
      break
    }
    $item = Get-Item -LiteralPath $cursor -Force
    if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
      throw "Output path contains a reparse point: $cursor"
    }
    if ($cursor -ne $resolved -and -not $item.PSIsContainer) {
      throw "Output parent is not a directory: $cursor"
    }
  }

  $realOutput = (Resolve-Path -LiteralPath $outputRoot).Path.TrimEnd("\")
  if (Test-Path -LiteralPath $resolved) {
    $existingItem = Get-Item -LiteralPath $resolved -Force
    if (($existingItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
      throw "Path must not be a reparse-point target: $resolved"
    }
    if (
      $existingItem.PSObject.Properties.Name -contains "LinkType" -and
      "$($existingItem.LinkType)" -eq "HardLink"
    ) {
      throw "Path must not be a hardlink target: $resolved"
    }
    $realTarget = (Resolve-Path -LiteralPath $resolved).Path
    if (
      $realTarget -eq $realOutput -or
      -not $realTarget.StartsWith($realOutput + "\", [System.StringComparison]::OrdinalIgnoreCase)
    ) {
      throw "Path resolves outside output/: $resolved"
    }
  }
  if ($MustNotExist -and (Test-Path -LiteralPath $resolved)) {
    throw "Refusing existing or stale output target: $resolved"
  }
  return $resolved
}

function Get-BoundedFrontendProcessSnapshot {
  return @(
    Invoke-VectorGridBoundedProcessSnapshot `
      -TimeoutMilliseconds 5000 `
      -SnapshotProvider {
        foreach ($row in @(Get-CimInstance Win32_Process -ErrorAction Stop)) {
          [pscustomobject]@{
            ProcessId = [int]$row.ProcessId
            ParentProcessId = [int]$row.ParentProcessId
            Name = [string]$row.Name
            CommandLine = [string]$row.CommandLine
            StartedAt = if ($null -eq $row.CreationDate) {
              ""
            } else {
              $row.CreationDate.ToUniversalTime().ToString("o")
            }
          }
        }
      }
  )
}

function Get-AncestorProcessIds {
  param(
    [Parameter(Mandatory = $true)][object[]]$Snapshot,
    [int]$StartProcessId = $PID
  )

  $byId = @{}
  foreach ($row in $Snapshot) {
    $byId[[int]$row.ProcessId] = $row
  }
  $ids = [System.Collections.Generic.HashSet[int]]::new()
  $currentId = $StartProcessId
  while ($currentId -gt 0 -and $ids.Add($currentId)) {
    if (-not $byId.ContainsKey($currentId)) {
      throw "Bounded process snapshot is missing ancestry row for PID $currentId."
    }
    $row = $byId[$currentId]
    $currentId = [int]$row.ParentProcessId
  }
  return ,$ids
}

function Get-RequiredProcessSnapshotRow {
  param(
    [Parameter(Mandatory = $true)][object[]]$Snapshot,
    [Parameter(Mandatory = $true)][int]$ProcessId
  )
  $matches = @($Snapshot | Where-Object { [int]$_.ProcessId -eq $ProcessId })
  if ($matches.Count -ne 1) {
    throw "Bounded process snapshot did not contain exactly one row for PID $ProcessId."
  }
  return $matches[0]
}

function Assert-NoConcurrentFrontendPerformanceProcess {
  $processRows = @(Get-BoundedFrontendProcessSnapshot)
  $excluded = Get-AncestorProcessIds -Snapshot $processRows
  $repoNeedle = $repoRoot.ToLowerInvariant()
  $blocked = @()
  foreach ($row in $processRows) {
    $processId = [int]$row.ProcessId
    if ($excluded.Contains($processId)) {
      continue
    }
    $name = ([string]$row.Name).ToLowerInvariant()
    $commandLine = ([string]$row.CommandLine).ToLowerInvariant()
    $reason = ""

    if ($name -in @("cargo.exe", "rustc.exe", "ncu.exe", "nsys.exe", "nsys-ui.exe")) {
      $reason = "build/benchmark/profiler"
    } elseif (
      $name -in @("node.exe", "npm.exe", "npx.exe") -and
      (
        $commandLine.Contains($repoNeedle) -or
        $commandLine -match '(^|[\\/\s])vite(\.js)?([\\/\s]|$)' -or
        $commandLine.Contains("playwright") -or
        $commandLine.Contains("frontend-interaction-latency")
      )
    ) {
      $reason = "Node/Vite/Playwright"
    } elseif (
      $name -in @(
        "chrome.exe",
        "chromium.exe",
        "msedge.exe",
        "headless_shell.exe",
        "chrome-headless-shell.exe"
      ) -and
      (
        $commandLine.Contains("--remote-debugging-pipe") -or
        $commandLine.Contains("ms-playwright") -or
        $commandLine.Contains("playwright_chromium") -or
        (
          $commandLine.Contains("--headless") -and
          $commandLine.Contains("--remote-debugging-port")
        )
      )
    ) {
      $reason = "Playwright Chromium"
    }

    if ($reason) {
      $blocked += [pscustomobject]@{
        id = $processId
        name = $name
        reason = $reason
      }
    }
  }
  if ($blocked.Count -gt 0) {
    $summary = ($blocked | ForEach-Object { "$($_.name)#$($_.id) [$($_.reason)]" }) -join ", "
    throw "Refusing scoped frontend latency authority while relevant concurrent processes are active: $summary"
  }
}

function Get-Sha256Text {
  param([Parameter(Mandatory = $true)][string]$Value)
  $bytes = [System.Text.Encoding]::UTF8.GetBytes($Value)
  $sha = [System.Security.Cryptography.SHA256]::Create()
  try {
    return (
      [System.BitConverter]::ToString($sha.ComputeHash($bytes))
    ).Replace("-", "").ToLowerInvariant()
  } finally {
    $sha.Dispose()
  }
}

function Assert-AuthorityMutexHeld {
  $probeJson = & node $leaseVerifier mutex-probe
  if ($LASTEXITCODE -ne 0) {
    throw "Named benchmark mutex is not held by a live external owner."
  }
  $probe = ($probeJson -join "`n") | ConvertFrom-Json
  if (
    -not [bool]$probe.held_by_another_process -or
    [int]$probe.exit_code -ne 0
  ) {
    throw "Named benchmark mutex probe did not prove live ownership."
  }
}

function Assert-BenchmarkMutexHeld {
  $probeJson = & node $leaseVerifier benchmark-mutex-probe
  if ($LASTEXITCODE -ne 0) {
    throw "Repository benchmark mutex is not held by a live external owner."
  }
  $probe = ($probeJson -join "`n") | ConvertFrom-Json
  if (
    -not [bool]$probe.held_by_another_process -or
    [int]$probe.exit_code -ne 0
  ) {
    throw "Repository benchmark mutex probe did not prove live ownership."
  }
}

function Enter-FrontendAuthorityMutex {
  $mutex = [System.Threading.Mutex]::new($false, $authorityMutexName)
  $acquired = $false
  try {
    try {
      $acquired = $mutex.WaitOne([TimeSpan]::FromSeconds(30))
    } catch [System.Threading.AbandonedMutexException] {
      $acquired = $true
    }
    if (-not $acquired) {
      throw "Timed out waiting for the frontend interaction authority mutex."
    }
    return $mutex
  } catch {
    if ($acquired) {
      try {
        $mutex.ReleaseMutex() | Out-Null
      } catch {
      }
    }
    $mutex.Dispose()
    throw
  }
}

function Exit-FrontendAuthorityMutex {
  param($Mutex)
  if ($null -eq $Mutex) {
    return
  }
  try {
    $Mutex.ReleaseMutex() | Out-Null
  } finally {
    $Mutex.Dispose()
  }
}

function Get-ProcessStartIdentity {
  param([Parameter(Mandatory = $true)][int]$ProcessId)
  $process = Get-Process -Id $ProcessId -ErrorAction Stop
  return $process.StartTime.ToUniversalTime().ToString("o")
}

function Assert-BenchmarkLockOwnership {
  param([Parameter(Mandatory = $true)]$LockHandle)

  if ([bool]$LockHandle.inherited) {
    throw "Frontend interaction authority refuses inherited benchmark-lock markers; invoke it directly so this wrapper owns the repository benchmark mutex."
  }
  if (-not [bool]$LockHandle.acquired) {
    throw "Frontend interaction authority requires a directly acquired repository benchmark mutex."
  }

  $snapshot = @(Get-BoundedFrontendProcessSnapshot)
  $ancestors = Get-AncestorProcessIds -Snapshot $snapshot
  $null = Get-RequiredProcessSnapshotRow -Snapshot $snapshot -ProcessId $PID

  $mutexOwnerPid = 0
  if (
    -not [int]::TryParse(
      [string]$env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_PID,
      [ref]$mutexOwnerPid
    ) -or
    $mutexOwnerPid -le 0
  ) {
    throw "Benchmark-lock owner PID is missing or invalid."
  }
  if (-not $ancestors.Contains($mutexOwnerPid)) {
    throw "Benchmark-lock owner PID $mutexOwnerPid is not in the wrapper ancestry."
  }
  if (
    $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_HELD -ne "1" -or
    $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN -notmatch "^[a-f0-9]{32}$" -or
    $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN_SHA256 -notmatch
    "^[a-f0-9]{64}$" -or
    $env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_LEASE_SHA256 -notmatch
    "^[a-f0-9]{64}$" -or
    [string]::IsNullOrWhiteSpace(
      [string]$env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_CONTEXT
    )
  ) {
    throw "Benchmark-lock inheritance markers are incomplete or invalid."
  }

  if ($mutexOwnerPid -ne $PID) {
    throw "Directly acquired benchmark mutex does not identify this wrapper."
  }
  $wrapperStartedAt = Get-ProcessStartIdentity -ProcessId $PID
  $tokenSha256 = Get-Sha256Text `
    -Value ([string]$env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN)
  $canonicalLease = Read-BenchmarkLockLease `
    -Path ([string]$env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_LEASE_PATH)
  $canonicalWriteProbe = Get-BenchmarkLockLeaseWriteProbeCode `
    -Path $canonicalLease.path
  if (
    "$($env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_STARTED_AT)" -ne
    $wrapperStartedAt -or
    "$($env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN_SHA256)" -ne
    $tokenSha256 -or
    "$($env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_LEASE_SHA256)" -ne
    "$($canonicalLease.sha256)" -or
    "$($env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_MUTEX_NAME)" -ne
    $benchmarkMutexName -or
    "$($LockHandle.lease_path)" -ne "$($canonicalLease.path)" -or
    "$($LockHandle.lease_sha256)" -ne "$($canonicalLease.sha256)" -or
    "$($LockHandle.lease_identity)" -ne "$($canonicalLease.identity)" -or
    "$($LockHandle.owner_started_at)" -ne $wrapperStartedAt -or
    "$($LockHandle.token_sha256)" -ne $tokenSha256 -or
    "$($LockHandle.mutex_name)" -ne $benchmarkMutexName -or
    "$($canonicalLease.payload.owner_pid)" -ne "$PID" -or
    "$($canonicalLease.payload.owner_started_at)" -ne $wrapperStartedAt -or
    "$($canonicalLease.payload.context)" -ne
    "$($env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_CONTEXT)" -or
    "$($canonicalLease.payload.token_sha256)" -ne $tokenSha256 -or
    "$($canonicalLease.payload.mutex_name)" -ne $benchmarkMutexName -or
    "$($canonicalLease.payload.file_identity)" -ne
    "$($canonicalLease.identity)" -or
    [int]$canonicalWriteProbe -ne 32
  ) {
    throw "Canonical benchmark lease does not match this wrapper's live lock ownership."
  }
  Assert-BenchmarkLockLeaseHolder `
    -Path $canonicalLease.path `
    -OwnerPid $PID `
    -OwnerStartedAt $wrapperStartedAt
  Assert-BenchmarkMutexHeld
  return [pscustomobject]@{
    wrapper_started_at = $wrapperStartedAt
    benchmark_mutex_owner_pid = $mutexOwnerPid
    benchmark_mutex_owner_started_at = Get-ProcessStartIdentity `
      -ProcessId $mutexOwnerPid
    benchmark_mutex_ownership_mode = "acquired"
    benchmark_lock_context = [string]$env:TA_DESKTOP_DEMO_BENCHMARK_LOCK_CONTEXT
    benchmark_lock_token_sha256 = $tokenSha256
    benchmark_lease_path = [System.IO.Path]::GetRelativePath(
      $repoRoot,
      $canonicalLease.path
    ).Replace("\", "/")
    benchmark_lease_sha256 = [string]$canonicalLease.sha256
    benchmark_lease_identity = [string]$canonicalLease.identity
    benchmark_lease_write_probe_win32_error_code = [int]$canonicalWriteProbe
  }
}

function New-AuthorityLease {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)]$Ownership,
    [Parameter(Mandatory = $true)][string]$RunId,
    [Parameter(Mandatory = $true)][string]$CreatedAt
  )

  $resolvedPath = Resolve-BoundedOutputPath -Path $Path -MustNotExist
  $parent = Split-Path -Parent $resolvedPath
  New-Item -ItemType Directory -Force -Path $parent | Out-Null
  $resolvedPath = Resolve-BoundedOutputPath -Path $resolvedPath -MustNotExist
  $nonce = [guid]::NewGuid().ToString("N")
  $payload = [ordered]@{
    schema_version = 1
    lease_kind = "frontend-interaction-authority-lease"
    created_at = $CreatedAt
    run_id = $RunId
    nonce = $nonce
    wrapper_path = "scripts/run_frontend_interaction_latency.ps1"
    wrapper_context = $benchmarkContext
    wrapper_pid = $PID
    wrapper_started_at = [string]$Ownership.wrapper_started_at
    mutex_name = $authorityMutexName
    mutex_owner_pid = $PID
    mutex_owner_started_at = [string]$Ownership.wrapper_started_at
    mutex_ownership_mode = "acquired"
    benchmark_mutex_name = $benchmarkMutexName
    benchmark_mutex_owner_pid = [int]$Ownership.benchmark_mutex_owner_pid
    benchmark_mutex_owner_started_at = [string]$Ownership.benchmark_mutex_owner_started_at
    benchmark_mutex_ownership_mode = [string]$Ownership.benchmark_mutex_ownership_mode
    benchmark_lock_context = [string]$Ownership.benchmark_lock_context
    benchmark_lock_token_sha256 = [string]$Ownership.benchmark_lock_token_sha256
    benchmark_lease_path = [string]$Ownership.benchmark_lease_path
    benchmark_lease_sha256 = [string]$Ownership.benchmark_lease_sha256
    benchmark_lease_identity = [string]$Ownership.benchmark_lease_identity
    benchmark_lease_write_probe_win32_error_code = (
      [int]$Ownership.benchmark_lease_write_probe_win32_error_code
    )
  }
  $json = ($payload | ConvertTo-Json -Depth 5) + "`n"
  $bytes = [System.Text.UTF8Encoding]::new($false).GetBytes($json)
  $stream = [System.IO.FileStream]::new(
    $resolvedPath,
    [System.IO.FileMode]::CreateNew,
    [System.IO.FileAccess]::ReadWrite,
    [System.IO.FileShare]::Read
  )
  try {
    $stream.Write($bytes, 0, $bytes.Length)
    $stream.Flush($true)
    $stream.Position = 0
    $resolvedPath = Resolve-BoundedOutputPath -Path $resolvedPath
    return [pscustomobject]@{
      path = $resolvedPath
      nonce = $nonce
      stream = $stream
      payload = $payload
    }
  } catch {
    $stream.Dispose()
    throw
  }
}

function Read-StableAuthorityJson {
  param([Parameter(Mandatory = $true)][string]$Path)

  $resolvedPath = Resolve-BoundedOutputPath -Path $Path
  $beforeJson = & node $attestor inspect-output --out $resolvedPath
  if ($LASTEXITCODE -ne 0) {
    throw "Authority output failed pre-read containment validation."
  }
  $before = ($beforeJson -join "`n") | ConvertFrom-Json
  $bytes = [System.IO.File]::ReadAllBytes($resolvedPath)
  $resolvedPath = Resolve-BoundedOutputPath -Path $resolvedPath
  $afterJson = & node $attestor inspect-output `
    --out $resolvedPath `
    --expected-sha ([string]$before.sha256)
  if ($LASTEXITCODE -ne 0) {
    throw "Authority output changed or escaped containment while being read."
  }
  $after = ($afterJson -join "`n") | ConvertFrom-Json
  if (
    [long]$before.size -ne [long]$after.size -or
    [long]$before.size -ne [long]$bytes.Length -or
    [string]$before.sha256 -ne [string]$after.sha256 -or
    [string]$before.path -ne [string]$after.path
  ) {
    throw "Authority output publication identity changed while being read."
  }
  $json = [System.Text.UTF8Encoding]::new($false, $true).GetString($bytes)
  return [pscustomobject]@{
    report = $json | ConvertFrom-Json
    publication = $after
  }
}

foreach ($requiredFile in @(
    $runner,
    $attestor,
    $leaseVerifier,
    $reportValidator,
    $policyPath,
    $processPreflight
  )) {
  if (-not (Test-Path -LiteralPath $requiredFile -PathType Leaf)) {
    throw "Missing frontend interaction latency dependency: $requiredFile"
  }
}

$policy = Get-Content -Raw -LiteralPath $policyPath | ConvertFrom-Json
$policySha256 = (
  Get-FileHash -LiteralPath $policyPath -Algorithm SHA256
).Hash.ToLowerInvariant()
if ($policySha256 -notmatch "^[a-f0-9]{64}$") {
  throw "Frontend interaction latency policy did not hash to a SHA-256."
}
$maxAttestationAgeMinutes = [int]$policy.build_attestation_max_age_minutes
if ($maxAttestationAgeMinutes -lt 1) {
  throw "Invalid build attestation age policy."
}

$runId = [guid]::NewGuid().ToString("N")
$runStarted = [DateTimeOffset]::UtcNow
$runStartedText = $runStarted.ToString("o")
if ([string]::IsNullOrWhiteSpace($OutPath)) {
  $safeTimestamp = $runStarted.ToString("yyyyMMddTHHmmssfffZ")
  $OutPath = "output/playwright/frontend-interaction-latency/$safeTimestamp-$runId/summary.json"
}
$publishLatestAttestation = $false
if (
  -not $SkipBuild -and
  $BuildAttestationPath -eq $defaultBuildAttestationPath
) {
  $publishLatestAttestation = $true
  $BuildAttestationPath = "output/qa/frontend-interaction-latency/build-attestations/$runId.json"
}
$leasePath = "output/qa/frontend-interaction-latency/leases/$runId.json"
$distSnapshotPath = "output/qa/frontend-interaction-latency/dist-snapshots/$runId"
$resolvedOut = Resolve-BoundedOutputPath -Path $OutPath -MustNotExist
$resolvedAttestation = Resolve-BoundedOutputPath -Path $BuildAttestationPath
$resolvedDistSnapshot = Resolve-BoundedOutputPath `
  -Path $distSnapshotPath `
  -MustNotExist

Push-Location $repoRoot
$lock = $null
$authorityMutex = $null
$authorityLease = $null
try {
  $lock = Enter-BenchmarkLock -Context $benchmarkContext
  $ownership = Assert-BenchmarkLockOwnership -LockHandle $lock
  $authorityMutex = Enter-FrontendAuthorityMutex
  Assert-AuthorityMutexHeld
  $authorityLease = New-AuthorityLease `
    -Path $leasePath `
    -Ownership $ownership `
    -RunId $runId `
    -CreatedAt $runStartedText
  Assert-NoConcurrentFrontendPerformanceProcess

  if ($SkipBuild) {
    if (-not (Test-Path -LiteralPath $resolvedAttestation -PathType Leaf)) {
      throw "-SkipBuild requires a fresh build attestation: $resolvedAttestation"
    }
    Invoke-Step "Verify fresh frontend source/build attestation" {
      & node $attestor verify `
        --attestation $resolvedAttestation `
        --max-age-minutes "$maxAttestationAgeMinutes"
    }
  } else {
    $fingerprintJson = & node $attestor fingerprint
    if ($LASTEXITCODE -ne 0) {
      throw "Frontend build-input fingerprint failed with exit code $LASTEXITCODE"
    }
    $sourceFingerprint = ($fingerprintJson -join "`n") | ConvertFrom-Json
    $sourceSha = [string]$sourceFingerprint.sha256
    if ($sourceSha -notmatch "^[a-f0-9]{64}$") {
      throw "Frontend build-input fingerprint did not return a SHA-256."
    }

    Invoke-Step "Production web build under benchmark lock" {
      & npm.cmd --prefix web run build
    }
    Invoke-Step "Capture fresh frontend source/build attestation" {
      & node $attestor capture `
        --out $resolvedAttestation `
        --expected-source-sha $sourceSha
    }
    if ($publishLatestAttestation) {
      Invoke-Step "Publish non-authority latest-attestation convenience copy" {
        & node $attestor publish-copy `
          --attestation $resolvedAttestation `
          --out $defaultBuildAttestationPath
      }
    }
  }

  $attestationRead = Read-StableAuthorityJson -Path $resolvedAttestation
  $distSha = [string]$attestationRead.report.dist.sha256
  if ($distSha -notmatch "^[a-f0-9]{64}$") {
    throw "Build attestation did not contain a valid distribution SHA-256."
  }
  Invoke-Step "Seal immutable per-run frontend distribution snapshot" {
    & node $attestor snapshot `
      --snapshot $resolvedDistSnapshot `
      --expected-dist-sha $distSha
  }
  $snapshotManifestPath = Join-Path $resolvedDistSnapshot "manifest.json"
  $snapshotManifestRead = Read-StableAuthorityJson -Path $snapshotManifestPath
  Invoke-Step "Verify child-readable benchmark ownership lease" {
    & node $leaseVerifier probe `
      --lease $authorityLease.path `
      --nonce $authorityLease.nonce `
      --run-id $runId `
      --max-age-minutes "$($policy.report_freshness_max_minutes)"
  }

  Assert-NoConcurrentFrontendPerformanceProcess
  Write-Host "==> Scoped warm mocked-browser click-to-painted-stable authority"
  $runnerOutput = @(
    & node $runner `
      --authority `
      --build-attestation $resolvedAttestation `
      --dist-snapshot $resolvedDistSnapshot `
      --lock-lease $authorityLease.path `
      --lock-nonce $authorityLease.nonce `
      --out $resolvedOut `
      --run-id $runId `
      --run-started-at $runStartedText 2>&1
  )
  $runnerExitCode = $LASTEXITCODE
  foreach ($line in $runnerOutput) {
    Write-Host "$line"
  }
  if ($runnerExitCode -ne 0) {
    throw "Scoped warm mocked-browser click-to-painted-stable authority failed with exit code $runnerExitCode"
  }
  $runnerShaLines = @(
    $runnerOutput |
      ForEach-Object { "$_" } |
      Where-Object { $_ -match "^Evidence-SHA256: [a-f0-9]{64}$" }
  )
  if ($runnerShaLines.Count -ne 1) {
    throw "Frontend latency runner did not publish exactly one evidence SHA-256."
  }
  $runnerPublicationSha256 = (
    "$($runnerShaLines[0])" -replace "^Evidence-SHA256: ", ""
  )
  if ($runnerPublicationSha256 -notmatch "^[a-f0-9]{64}$") {
    throw "Frontend latency runner publication SHA-256 is invalid."
  }
  Write-Host "[PASS] Scoped warm mocked-browser click-to-painted-stable authority"

  if (-not (Test-Path -LiteralPath $resolvedOut -PathType Leaf)) {
    throw "Frontend latency runner did not write evidence: $resolvedOut"
  }
  Invoke-Step "Reverify benchmark ownership lease after measurement" {
    & node $leaseVerifier probe `
      --lease $authorityLease.path `
      --nonce $authorityLease.nonce `
      --run-id $runId `
      --max-age-minutes "$($policy.report_freshness_max_minutes)"
  }
  Assert-AuthorityMutexHeld
  Assert-BenchmarkMutexHeld
  $resolvedOut = Resolve-BoundedOutputPath -Path $resolvedOut
  $reportFile = Get-Item -LiteralPath $resolvedOut
  if ($reportFile.LastWriteTimeUtc -lt $runStarted.UtcDateTime) {
    throw "Frontend latency evidence predates this run."
  }
  $stableReportRead = Read-StableAuthorityJson -Path $resolvedOut
  $report = $stableReportRead.report
  if (
    "$($stableReportRead.publication.sha256)" -ne
    $runnerPublicationSha256
  ) {
    throw "Frontend latency evidence bytes do not match the runner publication SHA-256."
  }
  if ("$($report.status)" -ne "passed" -or -not [bool]$report.authority_eligible) {
    throw "Frontend latency evidence is not authoritative: status=$($report.status) authority_eligible=$($report.authority_eligible)"
  }
  $expectedReleaseEvidenceEligibility = [bool]$report.source.scoped_worktree_clean
  $expectedEvidenceScope = if ($expectedReleaseEvidenceEligibility) {
    "clean-source-release-evidence-candidate"
  } else {
    "dirty-source-local-candidate-rehearsal"
  }
  if (
    [bool]$report.release_evidence_eligible -ne
      $expectedReleaseEvidenceEligibility -or
    "$($report.evidence_scope)" -ne $expectedEvidenceScope
  ) {
    throw "Frontend latency evidence has inconsistent worktree/release classification."
  }
  if (-not $expectedReleaseEvidenceEligibility) {
    Write-Warning "Frontend latency passed as exact snapshot-bound local candidate authority only; the scoped worktree is dirty, so this evidence is ineligible for release publication."
  }
  if ("$($report.run.id)" -ne $runId -or "$($report.run.started_at)" -ne $runStartedText) {
    throw "Frontend latency evidence run identity does not match the wrapper."
  }
  $generatedAt = [DateTimeOffset]::Parse([string]$report.generated_at)
  $completedAt = [DateTimeOffset]::Parse([string]$report.run.completed_at)
  if ($generatedAt -ne $completedAt -or $generatedAt -lt $runStarted) {
    throw "Frontend latency evidence timestamps are stale or inconsistent."
  }
  $attestationRelative = [System.IO.Path]::GetRelativePath(
    $repoRoot,
    $resolvedAttestation
  ).Replace("\", "/")
  if ("$($report.source.build_attestation_path)" -ne $attestationRelative) {
    throw "Frontend latency evidence references a different build attestation."
  }
  if (
    "$($report.policy.path)" -ne
    "scripts/frontend_interaction_latency_policy.json" -or
    "$($report.policy.sha256)" -ne $policySha256 -or
    "$($report.source.build_attestation_sha256)" -ne
    "$($attestationRead.publication.sha256)" -or
    "$($report.source.build_attestation_kind)" -ne
    "$($attestationRead.report.attestation_kind)" -or
    "$($report.source.build_attestation_generated_at)" -ne
    "$($attestationRead.report.generated_at)" -or
    "$($report.source.build_profile)" -ne
    "$($attestationRead.report.build_profile)" -or
    "$($report.source.build_input_sha256)" -ne
    "$($attestationRead.report.source_inputs.sha256)" -or
    "$($report.source.attested_build_input_sha256)" -ne
    "$($attestationRead.report.source_inputs.sha256)" -or
    "$($report.source.dist_sha256)" -ne
    "$($attestationRead.report.dist.sha256)" -or
    "$($report.source.attested_dist_sha256)" -ne
    "$($attestationRead.report.dist.sha256)"
  ) {
    throw "Frontend latency evidence does not match the policy and build attestation bytes."
  }
  $leaseRelative = [System.IO.Path]::GetRelativePath(
    $repoRoot,
    $authorityLease.path
  ).Replace("\", "/")
  $stableLeaseRead = Read-StableAuthorityJson -Path $authorityLease.path
  $expectedNonceSha = Get-Sha256Text -Value $authorityLease.nonce
  if (
    "$($report.source.benchmark_lock.lease_path)" -ne $leaseRelative -or
    "$($report.source.benchmark_lock.lease_sha256)" -ne
    "$($stableLeaseRead.publication.sha256)" -or
    "$($report.source.benchmark_lock.nonce_sha256)" -ne $expectedNonceSha -or
    [int]$report.source.benchmark_lock.owner_pid -ne $PID -or
    "$($report.source.benchmark_lock.owner_started_at)" -ne
    "$($ownership.wrapper_started_at)" -or
    [int]$report.source.benchmark_lock.mutex_owner_pid -ne $PID -or
    "$($report.source.benchmark_lock.mutex_owner_started_at)" -ne
    "$($ownership.wrapper_started_at)" -or
    "$($report.source.benchmark_lock.mutex_ownership_mode)" -ne "acquired" -or
    "$($report.source.benchmark_lock.mutex_name)" -ne $authorityMutexName -or
    [int]$report.source.benchmark_lock.benchmark_mutex_owner_pid -ne
    [int]$ownership.benchmark_mutex_owner_pid -or
    "$($report.source.benchmark_lock.benchmark_mutex_owner_started_at)" -ne
    "$($ownership.benchmark_mutex_owner_started_at)" -or
    "$($report.source.benchmark_lock.benchmark_mutex_ownership_mode)" -ne
    "$($ownership.benchmark_mutex_ownership_mode)" -or
    "$($report.source.benchmark_lock.benchmark_mutex_name)" -ne
    $benchmarkMutexName -or
    "$($report.source.benchmark_lock.benchmark_lease_path)" -ne
    "$($ownership.benchmark_lease_path)" -or
    "$($report.source.benchmark_lock.benchmark_lease_sha256)" -ne
    "$($ownership.benchmark_lease_sha256)" -or
    "$($report.source.benchmark_lock.benchmark_lease_identity)" -ne
    "$($ownership.benchmark_lease_identity)" -or
    [int]$report.source.benchmark_lock.benchmark_lease_write_probe_win32_error_code -ne
    32 -or
    [int]$report.source.benchmark_lock.benchmark_lease_write_probe_exit_code -ne
    0 -or
    -not [bool]$report.source.benchmark_lock.benchmark_lease_holder_verified -or
    [int]$report.source.benchmark_lock.benchmark_lease_holder_probe_exit_code -ne
    0 -or
    "$($report.source.benchmark_lock.token_sha256)" -ne
    "$($ownership.benchmark_lock_token_sha256)" -or
    -not [bool]$report.source.benchmark_lock.verified_at_start -or
    -not [bool]$report.source.benchmark_lock.verified_before_publication
  ) {
    throw "Frontend latency evidence does not prove the wrapper-held lease."
  }
  $snapshotRelative = [System.IO.Path]::GetRelativePath(
    $repoRoot,
    $resolvedDistSnapshot
  ).Replace("\", "/")
  $snapshotManifestRelative = [System.IO.Path]::GetRelativePath(
    $repoRoot,
    $snapshotManifestPath
  ).Replace("\", "/")
  if (
    "$($report.source.served_dist_snapshot_path)" -ne $snapshotRelative -or
    "$($report.source.served_dist_snapshot_manifest_path)" -ne
    $snapshotManifestRelative -or
    "$($report.source.served_dist_snapshot_manifest_sha256)" -ne
    "$($snapshotManifestRead.publication.sha256)" -or
    "$($report.source.served_dist_snapshot_sha256)" -ne
    "$($snapshotManifestRead.report.dist.sha256)" -or
    "$($report.source.served_dist_snapshot_path_manifest_sha256)" -ne
    "$($snapshotManifestRead.report.dist.path_manifest_sha256)" -or
    [int]$report.source.served_dist_snapshot_file_count -ne
    [int]$snapshotManifestRead.report.dist.file_count -or
    [long]$report.source.served_dist_snapshot_total_bytes -ne
    [long]$snapshotManifestRead.report.dist.total_bytes -or
    [int]$report.source.server_snapshot_entry_count -ne
    [int]$snapshotManifestRead.report.dist.file_count -or
    [long]$report.source.server_snapshot_total_bytes -ne
    [long]$snapshotManifestRead.report.dist.total_bytes -or
    "$($snapshotManifestRead.report.source_dist_sha256)" -ne $distSha -or
    -not [bool]$report.source.served_from_memory_snapshot
  ) {
    throw "Frontend latency evidence does not identify the sealed served snapshot."
  }
  Invoke-Step "Recompute and bind complete frontend latency evidence" {
    & node $reportValidator `
      --repo-root $repoRoot `
      --report $resolvedOut `
      --report-sha $runnerPublicationSha256 `
      --policy $policyPath `
      --policy-sha $policySha256 `
      --build-attestation $resolvedAttestation `
      --dist-snapshot $resolvedDistSnapshot `
      --lock-lease $authorityLease.path `
      --lock-nonce $authorityLease.nonce `
      --run-id $runId `
      --run-started-at $runStartedText
  }
  $finalReportRead = Read-StableAuthorityJson -Path $resolvedOut
  if (
    "$($finalReportRead.publication.sha256)" -ne
    "$($stableReportRead.publication.sha256)" -or
    "$($finalReportRead.publication.sha256)" -ne
    $runnerPublicationSha256
  ) {
    throw "Frontend latency evidence changed during final wrapper validation."
  }
  Write-Host "[PASS] Scoped frontend interaction latency evidence: $resolvedOut"
} finally {
  if ($null -ne $authorityLease) {
    try {
      $authorityLease.stream.Dispose()
    } catch {
    }
  }
  if ($null -ne $authorityMutex) {
    Exit-FrontendAuthorityMutex -Mutex $authorityMutex
  }
  if ($null -ne $lock) {
    Exit-BenchmarkLock $lock
  }
  Pop-Location
}
