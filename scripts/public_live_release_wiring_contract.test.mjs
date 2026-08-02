import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import test from 'node:test'
import { fileURLToPath } from 'node:url'
import { validateAuthorityMatrix } from './check_release_authority_matrix.mjs'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.resolve(scriptDir, '..')
const candidatePath = path.join(scriptDir, 'run_release_candidate_gate.ps1')
const publicPath = path.join(scriptDir, 'run_public_release_gate.ps1')
const executionSafetyPath = path.join(scriptDir, 'run_execution_safety_gate.ps1')
const matrixPath = path.join(repoRoot, 'docs', 'release_authority_matrix.json')
const candidate = fs.readFileSync(candidatePath, 'utf8')
const publicGate = fs.readFileSync(publicPath, 'utf8')
const executionSafety = fs.readFileSync(executionSafetyPath, 'utf8')
const matrix = JSON.parse(fs.readFileSync(matrixPath, 'utf8'))

function parsePowerShell(scriptPath) {
  const source = String.raw`
$tokens = $null
$errors = $null
[System.Management.Automation.Language.Parser]::ParseFile(
  $env:VECTORGRID_SCRIPT_UNDER_TEST,
  [ref]$tokens,
  [ref]$errors
) | Out-Null
if ($errors.Count -gt 0) {
  $errors | ForEach-Object { [Console]::Error.WriteLine($_.Message) }
  exit 1
}
`
  const encoded = Buffer.from(source, 'utf16le').toString('base64')
  return spawnSync(
    'powershell.exe',
    ['-NoProfile', '-NonInteractive', '-EncodedCommand', encoded],
    {
      cwd: repoRoot,
      encoding: 'utf8',
      env: {
        ...process.env,
        VECTORGRID_SCRIPT_UNDER_TEST: scriptPath,
      },
      shell: false,
      windowsHide: true,
    },
  )
}

