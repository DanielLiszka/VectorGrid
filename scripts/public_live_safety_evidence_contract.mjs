#!/usr/bin/env node

import crypto from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const defaultRepoRoot = path.resolve(scriptDir, '..')

const CONTRACT_ID = 'vectorgrid-public-live-safety-evidence-v1'
const REQUIRED_TEST_COUNT = 128
const ALLOWED_VARIANTS = new Set(['cpu', 'gpu-ma', 'gpu-kernel'])
const SUMMARY_FILE = 'live-safety-summary.json'
const STDOUT_FILE = 'live-safety-cargo.stdout.log'
const STDERR_FILE = 'live-safety-cargo.stderr.log'
const CANDIDATE_SUMMARY_FILE = 'summary.json'

const REQUIRED_CARGO_ARGUMENTS = [
  'test',
  '--locked',
  '--offline',
  '-p',
  'ta_desktop_demo_app',
  '--no-default-features',
  '--features',
  'desktop-ui,custom-protocol',
  '--lib',
  'execution',
  '--',
  '--test-threads=1',
]

const REQUIRED_CREDENTIAL_ENVIRONMENT_NAMES = [
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
]

const SOURCE_CONTRACT = [
  ['harness', 'scripts/run_live_execution_safety_harness.ps1'],
  ['commands_execution', 'src-tauri/src/commands_execution.rs'],
  ['execution_policy', 'src-tauri/src/execution_policy.rs'],
  ['execution_mocks', 'src-tauri/src/execution_mocks.rs'],
  ['execution_quantity', 'src-tauri/src/execution_quantity.rs'],
  ['settings', 'src-tauri/src/settings.rs'],
  ['run_artifacts', 'src-tauri/src/run_artifacts.rs'],
  ['tauri_manifest', 'src-tauri/Cargo.toml'],
  ['app_core_manifest', 'crates/app_core/Cargo.toml'],
  ['cargo_lock', 'Cargo.lock'],
]

const COVERAGE_CATEGORIES = [
  'commands_execution_complete',
  'execution_policy_complete',
  'execution_mocks_quantity_and_settings',
]

function fail(message) {
  throw new Error(message)
}

