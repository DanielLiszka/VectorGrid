import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
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