function runPublicSourceStabilityProbe(repositoryPath) {
  const source = String.raw`
$ErrorActionPreference = "Stop"
$WarningPreference = "SilentlyContinue"
$tokens = $null
$errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
  $env:VECTORGRID_PUBLIC_GATE_PATH,
  [ref]$tokens,
  [ref]$errors
)
if ($errors.Count -gt 0) {
  throw "Public gate parser errors: $($errors -join '; ')"
}
$requiredFunctions = @(
  "Get-BytesSha256",
  "Assert-PublicReleaseSourceHasSingleLink",
  "Get-FileSha256Identity",
  "Invoke-PublicReleaseGitQuery",
  "ConvertFrom-PublicReleaseGitNulList",
  "Assert-PublicReleaseManifestPathSafe",
  "Assert-PublicReleasePathHasNoReparseAncestor",
  "Get-PublicReleaseRawFileManifest",
  "Get-PublicReleaseTrackedInventory",
  "Get-PublicReleaseUntrackedManifest",
  "Get-PublicReleaseSourceSnapshot",
  "Assert-PublicReleaseSourceSnapshotStable",
  "Assert-PublicReleaseSourceReadLeasesActive",
  "Close-PublicReleaseSourceReadLeases",
  "Test-PublicReleasePathUnderOrEqualRoot",
  "Get-PublicReleaseSourceMutationExcludedRoots",
  "Test-PublicReleaseSourceMutationPathExcluded",
  "Get-PublicReleaseSourceMutationRelativePath",
  "Start-PublicReleaseSourceMutationMonitor",
  "Complete-PublicReleaseSourceMutationMonitor",
  "Assert-PublicReleaseSourceMutationMonitorClean",
  "Assert-CleanWorktree",
  "Get-PublicReleaseSourceSnapshotEvidence",
  "Get-PublicReleaseSourceMutationMonitorEvidence",
  "Test-PublicReleaseEligibility"
)
$functionDefinitions = @($ast.FindAll({
  param($node)
  $node -is [System.Management.Automation.Language.FunctionDefinitionAst]
}, $true))
foreach ($name in $requiredFunctions) {
  $matches = @($functionDefinitions | Where-Object { $_.Name -eq $name })
  if ($matches.Count -ne 1) {
    throw "Expected one public-gate function '$name', found $($matches.Count)."
  }
  Invoke-Expression $matches[0].Extent.Text
}

$repo = [System.IO.Path]::GetFullPath(
  $env:VECTORGRID_SOURCE_STABILITY_REPO
)
$git = [System.IO.Path]::GetFullPath(
  [string](Get-Command git.exe -CommandType Application -ErrorAction Stop |
    Select-Object -First 1).Source
)
& $git -C $repo init --quiet
if ($LASTEXITCODE -ne 0) { throw "git init failed" }
& $git -C $repo config user.email "source-stability@example.invalid"
if ($LASTEXITCODE -ne 0) { throw "git config email failed" }
& $git -C $repo config user.name "Source Stability Contract"
if ($LASTEXITCODE -ne 0) { throw "git config name failed" }
$trackedPath = Join-Path $repo "tracked.txt"
$initialTrackedBytes = [System.Text.Encoding]::UTF8.GetBytes(
  "initial" + [Environment]::NewLine
)
[System.IO.File]::WriteAllText(
  $trackedPath,
  ("initial" + [Environment]::NewLine),
  (New-Object System.Text.UTF8Encoding($false))
)
& $git -C $repo add -- tracked.txt
if ($LASTEXITCODE -ne 0) { throw "git add failed" }
& $git -C $repo commit --quiet -m "initial"
if ($LASTEXITCODE -ne 0) { throw "git commit failed" }

$cleanInitial = Get-PublicReleaseSourceSnapshot -GitExecutablePath $git -RepositoryRoot $repo
$cleanFinal = Get-PublicReleaseSourceSnapshot -GitExecutablePath $git -RepositoryRoot $repo
Assert-PublicReleaseSourceSnapshotStable -Initial $cleanInitial -Final $cleanFinal

$script:InitialSourceSnapshot = $cleanInitial
$script:FinalSourceSnapshot = $cleanFinal
$script:SourceStateRevalidated = $true
$passedMonitorEvidence = [pscustomobject][ordered]@{
  schema_version = 1
  status = "passed"
  started_at_utc = "2026-01-01T00:00:00.0000000Z"
  completed_at_utc = "2026-01-01T00:00:01.0000000Z"
  include_subdirectories = $true
  internal_buffer_size_bytes = 65536
  excluded_root_count = 1
  event_count = 0
  error_count = 0
  event_sha256 = Get-BytesSha256 -Bytes ([byte[]]@())
}
$script:SourceMutationMonitorEvidence = $passedMonitorEvidence
$script:SourceMutationMonitorRevalidated = $true
$script:SourceReadLeaseRevalidated = $true
$AllowDirtyWorktree = $false
$AllowUnsignedBeta = $false
$cleanEligible = Test-PublicReleaseEligibility -Status "passed"
$script:SourceStateRevalidated = $false
$missingFinalRevalidationEligible =
  Test-PublicReleaseEligibility -Status "passed"
$script:SourceStateRevalidated = $true

$AllowDirtyWorktree = $true
$dirtyAllowancePassedEligible =
  Test-PublicReleaseEligibility -Status "passed"
$AllowDirtyWorktree = $false
$AllowUnsignedBeta = $true
$unsignedAllowancePassedEligible =
  Test-PublicReleaseEligibility -Status "passed"
$AllowUnsignedBeta = $false
$script:SourceMutationMonitorRevalidated = $false
$missingMonitorRevalidationEligible =
  Test-PublicReleaseEligibility -Status "passed"
$script:SourceMutationMonitorRevalidated = $true
$script:SourceReadLeaseRevalidated = $false
$missingSourceLeaseRevalidationEligible =
  Test-PublicReleaseEligibility -Status "passed"
$script:SourceReadLeaseRevalidated = $true
$script:SourceMutationMonitorEvidence = [pscustomobject][ordered]@{
  schema_version = 1
  status = "mutated"
  event_count = 1
  error_count = 0
}
$mutatedMonitorPassedEligible =
  Test-PublicReleaseEligibility -Status "passed"
$script:SourceMutationMonitorEvidence = $passedMonitorEvidence

& $git -C $repo update-index --assume-unchanged -- tracked.txt
if ($LASTEXITCODE -ne 0) { throw "assume-unchanged setup failed" }
[System.IO.File]::WriteAllText(
  $trackedPath,
  ("hidden" + [Environment]::NewLine),
  (New-Object System.Text.UTF8Encoding($false))
)
$hiddenIndexMutationRejected = $false
try {
  Get-PublicReleaseSourceSnapshot -GitExecutablePath $git -RepositoryRoot $repo |
    Out-Null
} catch {
  $hiddenIndexMutationRejected = $true
} finally {
  [System.IO.File]::WriteAllBytes($trackedPath, $initialTrackedBytes)
  & $git -C $repo update-index --no-assume-unchanged -- tracked.txt
  if ($LASTEXITCODE -ne 0) {
    throw "assume-unchanged teardown failed"
  }
}
if (-not $hiddenIndexMutationRejected) {
  throw "A permanently changed assume-unchanged tracked file was accepted."
}

$externalAliasPath = Join-Path (
  Split-Path -Parent $repo
) ("vectorgrid-external-hardlink-" + [guid]::NewGuid().ToString("N") + ".txt")
New-Item -ItemType HardLink -Path $externalAliasPath -Target $trackedPath |
  Out-Null
$preexistingHardLinkRejected = $false
try {
  Get-PublicReleaseSourceSnapshot -GitExecutablePath $git -RepositoryRoot $repo |
    Out-Null
} catch {
  $preexistingHardLinkRejected = $true
} finally {
  if (Test-Path -LiteralPath $externalAliasPath) {
    [System.IO.File]::Delete($externalAliasPath)
  }
}
if (-not $preexistingHardLinkRejected) {
  throw "A tracked source with an external hard-link alias was accepted."
}

$leasedSnapshot = Get-PublicReleaseSourceSnapshot -GitExecutablePath $git -RepositoryRoot $repo -RetainSourceReadLeases
Assert-PublicReleaseSourceReadLeasesActive -SourceSnapshot $leasedSnapshot
$externalAliasWriteBlocked = $false
try {
  try {
    New-Item -ItemType HardLink -Path $externalAliasPath -Target $trackedPath |
      Out-Null
  } catch {
    $externalAliasWriteBlocked = $true
  }
  if (Test-Path -LiteralPath $externalAliasPath) {
    try {
      [System.IO.File]::WriteAllText(
        $externalAliasPath,
        ("external-alias-write" + [Environment]::NewLine),
        (New-Object System.Text.UTF8Encoding($false))
      )
    } catch {
      $externalAliasWriteBlocked = $true
    }
  }
} finally {
  Close-PublicReleaseSourceReadLeases -SourceSnapshot $leasedSnapshot
  [System.IO.File]::WriteAllBytes($trackedPath, $initialTrackedBytes)
  if (Test-Path -LiteralPath $externalAliasPath) {
    [System.IO.File]::Delete($externalAliasPath)
  }
}
if (-not $externalAliasWriteBlocked) {
  throw "A retained source read lease allowed an external-alias write."
}
$hardLinkCycleFinal = Get-PublicReleaseSourceSnapshot -GitExecutablePath $git -RepositoryRoot $repo
Assert-PublicReleaseSourceSnapshotStable -Initial $cleanInitial -Final $hardLinkCycleFinal

$monitorExclusions = @(
  [System.IO.Path]::GetFullPath((Join-Path $repo ".git"))
)
$generatedRoot = Join-Path $repo "target"
New-Item -ItemType Directory -Force -Path $generatedRoot | Out-Null
$generatedExclusions = Get-PublicReleaseSourceMutationExcludedRoots -RepositoryRoot $repo -ReleaseOutputDirectory (Join-Path $repo "output")
$generatedMonitor = Start-PublicReleaseSourceMutationMonitor -RepositoryRoot $repo -ExcludedRoots $generatedExclusions
$generatedProbePath = Join-Path $generatedRoot "generated.txt"
[System.IO.File]::WriteAllText(
  $generatedProbePath,
  ("generated" + [Environment]::NewLine),
  (New-Object System.Text.UTF8Encoding($false))
)
[System.IO.File]::Delete($generatedProbePath)
$generatedEvidence = Complete-PublicReleaseSourceMutationMonitor -Monitor $generatedMonitor
Assert-PublicReleaseSourceMutationMonitorClean -Evidence $generatedEvidence
$generatedRootExcluded = (
  [string]$generatedEvidence.status -eq "passed" -and
  [int]$generatedEvidence.event_count -eq 0
)

$watcherErrorRejected = $false
try {
  Assert-PublicReleaseSourceMutationMonitorClean -Evidence ([pscustomobject]@{
    schema_version = 1
    status = "error"
    event_count = 0
    error_count = 1
  })
} catch {
  $watcherErrorRejected = $true
}
if (-not $watcherErrorRejected) {
  throw "A FileSystemWatcher error/overflow state was accepted."
}

$trackedRestoreMonitor = Start-PublicReleaseSourceMutationMonitor -RepositoryRoot $repo -ExcludedRoots $monitorExclusions
[System.IO.File]::WriteAllText(
  $trackedPath,
  ("transient" + [Environment]::NewLine),
  (New-Object System.Text.UTF8Encoding($false))
)
[System.IO.File]::WriteAllBytes($trackedPath, $initialTrackedBytes)
$trackedRestoreEvidence = Complete-PublicReleaseSourceMutationMonitor -Monitor $trackedRestoreMonitor
$trackedRestoreRejected = $false
try {
  Assert-PublicReleaseSourceMutationMonitorClean -Evidence $trackedRestoreEvidence
} catch {
  $trackedRestoreRejected = $true
}
if (-not $trackedRestoreRejected) {
  throw "A tracked modify-restore cycle was accepted by the mutation monitor."
}
$trackedRestoreFinal = Get-PublicReleaseSourceSnapshot -GitExecutablePath $git -RepositoryRoot $repo
Assert-PublicReleaseSourceSnapshotStable -Initial $cleanInitial -Final $trackedRestoreFinal

$untrackedRestoreMonitor = Start-PublicReleaseSourceMutationMonitor -RepositoryRoot $repo -ExcludedRoots $monitorExclusions
$transientUntrackedPath = Join-Path $repo "transient-untracked.txt"
[System.IO.File]::WriteAllText(
  $transientUntrackedPath,
  ("transient" + [Environment]::NewLine),
  (New-Object System.Text.UTF8Encoding($false))
)
[System.IO.File]::Delete($transientUntrackedPath)
$untrackedRestoreEvidence = Complete-PublicReleaseSourceMutationMonitor -Monitor $untrackedRestoreMonitor
$untrackedRestoreRejected = $false
try {
  Assert-PublicReleaseSourceMutationMonitorClean -Evidence $untrackedRestoreEvidence
} catch {
  $untrackedRestoreRejected = $true
}
if (-not $untrackedRestoreRejected) {
  throw "An untracked create-remove cycle was accepted by the mutation monitor."
}
$untrackedRestoreFinal = Get-PublicReleaseSourceSnapshot -GitExecutablePath $git -RepositoryRoot $repo
Assert-PublicReleaseSourceSnapshotStable -Initial $cleanInitial -Final $untrackedRestoreFinal

[System.IO.File]::AppendAllText(
  $trackedPath,
  ("changed" + [Environment]::NewLine)
)
$dirtyInitial = Get-PublicReleaseSourceSnapshot -GitExecutablePath $git -RepositoryRoot $repo
$trackedMutationRejected = $false
try {
  Assert-PublicReleaseSourceSnapshotStable -Initial $cleanInitial -Final $dirtyInitial
} catch {
  $trackedMutationRejected = $true
}
if (-not $trackedMutationRejected) {
  throw "Tracked source mutation was accepted."
}

$dirtyUnchanged = Get-PublicReleaseSourceSnapshot -GitExecutablePath $git -RepositoryRoot $repo
Assert-PublicReleaseSourceSnapshotStable -Initial $dirtyInitial -Final $dirtyUnchanged
$AllowDirtyWorktree = $true
$AllowUnsignedBeta = $true
Assert-CleanWorktree -SourceSnapshot $dirtyInitial
$script:InitialSourceSnapshot = $dirtyInitial
$script:FinalSourceSnapshot = $dirtyUnchanged
$script:SourceStateRevalidated = $true
$dirtyRehearsalEligible =
  Test-PublicReleaseEligibility -Status "rehearsal-passed"
$AllowDirtyWorktree = $false
$AllowUnsignedBeta = $false
$dirtySnapshotPassedEligible =
  Test-PublicReleaseEligibility -Status "passed"

$unstableFinal = $cleanFinal |
  ConvertTo-Json -Depth 12 |
  ConvertFrom-Json
$unstableFinal.source_state_sha256 = (("0" * 64) -join "")
$script:InitialSourceSnapshot = $cleanInitial
$script:FinalSourceSnapshot = $unstableFinal
$unstableSnapshotPassedEligible =
  Test-PublicReleaseEligibility -Status "passed"
$script:InitialSourceSnapshot = $dirtyInitial
$script:FinalSourceSnapshot = $dirtyUnchanged
$AllowDirtyWorktree = $true
$AllowUnsignedBeta = $true

[System.IO.File]::AppendAllText(
  $trackedPath,
  ("changed-again" + [Environment]::NewLine)
)
$dirtyChanged = Get-PublicReleaseSourceSnapshot -GitExecutablePath $git -RepositoryRoot $repo
$dirtyMutationRejected = $false
try {
  Assert-PublicReleaseSourceSnapshotStable -Initial $dirtyInitial -Final $dirtyChanged
} catch {
  $dirtyMutationRejected = $true
}
if (-not $dirtyMutationRejected) {
  throw "A changed dirty rehearsal snapshot was accepted."
}

& $git -C $repo add -- tracked.txt
if ($LASTEXITCODE -ne 0) { throw "second git add failed" }
& $git -C $repo commit --quiet -m "new head"
if ($LASTEXITCODE -ne 0) { throw "second git commit failed" }
$newHeadClean = Get-PublicReleaseSourceSnapshot -GitExecutablePath $git -RepositoryRoot $repo
$headMutationRejected = $false
try {
  Assert-PublicReleaseSourceSnapshotStable -Initial $cleanInitial -Final $newHeadClean
} catch {
  $headMutationRejected = $true
}
if (-not $headMutationRejected) {
  throw "A changed HEAD with a clean status was accepted."
}

$untrackedPath = Join-Path $repo "untracked.txt"
[System.IO.File]::WriteAllText(
  $untrackedPath,
  ("untracked" + [Environment]::NewLine),
  (New-Object System.Text.UTF8Encoding($false))
)
$untrackedInitial = Get-PublicReleaseSourceSnapshot -GitExecutablePath $git -RepositoryRoot $repo
[System.IO.File]::AppendAllText(
  $untrackedPath,
  ("changed" + [Environment]::NewLine)
)
$untrackedChanged = Get-PublicReleaseSourceSnapshot -GitExecutablePath $git -RepositoryRoot $repo
$untrackedMutationRejected = $false
try {
  Assert-PublicReleaseSourceSnapshotStable -Initial $untrackedInitial -Final $untrackedChanged
} catch {
  $untrackedMutationRejected = $true
}
if (-not $untrackedMutationRejected) {
  throw "Changed content under an already-untracked path was accepted."
}

$snapshotEvidence = Get-PublicReleaseSourceSnapshotEvidence -Snapshot $untrackedChanged
$snapshotEvidenceSanitized = (
  $null -eq $snapshotEvidence.PSObject.Properties["status_base64"] -and
  $null -eq $snapshotEvidence.PSObject.Properties["tracked_diff_base64"] -and
  $null -eq $snapshotEvidence.PSObject.Properties["tracked_index_base64"] -and
  $null -eq $snapshotEvidence.PSObject.Properties["tracked_manifest_base64"] -and
  $null -eq $snapshotEvidence.PSObject.Properties["untracked_manifest_base64"] -and
  $null -eq $snapshotEvidence.PSObject.Properties["source_read_leases"]
)
$monitorSummaryEvidence =
  Get-PublicReleaseSourceMutationMonitorEvidence -Evidence $trackedRestoreEvidence
$monitorEvidenceSanitized = (
  $null -eq $monitorSummaryEvidence.PSObject.Properties["paths"] -and
  $null -eq $monitorSummaryEvidence.PSObject.Properties["events"] -and
  $null -eq $monitorSummaryEvidence.PSObject.Properties["excluded_roots"]
)

[pscustomobject][ordered]@{
  clean_status_size_bytes = [int64]$cleanInitial.status_size_bytes
  clean_tracked_manifest_file_count =
    [int]$cleanInitial.tracked_manifest_file_count
  clean_stable = $true
  clean_eligible = [bool]$cleanEligible
  missing_final_revalidation_eligible =
    [bool]$missingFinalRevalidationEligible
  missing_monitor_revalidation_eligible =
    [bool]$missingMonitorRevalidationEligible
  missing_source_lease_revalidation_eligible =
    [bool]$missingSourceLeaseRevalidationEligible
  dirty_allowance_passed_eligible =
    [bool]$dirtyAllowancePassedEligible
  unsigned_allowance_passed_eligible =
    [bool]$unsignedAllowancePassedEligible
  mutated_monitor_passed_eligible =
    [bool]$mutatedMonitorPassedEligible
  dirty_snapshot_passed_eligible =
    [bool]$dirtySnapshotPassedEligible
  unstable_snapshot_passed_eligible =
    [bool]$unstableSnapshotPassedEligible
  hidden_index_mutation_rejected =
    [bool]$hiddenIndexMutationRejected
  preexisting_hard_link_rejected =
    [bool]$preexistingHardLinkRejected
  external_alias_write_blocked =
    [bool]$externalAliasWriteBlocked
  generated_root_excluded = [bool]$generatedRootExcluded
  watcher_error_rejected = [bool]$watcherErrorRejected
  tracked_restore_monitor_rejected =
    [bool]$trackedRestoreRejected
  untracked_restore_monitor_rejected =
    [bool]$untrackedRestoreRejected
  tracked_mutation_rejected = [bool]$trackedMutationRejected
  dirty_unchanged_accepted = $true
  dirty_rehearsal_eligible = [bool]$dirtyRehearsalEligible
  dirty_mutation_rejected = [bool]$dirtyMutationRejected
  head_mutation_rejected = [bool]$headMutationRejected
  untracked_mutation_rejected = [bool]$untrackedMutationRejected
  snapshot_evidence_sanitized = [bool]$snapshotEvidenceSanitized
  monitor_evidence_sanitized = [bool]$monitorEvidenceSanitized
} | ConvertTo-Json -Compress
`
  const probeDirectory = fs.mkdtempSync(
    path.join(os.tmpdir(), 'vectorgrid-public-source-probe-script-'),
  )
  const probePath = path.join(probeDirectory, 'probe.ps1')
  fs.writeFileSync(probePath, source, 'utf8')
  try {
    return spawnSync(
      'powershell.exe',
      ['-NoProfile', '-NonInteractive', '-File', probePath],
      {
        cwd: repoRoot,
        encoding: 'utf8',
        env: {
          ...process.env,
          VECTORGRID_PUBLIC_GATE_PATH: publicPath,
          VECTORGRID_SOURCE_STABILITY_REPO: repositoryPath,
        },
        shell: false,
        windowsHide: true,
      },
    )
  } finally {
    fs.rmSync(probeDirectory, { force: true, recursive: true })
  }
}