function normalizedPath(value) {
  let full = path.resolve(String(value ?? ''))
  if (process.platform !== 'win32') return full
  full = full.replace(/\//g, '\\')
  if (full.startsWith('\\\\?\\UNC\\')) full = `\\\\${full.slice(8)}`
  else if (full.startsWith('\\\\?\\')) full = full.slice(4)
  return full.toLowerCase()
}

function samePath(left, right) {
  return normalizedPath(left) === normalizedPath(right)
}

function sha256Bytes(bytes) {
  return crypto.createHash('sha256').update(bytes).digest('hex')
}

function requireOrdinaryDirectory(dirPath, label) {
  let stat
  try {
    stat = fs.lstatSync(dirPath)
  } catch {
    fail(`${label} is missing: ${dirPath}`)
  }
  if (!stat.isDirectory() || stat.isSymbolicLink()) {
    fail(`${label} must be an ordinary directory: ${dirPath}`)
  }
  return stat
}

function requireOrdinaryFile(filePath, label) {
  let stat
  try {
    stat = fs.lstatSync(filePath)
  } catch {
    fail(`${label} is missing: ${filePath}`)
  }
  if (!stat.isFile() || stat.isSymbolicLink()) {
    fail(`${label} must be an ordinary file: ${filePath}`)
  }
  return stat
}

function readOrdinaryFileSnapshot(
  filePath,
  label,
  { maximumBytes = Number.MAX_SAFE_INTEGER, allowEmpty = false } = {},
) {
  const beforeOpen = requireOrdinaryFile(filePath, label)
  let handle
  try {
    handle = fs.openSync(filePath, 'r')
    const opened = fs.fstatSync(handle)
    if (
      !opened.isFile()
      || opened.dev !== beforeOpen.dev
      || opened.ino !== beforeOpen.ino
    ) {
      fail(`${label} changed while it was being opened: ${filePath}`)
    }
    if (
      (!allowEmpty && opened.size <= 0)
      || opened.size < 0
      || opened.size > maximumBytes
    ) {
      fail(`${label} has an invalid size (${String(opened.size)} bytes): ${filePath}`)
    }
    const bytes = fs.readFileSync(handle)
    const afterRead = fs.fstatSync(handle)
    if (
      afterRead.dev !== opened.dev
      || afterRead.ino !== opened.ino
      || afterRead.size !== opened.size
      || afterRead.mtimeMs !== opened.mtimeMs
      || afterRead.ctimeMs !== opened.ctimeMs
      || bytes.length !== opened.size
    ) {
      fail(`${label} changed while it was being read: ${filePath}`)
    }
    return {
      bytes,
      size_bytes: opened.size,
      sha256: sha256Bytes(bytes),
    }
  } finally {
    if (handle !== undefined) fs.closeSync(handle)
  }
}

function readJsonSnapshot(filePath, label) {
  const snapshot = readOrdinaryFileSnapshot(filePath, label, {
    maximumBytes: 8 * 1024 * 1024,
  })
  try {
    return {
      ...snapshot,
      value: JSON.parse(snapshot.bytes.toString('utf8').replace(/^\uFEFF/, '')),
    }
  } catch (error) {
    fail(`${label} is not valid JSON: ${filePath}: ${String(error)}`)
  }
}

function requireExactArray(actual, expected, label) {
  if (!Array.isArray(actual) || JSON.stringify(actual) !== JSON.stringify(expected)) {
    fail(`${label} does not match the reviewed authority contract.`)
  }
}

function requireBoolean(value, expected, label) {
  if (value !== expected) fail(`${label} must equal ${String(expected)}.`)
}

function requireSha256(value, label) {
  const normalized = String(value ?? '').trim().toLowerCase()
  if (!/^[a-f0-9]{64}$/.test(normalized)) {
    fail(`${label} must be a SHA-256 digest.`)
  }
  return normalized
}

function parseTimestamp(value, label) {
  if (typeof value !== 'string' || value.trim() === '') {
    fail(`${label} must be an ISO timestamp.`)
  }
  const parsed = Date.parse(value)
  if (!Number.isFinite(parsed)) fail(`${label} must be an ISO timestamp.`)
  return parsed
}

function normalizeCommand(value) {
  return String(value ?? '')
    .replaceAll('\\', '/')
    .toLowerCase()
    .replace(/\s+/g, ' ')
    .trim()
}

function requireSinglePassingStep(
  candidate,
  name,
  scriptName,
  forbiddenArguments = [],
  requiredArguments = [],
) {
  if (!Array.isArray(candidate.steps)) {
    fail('Candidate summary has no steps array.')
  }
  const matching = candidate.steps.filter((step) => step?.name === name)
  if (
    matching.length !== 1
    || matching[0]?.passed !== true
    || matching[0]?.skipped === true
  ) {
    fail(`Candidate summary must contain exactly one non-skipped passing "${name}" step.`)
  }
  const step = matching[0]
  const command = normalizeCommand(step.command)
  if (!command.includes(`/scripts/${scriptName.toLowerCase()}`)) {
    fail(`Candidate "${name}" step does not invoke scripts/${scriptName}.`)
  }
  for (const argument of forbiddenArguments) {
    if (command.includes(argument.toLowerCase())) {
      fail(`Candidate "${name}" step uses forbidden diagnostic argument ${argument}.`)
    }
  }
  for (const argument of requiredArguments) {
    if (!command.includes(argument.toLowerCase())) {
      fail(`Candidate "${name}" step is missing required argument ${argument}.`)
    }
  }
  const started = parseTimestamp(step.started_at_utc, `${name}.started_at_utc`)
  const finished = parseTimestamp(step.finished_at_utc, `${name}.finished_at_utc`)
  if (finished < started) fail(`Candidate "${name}" step has an invalid time interval.`)
  return { step, started, finished }
}

function parseRequiredTests(harnessText) {
  const identities = [
    ...harnessText.matchAll(
      /"(commands::commands_execution::tests::[A-Za-z0-9_]+|execution_policy::tests::[A-Za-z0-9_]+|execution_mocks::tests::[A-Za-z0-9_]+|execution_quantity::tests::[A-Za-z0-9_]+|settings::tests::[A-Za-z0-9_]+)"/g,
    ),
  ].map((match) => match[1])
  if (identities.length !== new Set(identities).size) {
    fail('Live safety harness contains duplicate required test identities.')
  }
  if (identities.length !== REQUIRED_TEST_COUNT) {
    fail(
      `Live safety harness required-test contract changed: expected ${String(REQUIRED_TEST_COUNT)}, observed ${String(identities.length)}.`,
    )
  }
  return identities
}

function categorizeTests(requiredTests) {
  return new Map([
    [
      'commands_execution_complete',
      requiredTests.filter((name) =>
        name.startsWith('commands::commands_execution::tests::')),
    ],
    [
      'execution_policy_complete',
      requiredTests.filter((name) => name.startsWith('execution_policy::tests::')),
    ],
    [
      'execution_mocks_quantity_and_settings',
      requiredTests.filter((name) =>
        name.startsWith('execution_mocks::tests::')
        || name.startsWith('execution_quantity::tests::')
        || name.startsWith('settings::tests::')),
    ],
  ])
}

function parseCargoTestRows(text) {
  const rows = []
  for (const line of text.split(/\r?\n/)) {
    const match = line.match(
      /^\s*test\s+(?<name>\S+)\s+\.\.\.\s+(?<status>ok|FAILED|ignored)(?:\s|$)/,
    )
    if (!match?.groups) continue
    rows.push({
      name: match.groups.name,
      status: match.groups.status.toLowerCase(),
    })
  }
  return rows
}

function resolveRepoFile(repoRoot, relativePath, label) {
  if (
    typeof relativePath !== 'string'
    || path.isAbsolute(relativePath)
    || relativePath.replaceAll('\\', '/').split('/').includes('..')
  ) {
    fail(`${label} is not a safe repo-relative path.`)
  }
  const resolved = path.resolve(repoRoot, relativePath)
  const prefix = `${normalizedPath(repoRoot)}${path.sep}`
  if (!normalizedPath(resolved).startsWith(prefix)) {
    fail(`${label} escaped the repository root.`)
  }
  return resolved
}

export function loadPublicLiveSafetyContract(repoRoot = defaultRepoRoot) {
  const resolvedRepoRoot = path.resolve(repoRoot)
  const harnessPath = path.join(
    resolvedRepoRoot,
    'scripts',
    'run_live_execution_safety_harness.ps1',
  )
  const harnessSnapshot = readOrdinaryFileSnapshot(
    harnessPath,
    'Live safety harness source',
    { maximumBytes: 2 * 1024 * 1024 },
  )
  const requiredTests = parseRequiredTests(harnessSnapshot.bytes.toString('utf8'))
  return {
    contract: CONTRACT_ID,
    required_test_count: REQUIRED_TEST_COUNT,
    required_tests: requiredTests,
    coverage: categorizeTests(requiredTests),
    source_contract: SOURCE_CONTRACT.map(([id, relativePath]) => ({
      id,
      path: relativePath,
      sha256: readOrdinaryFileSnapshot(
        resolveRepoFile(resolvedRepoRoot, relativePath, `Source contract ${id}`),
        `Source contract ${id}`,
        { maximumBytes: 32 * 1024 * 1024 },
      ).sha256,
    })),
  }
}

function validateCandidateSummary({
  candidate,
  candidateSummaryPath,
  outDir,
  variant,
}) {
  if (
    Number(candidate.schema_version) !== 1
    || candidate.status !== 'passed'
    || candidate.variant !== variant
    || candidate.include_execution_safety_gate !== true
    || candidate.include_live_safety_harness !== true
  ) {
    fail(
      'Candidate summary is failed, foreign-variant, or missing required execution/live safety authority flags.',
    )
  }
  if (!samePath(candidateSummaryPath, path.join(outDir, CANDIDATE_SUMMARY_FILE))) {
    fail('Candidate summary must use the canonical direct-child path.')
  }
  if (!samePath(candidate.out_dir, outDir)) {
    fail('Candidate summary belongs to a foreign output directory.')
  }
  if (!samePath(candidate.artifacts?.live_safety, path.join(outDir, SUMMARY_FILE))) {
    fail('Candidate summary does not bind the canonical live-safety summary path.')
  }

  const execution = requireSinglePassingStep(
    candidate,
    'Execution safety gate',
    'run_execution_safety_gate.ps1',
    ['-SkipRust', '-SkipWeb', '-SkipCommandContract'],
    ['-NoProfile', '-ExecutionPolicy bypass'],
  )
  const live = requireSinglePassingStep(
    candidate,
    'Live execution safety harness',
    'run_live_execution_safety_harness.ps1',
    ['-ContractSimulation', '-CargoExecutable'],
    ['-NoProfile', '-ExecutionPolicy bypass'],
  )
  const executionIndex = candidate.steps.indexOf(execution.step)
  const liveIndex = candidate.steps.indexOf(live.step)
  if (
    executionIndex < 0
    || liveIndex <= executionIndex
    || live.started < execution.finished
  ) {
    fail('Candidate safety steps are missing or ordered incorrectly.')
  }

  const candidateGenerated = parseTimestamp(
    candidate.generated_at_utc,
    'candidate.generated_at_utc',
  )
  if (candidateGenerated < live.finished) {
    fail('Candidate summary predates completion of its live safety step.')
  }
  return { execution, live }
}

function validateSourceContract(summary, expectedSources, repoRoot) {
  if (summary.source_contract_stable !== true) {
    fail('Live safety source contract is not stable.')
  }
  for (const property of ['source_contract_before', 'source_contract_after']) {
    const rows = summary[property]
    if (!Array.isArray(rows) || rows.length !== expectedSources.length) {
      fail(`${property} does not contain the exact reviewed source set.`)
    }
    for (let index = 0; index < expectedSources.length; index += 1) {
      const expected = expectedSources[index]
      const actual = rows[index]
      if (
        actual?.id !== expected.id
        || String(actual?.path ?? '').replaceAll('\\', '/') !== expected.path
      ) {
        fail(`${property}[${String(index)}] has a foreign source identity.`)
      }
      const actualHash = requireSha256(
        actual.sha256,
        `${property}[${String(index)}].sha256`,
      )
      if (actualHash !== expected.sha256) {
        fail(`${property}[${String(index)}] no longer matches the current repo source.`)
      }
      const sourcePath = resolveRepoFile(repoRoot, expected.path, expected.id)
      if (
        readOrdinaryFileSnapshot(sourcePath, `Current source ${expected.id}`, {
          maximumBytes: 32 * 1024 * 1024,
        }).sha256 !== actualHash
      ) {
        fail(`Current source ${expected.id} changed during public revalidation.`)
      }
    }
  }
  if (
    JSON.stringify(summary.source_contract_before)
    !== JSON.stringify(summary.source_contract_after)
  ) {
    fail('Live safety source contract changed during its Cargo invocation.')
  }
}

function validateCoverage(summary, contract, cargoRows) {
  if (
    Number(summary.required_test_count) !== REQUIRED_TEST_COUNT
    || Number(summary.observed_test_count) !== cargoRows.length
    || cargoRows.length < REQUIRED_TEST_COUNT
  ) {
    fail('Live safety observed-test counts do not prove the complete pinned contract.')
  }
  if (!Array.isArray(summary.coverage) || summary.coverage.length !== 3) {
    fail('Live safety coverage does not contain the three reviewed categories.')
  }

  const cargoStatus = new Map(cargoRows.map((row) => [row.name, row.status]))
  const flattened = []
  for (let index = 0; index < COVERAGE_CATEGORIES.length; index += 1) {
    const category = COVERAGE_CATEGORIES[index]
    const row = summary.coverage[index]
    const expectedTests = contract.coverage.get(category)
    if (row?.category !== category || row?.passed !== true || !Array.isArray(row.tests)) {
      fail(`Live safety coverage category ${category} is missing or failed.`)
    }
    if (row.tests.length !== expectedTests.length) {
      fail(`Live safety coverage category ${category} has an incomplete test set.`)
    }
    for (let testIndex = 0; testIndex < expectedTests.length; testIndex += 1) {
      const expectedName = expectedTests[testIndex]
      const testRow = row.tests[testIndex]
      if (
        testRow?.name !== expectedName
        || testRow?.status !== 'ok'
        || testRow?.passed !== true
        || cargoStatus.get(expectedName) !== 'ok'
      ) {
        fail(`Live safety required test did not pass exactly: ${expectedName}`)
      }
      flattened.push(expectedName)
    }
  }
  requireExactArray(flattened, contract.required_tests, 'Live safety flattened test set')
}

function validateLiveSummary({
  summary,
  contract,
  liveStep,
  stdoutSnapshot,
  stderrSnapshot,
  repoRoot,
}) {
  if (
    Number(summary.schema_version) !== 2
    || summary.report_kind !== 'live-execution-safety-authority'
    || summary.run_mode !== 'authority'
    || summary.status !== 'passed'
    || summary.passed !== true
    || summary.authority_eligible !== true
    || summary.error !== null
  ) {
    fail('Live safety summary is failed, diagnostic-only, or not authority-eligible.')
  }
  const generated = parseTimestamp(summary.generated_at_utc, 'live.generated_at_utc')
  if (generated < liveStep.started || generated > liveStep.finished) {
    fail('Live safety summary was not generated by the current candidate step.')
  }

  const isolation = summary.isolation
  for (const property of [
    'cargo_offline',
    'dependency_lock_enforced',
    'credential_environment_cleared',
    'external_http_proxy_trap_configured',
  ]) {
    requireBoolean(isolation?.[property], true, `isolation.${property}`)
  }
  requireBoolean(
    isolation?.operating_system_network_isolation,
    false,
    'isolation.operating_system_network_isolation',
  )
  requireBoolean(
    isolation?.proxy_bypassing_clients_excluded,
    true,
    'isolation.proxy_bypassing_clients_excluded',
  )
  if (
    isolation?.external_http_proxy !== 'http://127.0.0.1:9'
    || Number(isolation?.rust_test_threads) !== 1
  ) {
    fail('Live safety proxy trap or serialized test isolation is incomplete.')
  }
  requireExactArray(
    isolation?.credential_environment_names,
    REQUIRED_CREDENTIAL_ENVIRONMENT_NAMES,
    'Live safety credential environment names',
  )
  requireExactArray(
    isolation?.loopback_mock_hosts,
    ['localhost', '127.0.0.1', '::1'],
    'Live safety loopback mock hosts',
  )

  const cargo = summary.cargo
  if (
    cargo?.requested_executable !== 'cargo'
    || Number(cargo?.invocation_count) !== 1
    || cargo?.feature_contract_verified !== true
    || Number(cargo?.exit_code) !== 0
    || cargo?.stdout_log !== STDOUT_FILE
    || cargo?.stderr_log !== STDERR_FILE
  ) {
    fail('Live safety Cargo identity is diagnostic, incomplete, or failed.')
  }
  requireExactArray(cargo.arguments, REQUIRED_CARGO_ARGUMENTS, 'Live safety Cargo arguments')
  const cargoPath = String(cargo.resolved_executable_path ?? '')
  if (!path.isAbsolute(cargoPath)) fail('Live safety resolved Cargo path must be absolute.')
  const cargoSnapshot = readOrdinaryFileSnapshot(cargoPath, 'Resolved Cargo executable', {
    maximumBytes: 256 * 1024 * 1024,
  })
  if (
    requireSha256(cargo.executable_sha256, 'cargo.executable_sha256')
    !== cargoSnapshot.sha256
  ) {
    fail('Resolved Cargo executable identity changed after live safety authority.')
  }

  validateSourceContract(summary, contract.source_contract, repoRoot)
  const cargoRows = parseCargoTestRows(
    `${stdoutSnapshot.bytes.toString('utf8')}\n${stderrSnapshot.bytes.toString('utf8')}`,
  )
  validateCoverage(summary, contract, cargoRows)
  return cargoSnapshot
}

export function verifyPublicLiveSafetyEvidence({
  outDir,
  candidateSummaryPath = '',
  variant,
  repoRoot = defaultRepoRoot,
}) {
  const resolvedOutDir = path.resolve(outDir)
  const resolvedRepoRoot = path.resolve(repoRoot)
  if (!ALLOWED_VARIANTS.has(variant)) fail(`Unsupported public release variant: ${variant}`)
  requireOrdinaryDirectory(resolvedOutDir, 'Public release output directory')

  const resolvedCandidateSummaryPath = candidateSummaryPath
    ? path.resolve(candidateSummaryPath)
    : path.join(resolvedOutDir, CANDIDATE_SUMMARY_FILE)
  const candidateSnapshot = readJsonSnapshot(
    resolvedCandidateSummaryPath,
    'Release candidate summary',
  )
  const candidateSteps = validateCandidateSummary({
    candidate: candidateSnapshot.value,
    candidateSummaryPath: resolvedCandidateSummaryPath,
    outDir: resolvedOutDir,
    variant,
  })

  const summaryPath = path.join(resolvedOutDir, SUMMARY_FILE)
  const stdoutPath = path.join(resolvedOutDir, STDOUT_FILE)
  const stderrPath = path.join(resolvedOutDir, STDERR_FILE)
  const summarySnapshot = readJsonSnapshot(summaryPath, 'Live safety summary')
  const stdoutSnapshot = readOrdinaryFileSnapshot(stdoutPath, 'Live safety stdout log', {
    maximumBytes: 64 * 1024 * 1024,
    allowEmpty: true,
  })
  const stderrSnapshot = readOrdinaryFileSnapshot(stderrPath, 'Live safety stderr log', {
    maximumBytes: 64 * 1024 * 1024,
    allowEmpty: true,
  })
  const contract = loadPublicLiveSafetyContract(resolvedRepoRoot)
  const cargoSnapshot = validateLiveSummary({
    summary: summarySnapshot.value,
    contract,
    liveStep: candidateSteps.live,
    stdoutSnapshot,
    stderrSnapshot,
    repoRoot: resolvedRepoRoot,
  })

  return {
    schema_version: 1,
    contract: CONTRACT_ID,
    status: 'passed',
    generated_at_utc: new Date().toISOString(),
    variant,
    authority: {
      execution_safety_gate: 'passed',
      live_safety_harness: 'passed',
      live_safety_run_mode: 'authority',
      authority_eligible: true,
      required_test_count: REQUIRED_TEST_COUNT,
    },
    evidence: {
      candidate_summary: {
        path: resolvedCandidateSummaryPath,
        size_bytes: candidateSnapshot.size_bytes,
        sha256: candidateSnapshot.sha256,
      },
      live_safety_summary: {
        path: summaryPath,
        size_bytes: summarySnapshot.size_bytes,
        sha256: summarySnapshot.sha256,
      },
      stdout_log: {
        path: stdoutPath,
        size_bytes: stdoutSnapshot.size_bytes,
        sha256: stdoutSnapshot.sha256,
      },
      stderr_log: {
        path: stderrPath,
        size_bytes: stderrSnapshot.size_bytes,
        sha256: stderrSnapshot.sha256,
      },
    },
    cargo: {
      path: String(summarySnapshot.value.cargo.resolved_executable_path),
      size_bytes: cargoSnapshot.size_bytes,
      sha256: cargoSnapshot.sha256,
      invocation_count: 1,
      arguments: REQUIRED_CARGO_ARGUMENTS,
    },
    source_contract_sha256: sha256Bytes(
      Buffer.from(JSON.stringify(contract.source_contract), 'utf8'),
    ),
  }
}

function parseArgs(argv) {
  const parsed = {
    outDir: '',
    candidateSummaryPath: '',
    variant: '',
  }
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index]
    const value = argv[index + 1]
    if (argument === '--out-dir' && value) parsed.outDir = value
    else if (argument === '--candidate-summary-path' && value) {
      parsed.candidateSummaryPath = value
    } else if (argument === '--variant' && value) parsed.variant = value
    else fail(`Unknown or incomplete argument: ${String(argument)}`)
    index += 1
  }
  if (!parsed.outDir || !parsed.variant) {
    fail('--out-dir and --variant are required.')
  }
  return parsed
}

if (samePath(process.argv[1], fileURLToPath(import.meta.url))) {
  try {
    const result = verifyPublicLiveSafetyEvidence(parseArgs(process.argv.slice(2)))
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`)
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`)
    process.exitCode = 1
  }
}