test('public release unconditionally routes both live-execution safety authorities', () => {
  assert.match(candidate, /\[switch\]\$IncludeExecutionSafetyGate/)
  assert.match(candidate, /\[switch\]\$IncludeLiveSafetyHarness/)
  assert.match(
    candidate,
    /if\s*\(\$IncludeExecutionSafetyGate\)\s*\{[\s\S]*?run_execution_safety_gate\.ps1[\s\S]*?Invoke-ReleaseStep\s+-Name\s+"Execution safety gate"/,
  )
  assert.match(
    candidate,
    /if\s*\(\$IncludeLiveSafetyHarness\)\s*\{[\s\S]*?run_live_execution_safety_harness\.ps1[\s\S]*?Invoke-ReleaseStep\s+-Name\s+"Live execution safety harness"/,
  )
  assert.match(
    candidate,
    /\$liveArgs\s*=\s*@\([\s\S]*?"-NoProfile"[\s\S]*?"-ExecutionPolicy",\s*"Bypass"[\s\S]*?run_live_execution_safety_harness\.ps1/,
  )
  assert.match(
    publicGate,
    /\$candidateArgs\s*\+=\s*@\("-IncludeExecutionSafetyGate",\s*"-IncludeLiveSafetyHarness"\)/,
  )
  assert.match(publicGate, /public_live_safety_evidence_contract\.mjs/)
  assert.match(publicGate, /vectorgrid-public-live-safety-evidence-v1/)
  assert.doesNotMatch(publicGate, /\[switch\]\$Skip(?:Execution|Live)Safety/)
  assert.match(candidate, /include_execution_safety_gate\s*=\s*\[bool\]\$IncludeExecutionSafetyGate/)
  assert.match(candidate, /include_live_safety_harness\s*=\s*\[bool\]\$IncludeLiveSafetyHarness/)
})

test('public release monitors and revalidates raw source state before eligibility', () => {
  const monitorStart = publicGate.lastIndexOf(
    'Start-PublicReleaseSourceMutationMonitor',
  )
  const initialSnapshot = publicGate.indexOf(
    '$script:InitialSourceSnapshot = Get-PublicReleaseSourceSnapshot',
  )
  const candidateGate = publicGate.indexOf(
    'Invoke-PublicReleaseStep -Name "Release candidate gate"',
  )
  const finalSnapshot = publicGate.indexOf(
    '$script:FinalSourceSnapshot = Get-PublicReleaseSourceSnapshot',
  )
  const monitorComplete = publicGate.indexOf(
    '$script:SourceMutationMonitorEvidence =\n      Complete-PublicReleaseSourceMutationMonitor',
  )
  const completedSummary = publicGate.indexOf(
    'Write-PublicReleaseSummary -Status $completedStatus',
  )
  assert.ok(monitorStart >= 0)
  assert.ok(initialSnapshot > monitorStart)
  assert.ok(candidateGate > initialSnapshot)
  assert.ok(finalSnapshot > candidateGate)
  assert.ok(monitorComplete > finalSnapshot)
  assert.ok(completedSummary > monitorComplete)
  assert.match(
    publicGate,
    /status --porcelain=v1 -z --untracked-files=all -- \./,
  )
  assert.match(publicGate, /ls-files -v -z --cached -- \./)
  assert.match(publicGate, /Get-PublicReleaseRawFileManifest/)
  assert.match(publicGate, /-RetainSourceReadLeases/)
  assert.match(publicGate, /Assert-PublicReleaseSourceHasSingleLink/)
  assert.match(publicGate, /Assert-PublicReleaseSourceReadLeasesActive/)
  assert.match(
    publicGate,
    /foreach \(\$eventName in @\("Changed", "Created", "Deleted", "Renamed", "Error"\)\)/,
  )
  assert.match(
    publicGate,
    /Assert-PublicReleaseSourceMutationMonitorClean[\s\S]*?Assert-PublicReleaseSourceSnapshotStable[\s\S]*?\$script:SourceStateRevalidated = \$true/,
  )
  assert.match(
    publicGate,
    /function Test-PublicReleaseEligibility[\s\S]*?\[bool\]\$script:SourceStateRevalidated[\s\S]*?\[bool\]\$script:SourceMutationMonitorRevalidated[\s\S]*?\[bool\]\$script:SourceReadLeaseRevalidated/,
  )
})

test('public source stability binds raw files and rejects hidden and restored mutations', (t) => {
  const temporaryRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), 'vectorgrid-public-source-stability-'),
  )
  t.after(() => {
    fs.rmSync(temporaryRoot, { force: true, recursive: true })
  })
  const result = runPublicSourceStabilityProbe(temporaryRoot)
  assert.equal(
    result.status,
    0,
    `source-stability probe failed:\n${String(
      result.stderr || result.stdout,
    ).trim()}`,
  )
  const report = JSON.parse(result.stdout.trim())
  assert.equal(report.clean_status_size_bytes, 0)
  assert.equal(report.clean_tracked_manifest_file_count, 1)
  assert.equal(report.clean_stable, true)
  assert.equal(report.clean_eligible, true)
  assert.equal(report.missing_final_revalidation_eligible, false)
  assert.equal(report.missing_monitor_revalidation_eligible, false)
  assert.equal(report.missing_source_lease_revalidation_eligible, false)
  assert.equal(report.dirty_allowance_passed_eligible, false)
  assert.equal(report.unsigned_allowance_passed_eligible, false)
  assert.equal(report.mutated_monitor_passed_eligible, false)
  assert.equal(report.dirty_snapshot_passed_eligible, false)
  assert.equal(report.unstable_snapshot_passed_eligible, false)
  assert.equal(report.hidden_index_mutation_rejected, true)
  assert.equal(report.preexisting_hard_link_rejected, true)
  assert.equal(report.external_alias_write_blocked, true)
  assert.equal(report.generated_root_excluded, true)
  assert.equal(report.watcher_error_rejected, true)
  assert.equal(report.tracked_restore_monitor_rejected, true)
  assert.equal(report.untracked_restore_monitor_rejected, true)
  assert.equal(report.tracked_mutation_rejected, true)
  assert.equal(report.dirty_unchanged_accepted, true)
  assert.equal(report.dirty_rehearsal_eligible, false)
  assert.equal(report.dirty_mutation_rejected, true)
  assert.equal(report.head_mutation_rejected, true)
  assert.equal(report.untracked_mutation_rejected, true)
  assert.equal(report.snapshot_evidence_sanitized, true)
  assert.equal(report.monitor_evidence_sanitized, true)
})

test('all public compute variants require both safety evidence sources in the matrix', () => {
  for (const variantId of ['cpu', 'gpu-ma', 'gpu-kernel']) {
    const variant = matrix.release_variants.find((row) => row.id === variantId)
    assert.ok(variant, `missing ${variantId}`)
    assert.ok(variant.required_claims.includes('live-execution-safety'))
    assert.ok(variant.required_sources.includes('execution_safety_gate'))
    assert.ok(variant.required_sources.includes('live_safety_harness'))
  }

  const errors = validateAuthorityMatrix({
    matrix,
    repoRoot,
    matrixRelativePath: 'docs/release_authority_matrix.json',
  })
  assert.deepEqual(errors, [])

  const weakened = structuredClone(matrix)
  const cpu = weakened.release_variants.find((row) => row.id === 'cpu')
  cpu.required_sources = cpu.required_sources.filter(
    (source) => source !== 'live_safety_harness',
  )
  const weakenedErrors = validateAuthorityMatrix({
    matrix: weakened,
    repoRoot,
    matrixRelativePath: 'docs/release_authority_matrix.json',
  })
  assert.ok(
    weakenedErrors.some((message) =>
      message.includes('must require live_safety_harness')),
  )
})

test('live execution is shipped rather than feature-gated in every desktop variant', () => {
  const tauriManifest = fs.readFileSync(
    path.join(repoRoot, 'src-tauri', 'Cargo.toml'),
    'utf8',
  )
  const lib = fs.readFileSync(
    path.join(repoRoot, 'src-tauri', 'src', 'lib.rs'),
    'utf8',
  )
  const panel = fs.readFileSync(
    path.join(repoRoot, 'web', 'src', 'components', 'ExecutionWorkspacePanel.vue'),
    'utf8',
  )
  const store = fs.readFileSync(
    path.join(repoRoot, 'web', 'src', 'stores', 'executionStore.ts'),
    'utf8',
  )

  assert.doesNotMatch(tauriManifest, /^live-execution\s*=/m)
  for (const command of [
    'execute_strategy_signal_once',
    'cancel_live_execution_open_orders',
    'flatten_live_execution_position',
    'emergency_halt_live_execution',
    'start_live_execution_runner',
    'stop_live_execution_runner',
  ]) {
    assert.match(lib, new RegExp(`\\n\\s*${command},`))
  }
  assert.match(panel, /<option\s+value="alpaca_live">Alpaca live<\/option>/)
  assert.match(store, /executionTarget:\s*["']dry_run["']/)
})

test('both safety gates clear every reviewed production credential environment name', () => {
  const liveHarness = fs.readFileSync(
    path.join(scriptDir, 'run_live_execution_safety_harness.ps1'),
    'utf8',
  )
  for (const credentialName of [
    'ALPACA_API_KEY',
    'ALPACA_API_KEY_ID',
    'ALPACA_API_SECRET',
    'ALPACA_API_SECRET_KEY',
    'ALPACA_KEY',
    'ALPACA_KEY_ID',
    'ALPACA_SECRET',
    'ALPACA_SECRET_KEY',
    'ALPACA_API_BASE_URL',
    'APCA_API_KEY',
    'APCA_API_KEY_ID',
    'APCA_API_SECRET',
    'APCA_API_SECRET_KEY',
    'APCA_API_BASE_URL',
    'POLYGON_KEY',
    'POLYGON_API_KEY',
    'POLYGON_TOKEN',
    'POLYGON_API_BASE_URL',
  ]) {
    assert.ok(executionSafety.includes(`"${credentialName}"`), credentialName)
    assert.ok(liveHarness.includes(`"${credentialName}"`), credentialName)
  }
})

test('release and execution-safety PowerShell scripts remain parser-clean', () => {
  for (const scriptPath of [candidatePath, publicPath, executionSafetyPath]) {
    const result = parsePowerShell(scriptPath)
    assert.equal(
      result.status,
      0,
      `${path.basename(scriptPath)} parser errors:\n${String(
        result.stderr || result.stdout,
      ).trim()}`,
    )
  }
})
