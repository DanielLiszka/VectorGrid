import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import test from 'node:test'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.resolve(scriptDir, '..')
const harnessPath = path.join(scriptDir, 'run_live_execution_safety_harness.ps1')
const harness = fs.readFileSync(harnessPath, 'utf8')

const pinnedIdentityOccurrences = [...harness.matchAll(
  /"(commands::commands_execution::tests::[A-Za-z0-9_]+|execution_policy::tests::[A-Za-z0-9_]+|execution_mocks::tests::[A-Za-z0-9_]+|execution_quantity::tests::[A-Za-z0-9_]+|settings::tests::[A-Za-z0-9_]+)"/g,
)].map((match) => match[1])
const requiredTests = [...new Set(pinnedIdentityOccurrences)]
const pinnedCommandsExecutionTests = pinnedIdentityOccurrences.filter((identity) =>
  identity.startsWith('commands::commands_execution::tests::'),
)

const criticalAuthorityTests = [
  'commands::commands_execution::tests::alpaca_credential_storage_status_tracks_presence_bits',
  'commands::commands_execution::tests::resolve_alpaca_credentials_reads_stored_pair_by_default',
  'commands::commands_execution::tests::alpaca_http_client_policy_is_https_only_redirect_free_and_endpoints_are_pinned',
  'commands::commands_execution::tests::execution_target_normalization_blocks_legacy_broker_routing_and_pins_explicit_modes',
  'commands::commands_execution::tests::symbol_matching_does_not_collapse_punctuation_variants',
  'commands::commands_execution::tests::alpaca_live_validation_verification_rejects_dataset_content_drift',
  'commands::commands_execution::tests::alpaca_live_validation_verification_rejects_cost_drift',
  'commands::commands_execution::tests::alpaca_live_validation_verification_rejects_unapproved_run_verdict',
  'commands::commands_execution::tests::alpaca_live_validation_verification_rejects_inconsistent_approval_counts',
  'commands::commands_execution::tests::alpaca_live_validation_verification_rejects_research_only_promotion',
  'commands::commands_execution::tests::alpaca_account_tradeable_validation_blocks_non_active_or_blocked_accounts',
  'commands::commands_execution::tests::alpaca_market_clock_validation_requires_open_session',
  'commands::commands_execution::tests::live_execution_source_context_rejects_unverified_nonempty_fingerprint',
  'commands::commands_execution::tests::live_runner_stream_preconditions_reject_live_broker_on_delayed_stream',
  'commands::commands_execution::tests::live_runner_stream_preconditions_require_recent_accepted_bar',
  'commands::commands_execution::tests::execution_strategy_support_matrix_is_code_backed_and_fail_closed',
  'execution_policy::tests::sign_flip_is_risk_increasing_even_when_target_absolute_exposure_is_smaller',
  'execution_policy::tests::blocks_daily_order_and_loss_caps_and_fails_closed_without_state',
  'commands::commands_execution::tests::alpaca_account_daily_pnl_accepts_numeric_strings_and_rejects_missing_data',
  'commands::commands_execution::tests::broker_safety_action_rechecks_runner_after_waiting_for_shared_lease',
  'commands::commands_execution::tests::credential_mutation_quiesces_active_and_concurrently_rearmed_runner_under_broker_lease',
  'commands::commands_execution::tests::alpaca_order_intent_is_persisted_without_credentials_and_can_update_status',
  'commands::commands_execution::tests::corrupt_alpaca_order_intent_blocks_submission_with_reconciliation_guidance',
  'commands::commands_execution::tests::unresolved_alpaca_intent_blocks_new_client_order_for_same_symbol',
  'commands::commands_execution::tests::submit_or_reuse_alpaca_order_avoids_duplicate_submit_when_lookup_finds_existing_order',
  'commands::commands_execution::tests::alpaca_flatten_intent_retry_reuses_only_unconfirmed_matching_payload_and_terminal_history_gets_new_id',
  'commands::commands_execution::tests::unresolved_alpaca_intents_are_isolated_by_broker_target',
  'commands::commands_execution::tests::alpaca_intent_decode_requires_payload_hash_and_exact_client_order_binding',
  'commands::commands_execution::tests::alpaca_broker_response_validation_binds_full_payload_and_client_order_id',
  'commands::commands_execution::tests::non_retryable_broker_open_intent_cannot_be_reset_for_resubmission',
  'commands::commands_execution::tests::unresolved_alpaca_intent_restart_reconciliation_requires_position_refresh_for_fills',
  'commands::commands_execution::tests::broker_open_flatten_order_must_exactly_reduce_current_position',
  'commands::commands_execution::tests::owned_alpaca_protective_order_selection_is_target_symbol_and_intent_scoped',
  'commands::commands_execution::tests::guard_open_orders_before_fresh_delta_blocks_when_qty_metadata_is_missing',
  'commands::commands_execution::tests::cancel_order_snapshots_reports_partial_failure_without_hiding_completed_cancels',
  'commands::commands_execution::tests::flatten_position_order_payload_handles_long_and_short_positions',
  'commands::commands_execution::tests::runner_activation_requires_audit_slot_before_runner_becomes_visible',
  'commands::commands_execution::tests::manual_broker_execution_audit_is_durable_before_submission_attempt',
  'commands::commands_execution::tests::manual_execution_orchestration_begins_audit_before_starting_broker_work',
  'commands::commands_execution::tests::manual_execution_audit_failure_becomes_warning_only_after_submission',
  'commands::commands_execution::tests::live_session_exact_path_finalization_cannot_clear_replacement_session',
]

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8').replace(/^\uFEFF/, ''))
}

function writeFakeCargo(root, tests) {
  const fakeCargoPath = path.join(root, 'cargo.cmd')
  const rows = tests.map((name) => `echo test ${name} ... ok`)
  fs.writeFileSync(
    fakeCargoPath,
    [
      '@echo off',
      '>>"%VECTORGRID_FAKE_CARGO_LOG%" echo invocation=%*',
      '>>"%VECTORGRID_FAKE_CARGO_LOG%" echo ALPACA_API_KEY=[%ALPACA_API_KEY%]',
      '>>"%VECTORGRID_FAKE_CARGO_LOG%" echo APCA_API_KEY_ID=[%APCA_API_KEY_ID%]',
      '>>"%VECTORGRID_FAKE_CARGO_LOG%" echo POLYGON_API_KEY=[%POLYGON_API_KEY%]',
      '>>"%VECTORGRID_FAKE_CARGO_LOG%" echo HTTPS_PROXY=[%HTTPS_PROXY%]',
      `echo running ${tests.length} tests`,
      'echo.',
      ...rows,
      'echo.',
      `echo test result: ok. ${tests.length} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`,
      'exit /b 0',
      '',
    ].join('\r\n'),
  )
  return fakeCargoPath
}

function invokeHarness(fakeCargoPath, outDir, fakeLog, extraEnv = {}) {
  return spawnSync(
    'powershell.exe',
    [
      '-NoProfile',
      '-NonInteractive',
      '-ExecutionPolicy',
      'Bypass',
      '-File',
      harnessPath,
      '-OutDir',
      outDir,
      '-CargoExecutable',
      fakeCargoPath,
      '-ContractSimulation',
    ],
    {
      cwd: repoRoot,
      encoding: 'utf8',
      env: {
        ...process.env,
        VECTORGRID_FAKE_CARGO_LOG: fakeLog,
        ALPACA_API_KEY: 'LIVE_SAFETY_CONTRACT_CANARY',
        APCA_API_KEY_ID: 'LIVE_SAFETY_CONTRACT_CANARY',
        POLYGON_API_KEY: 'LIVE_SAFETY_CONTRACT_CANARY',
        HTTPS_PROXY: 'http://preexisting.invalid:1234',
        ...extraEnv,
      },
      shell: false,
      windowsHide: true,
    },
  )
}

test('live safety harness is one offline CPU Cargo authority with fail-closed evidence', () => {
  assert.equal(
    [...harness.matchAll(/Start-Process\s+-FilePath\s+\$resolvedCargoExecutable/g)].length,
    1,
  )
  assert.match(harness, /\$invocationCount\s+-ne\s+1/)
  assert.match(harness, /"test",\s*"--locked",\s*"--offline"/s)
  assert.match(
    harness,
    /"--no-default-features",\s*"--features",\s*"desktop-ui,custom-protocol"/s,
  )
  assert.match(harness, /"--lib",\s*"execution"/s)
  assert.match(harness, /CARGO_NET_OFFLINE\s*=\s*"true"/)
  assert.match(harness, /HTTP_PROXY\s*=\s*"http:\/\/127\.0\.0\.1:9"/)
  assert.match(harness, /Assert-FreshEvidencePaths/)
  assert.match(harness, /schema_version\s*=\s*2/)
  assert.match(harness, /"live-execution-safety-authority"/)
  assert.match(harness, /authority_eligible\s*=\s*\$authorityEligible/)
  assert.match(harness, /Custom -CargoExecutable values are allowed only with -ContractSimulation/)
  assert.match(harness, /feature_contract_verified\s*=\s*\$cpuFeatureContractVerified/)
  assert.match(harness, /commands_execution\s*=\s*"src-tauri\/src\/commands_execution\.rs"/)
  assert.match(harness, /execution_policy\s*=\s*"src-tauri\/src\/execution_policy\.rs"/)
  assert.match(harness, /System\.Security\.Cryptography\.SHA256/)
  assert.match(harness, /Get-SourceContractHashes/)
  assert.match(harness, /source_contract_stable\s*=\s*\$sourceContractStable/)
  assert.match(harness, /changed during the Cargo invocation/)
  assert.match(harness, /unit-contract authority for the enumerated live-execution safety invariants/)

  const argumentBlock = harness.match(/\$cargoArguments\s*=\s*@\(([\s\S]*?)\n\)/)?.[1]
  assert.ok(argumentBlock, 'cargo argument block was not found')
  assert.doesNotMatch(argumentBlock, /cuda/i)

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
    assert.ok(harness.includes(`"${credentialName}"`), `missing ${credentialName} isolation`)
  }

  for (const category of [
    'commands_execution_complete',
    'execution_policy_complete',
    'execution_mocks_quantity_and_settings',
  ]) {
    assert.match(harness, new RegExp(`\\b${category}\\s*=\\s*\\$required`))
  }
  assert.equal(pinnedIdentityOccurrences.length, requiredTests.length, 'pinned identities must be unique')
  assert.equal(pinnedCommandsExecutionTests.length, 107)
  assert.equal(new Set(pinnedCommandsExecutionTests).size, 107)
  for (const identity of criticalAuthorityTests) {
    assert.ok(requiredTests.includes(identity), `critical authority row was removed: ${identity}`)
  }

  const tauriManifest = fs.readFileSync(path.join(repoRoot, 'src-tauri', 'Cargo.toml'), 'utf8')
  const appCoreManifest = fs.readFileSync(
    path.join(repoRoot, 'crates', 'app_core', 'Cargo.toml'),
    'utf8',
  )
  assert.match(tauriManifest, /^default\s*=\s*\["custom-protocol",\s*"desktop-ui"\]\s*$/m)
  assert.match(tauriManifest, /^desktop-ui\s*=\s*\[\]\s*$/m)
  assert.match(tauriManifest, /^custom-protocol\s*=\s*\["tauri\/custom-protocol"\]\s*$/m)
  assert.match(tauriManifest, /^cuda\s*=\s*\["ta_app_core\/cuda",\s*"vector-ta\/cuda"\]\s*$/m)
  assert.match(appCoreManifest, /^default\s*=\s*\[\]\s*$/m)
  assert.match(appCoreManifest, /^cuda\s*=\s*\["vector-ta\/cuda",\s*"dep:cust"\]\s*$/m)
})

test('every exact authority identity still names a Rust unit test', () => {
  const sources = new Map([
    [
      'commands::commands_execution::tests::',
      fs.readFileSync(path.join(repoRoot, 'src-tauri', 'src', 'commands_execution.rs'), 'utf8'),
    ],
    [
      'execution_policy::tests::',
      fs.readFileSync(path.join(repoRoot, 'src-tauri', 'src', 'execution_policy.rs'), 'utf8'),
    ],
    [
      'execution_mocks::tests::',
      fs.readFileSync(path.join(repoRoot, 'src-tauri', 'src', 'execution_mocks.rs'), 'utf8'),
    ],
    [
      'execution_quantity::tests::',
      fs.readFileSync(path.join(repoRoot, 'src-tauri', 'src', 'execution_quantity.rs'), 'utf8'),
    ],
    [
      'settings::tests::',
      fs.readFileSync(path.join(repoRoot, 'src-tauri', 'src', 'settings.rs'), 'utf8'),
    ],
  ])

  for (const identity of requiredTests) {
    const leafName = identity.split('::').at(-1)
    const sourceEntry = [...sources].find(([prefix]) => identity.startsWith(prefix))
    assert.ok(sourceEntry, `no source mapping for ${identity}`)
    const source = sourceEntry[1]
    const escaped = leafName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    assert.match(
      source,
      new RegExp(`#\\[test\\][\\s\\S]{0,240}?fn\\s+${escaped}\\s*\\(`),
      `missing Rust test for ${identity}`,
    )
  }

  const commandsSource = sources.get('commands::commands_execution::tests::')
  const testsModule = commandsSource.slice(commandsSource.indexOf('#[cfg(test)]\nmod tests'))
  const sourceLeafNames = [
    ...testsModule.matchAll(/#\[test\][\s\S]{0,300}?\n\s*(?:async\s+)?fn\s+([A-Za-z0-9_]+)/g),
  ].map((match) => match[1])
  const sourceIdentities = sourceLeafNames.map(
    (name) => `commands::commands_execution::tests::${name}`,
  )
  assert.equal(sourceIdentities.length, 107)
  assert.equal(new Set(sourceIdentities).size, 107)
  assert.deepEqual(
    [...pinnedCommandsExecutionTests].sort(),
    [...sourceIdentities].sort(),
    'the harness must pin the exact commands_execution test namespace',
  )
})

test('live command wiring preserves audit-before-submit and shared broker leases', () => {
  const commands = fs.readFileSync(
    path.join(repoRoot, 'src-tauri', 'src', 'commands_execution.rs'),
    'utf8',
  )
  const sliceBetween = (startMarker, endMarker) => {
    const start = commands.indexOf(startMarker)
    const end = commands.indexOf(endMarker, start + startMarker.length)
    assert.ok(start >= 0, `missing start marker: ${startMarker}`)
    assert.ok(end > start, `missing end marker after ${startMarker}: ${endMarker}`)
    return commands.slice(start, end)
  }
  const assertBefore = (source, first, second, label) => {
    const firstIndex = source.indexOf(first)
    const secondIndex = source.indexOf(second)
    assert.ok(firstIndex >= 0, `${label}: missing ${first}`)
    assert.ok(secondIndex >= 0, `${label}: missing ${second}`)
    assert.ok(firstIndex < secondIndex, `${label}: ${first} must precede ${second}`)
  }

  const oneShot = sliceBetween(
    'pub async fn execute_strategy_signal_once(',
    'pub async fn start_live_execution_runner(',
  )
  assert.match(
    oneShot,
    /begin_audit_before_starting_execution\(\s*\|\|\s*\{\s*begin_manual_execution_live_session\([\s\S]*?\}\s*,\s*start_execution\s*,?\s*\)/,
  )
  assertBefore(
    oneShot,
    'begin_audit_before_starting_execution(',
    'let execution_result = execution_task',
    'manual execution audit-before-await wiring',
  )

  const cancel = sliceBetween(
    'pub async fn cancel_live_execution_open_orders(',
    'pub async fn flatten_live_execution_position(',
  )
  assertBefore(
    cancel,
    'with_broker_safety_action_lease(||',
    'resolve_alpaca_credentials_for_app(',
    'cancel safety-action lease',
  )
  assertBefore(
    cancel,
    'with_broker_safety_action_lease(||',
    'fetch_alpaca_open_orders_for_symbol(',
    'cancel broker I/O lease',
  )

  const flatten = sliceBetween(
    'pub async fn flatten_live_execution_position(',
    'pub fn emergency_halt_live_execution(',
  )
  assertBefore(
    flatten,
    'with_broker_safety_action_lease(||',
    'resolve_alpaca_credentials_for_app(',
    'flatten safety-action lease',
  )
  assertBefore(
    flatten,
    'reserve_alpaca_flatten_intent(',
    'submit_or_reuse_alpaca_order(',
    'flatten durable intent ordering',
  )

  const setCredentials = sliceBetween(
    'pub fn alpaca_set_api_credentials(',
    'pub fn alpaca_clear_api_credentials(',
  )
  assertBefore(
    setCredentials,
    'apply_credential_mutation_after_quiescing(',
    'store_alpaca_api_credentials(',
    'credential-set quiescence',
  )
  const clearCredentials = sliceBetween(
    'pub fn alpaca_clear_api_credentials(',
    'pub fn get_execution_risk_policy(',
  )
  assertBefore(
    clearCredentials,
    'apply_credential_mutation_after_quiescing(',
    'clear_alpaca_api_credentials_for_app(',
    'credential-clear quiescence',
  )
  const credentialWrapper = sliceBetween(
    'fn apply_credential_mutation_after_quiescing',
    'pub fn alpaca_set_api_credentials(',
  )
  assertBefore(
    credentialWrapper,
    'quiesce_runner_for_credential_mutation(app)?',
    'mutation()',
    'credential mutation wrapper',
  )
})

test('simulated run clears credentials, configures the HTTP proxy trap, and attests exact tests', () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'vectorgrid-live-safety-contract-'))
  try {
    const outDir = path.join(tempRoot, 'evidence')
    fs.mkdirSync(outDir)
    const unrelatedEvidence = path.join(outDir, 'readiness.json')
    fs.writeFileSync(unrelatedEvidence, '{"status":"passed"}')
    const fakeLog = path.join(tempRoot, 'fake-cargo.log')
    const fakeCargoPath = writeFakeCargo(tempRoot, requiredTests)
    const result = invokeHarness(fakeCargoPath, outDir, fakeLog)
    assert.equal(
      result.status,
      0,
      `simulated harness failed:\n${String(result.stdout)}\n${String(result.stderr)}`,
    )

    const invocations = fs.readFileSync(fakeLog, 'utf8')
    assert.equal((invocations.match(/^invocation=/gm) ?? []).length, 1)
    assert.match(
      invocations,
      /invocation=test --locked --offline -p ta_desktop_demo_app --no-default-features --features desktop-ui,custom-protocol --lib execution -- --test-threads=1/,
    )
    assert.match(invocations, /ALPACA_API_KEY=\[\]/)
    assert.match(invocations, /APCA_API_KEY_ID=\[\]/)
    assert.match(invocations, /POLYGON_API_KEY=\[\]/)
    assert.match(invocations, /HTTPS_PROXY=\[http:\/\/127\.0\.0\.1:9\]/)
    assert.doesNotMatch(invocations, /LIVE_SAFETY_CONTRACT_CANARY/)

    const summary = readJson(path.join(outDir, 'live-safety-summary.json'))
    assert.equal(summary.schema_version, 2)
    assert.equal(summary.report_kind, 'live-execution-safety-contract-simulation')
    assert.equal(summary.run_mode, 'contract_simulation')
    assert.equal(summary.status, 'passed')
    assert.equal(summary.passed, true)
    assert.equal(summary.authority_eligible, false)
    assert.equal(summary.cargo.invocation_count, 1)
    assert.equal(summary.cargo.exit_code, 0)
    assert.equal(path.resolve(summary.cargo.resolved_executable_path), path.resolve(fakeCargoPath))
    assert.match(summary.cargo.executable_sha256, /^[a-f0-9]{64}$/)
    assert.equal(summary.cargo.feature_contract_verified, true)
    assert.equal(summary.isolation.credential_environment_cleared, true)
    assert.equal(summary.isolation.cargo_offline, true)
    assert.equal(summary.isolation.dependency_lock_enforced, true)
    assert.equal(summary.isolation.external_http_proxy_trap_configured, true)
    assert.equal(summary.isolation.operating_system_network_isolation, false)
    assert.equal(summary.isolation.proxy_bypassing_clients_excluded, true)
    assert.equal(summary.required_test_count, requiredTests.length)
    assert.equal(summary.source_contract_stable, true)
    assert.deepEqual(
      summary.source_contract_before.map((row) => row.id),
      [
        'harness',
        'commands_execution',
        'execution_policy',
        'execution_mocks',
        'execution_quantity',
        'settings',
        'run_artifacts',
        'tauri_manifest',
        'app_core_manifest',
        'cargo_lock',
      ],
    )
    assert.equal(
      summary.source_contract_before.every((row) => /^[a-f0-9]{64}$/.test(row.sha256)),
      true,
    )
    assert.deepEqual(summary.source_contract_after, summary.source_contract_before)
    assert.equal(summary.coverage.every((row) => row.passed), true)
    assert.equal(
      summary.coverage.flatMap((row) => row.tests).every((row) => row.status === 'ok'),
      true,
    )
    assert.equal(fs.readFileSync(unrelatedEvidence, 'utf8'), '{"status":"passed"}')
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true })
  }
})

test('missing exact test and stale output both fail closed', () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'vectorgrid-live-safety-negative-'))
  try {
    const missingOutDir = path.join(tempRoot, 'missing-evidence')
    const missingLog = path.join(tempRoot, 'missing-cargo.log')
    const fakeCargoPath = writeFakeCargo(tempRoot, requiredTests.slice(1))
    const missing = invokeHarness(fakeCargoPath, missingOutDir, missingLog)
    assert.notEqual(missing.status, 0)
    const failedSummary = readJson(path.join(missingOutDir, 'live-safety-summary.json'))
    assert.equal(failedSummary.status, 'failed')
    assert.equal(failedSummary.passed, false)
    assert.match(failedSummary.error, /did not observe every required exact test/)
    assert.equal(
      failedSummary.coverage.flatMap((row) => row.tests).some((row) => row.status === 'missing'),
      true,
    )

    for (const ownedName of [
      'live-safety-summary.json',
      'live-safety-cargo.stdout.log',
      'live-safety-cargo.stderr.log',
    ]) {
      const staleOutDir = path.join(tempRoot, `stale-${ownedName.replaceAll('.', '-')}`)
      fs.mkdirSync(staleOutDir)
      const sentinel = path.join(staleOutDir, ownedName)
      fs.writeFileSync(sentinel, 'do not overwrite')
      const staleLog = path.join(tempRoot, `fake-${ownedName}.log`)
      const stale = invokeHarness(fakeCargoPath, staleOutDir, staleLog)
      assert.notEqual(stale.status, 0)
      assert.match(
        `${String(stale.stdout)}\n${String(stale.stderr)}`,
        /refusing to reuse stale evidence/,
      )
      assert.equal(fs.readFileSync(sentinel, 'utf8'), 'do not overwrite')
      assert.equal(fs.existsSync(staleLog), false)
    }
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true })
  }
})

test('executable-resolution failure records actual unverified controls and no authority eligibility', () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'vectorgrid-live-safety-preflight-'))
  try {
    const outDir = path.join(tempRoot, 'evidence')
    const fakeLog = path.join(tempRoot, 'fake-cargo.log')
    const missingCargoPath = path.join(tempRoot, 'missing-cargo.exe')
    const result = invokeHarness(missingCargoPath, outDir, fakeLog)
    assert.notEqual(result.status, 0)
    const summary = readJson(path.join(outDir, 'live-safety-summary.json'))
    assert.equal(summary.status, 'failed')
    assert.equal(summary.passed, false)
    assert.equal(summary.authority_eligible, false)
    assert.equal(summary.cargo.invocation_count, 0)
    assert.equal(summary.cargo.resolved_executable_path, null)
    assert.equal(summary.cargo.executable_sha256, null)
    assert.equal(summary.cargo.feature_contract_verified, false)
    assert.equal(summary.isolation.credential_environment_cleared, false)
    assert.equal(summary.isolation.cargo_offline, false)
    assert.equal(summary.isolation.dependency_lock_enforced, false)
    assert.equal(summary.isolation.external_http_proxy_trap_configured, false)
    assert.equal(fs.existsSync(fakeLog), false)
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true })
  }
})

test('managed environment is restored before returning to the calling PowerShell process', () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'vectorgrid-live-safety-restore-'))
  try {
    const outDir = path.join(tempRoot, 'evidence')
    const fakeLog = path.join(tempRoot, 'fake-cargo.log')
    const restorePath = path.join(tempRoot, 'restored.json')
    const wrapperPath = path.join(tempRoot, 'invoke-in-process.ps1')
    const fakeCargoPath = writeFakeCargo(tempRoot, requiredTests)
    fs.writeFileSync(
      wrapperPath,
      [
        '$ErrorActionPreference = "Stop"',
        '$env:ALPACA_API_KEY = "RESTORE_ALPACA_CANARY"',
        '$env:HTTPS_PROXY = "http://restore.invalid:4321"',
        '$env:CARGO_NET_OFFLINE = "restore-offline-value"',
        '$env:RUST_TEST_THREADS = "7"',
        '& $env:VECTORGRID_HARNESS_PATH -OutDir $env:VECTORGRID_OUT_DIR -CargoExecutable $env:VECTORGRID_FAKE_CARGO_PATH -ContractSimulation',
        '[ordered]@{',
        '  alpaca_api_key = $env:ALPACA_API_KEY',
        '  https_proxy = $env:HTTPS_PROXY',
        '  cargo_net_offline = $env:CARGO_NET_OFFLINE',
        '  rust_test_threads = $env:RUST_TEST_THREADS',
        '} | ConvertTo-Json | Set-Content -Encoding UTF8 $env:VECTORGRID_RESTORE_PATH',
        '',
      ].join('\r\n'),
    )
    const result = spawnSync(
      'powershell.exe',
      ['-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', wrapperPath],
      {
        cwd: repoRoot,
        encoding: 'utf8',
        env: {
          ...process.env,
          VECTORGRID_HARNESS_PATH: harnessPath,
          VECTORGRID_OUT_DIR: outDir,
          VECTORGRID_FAKE_CARGO_PATH: fakeCargoPath,
          VECTORGRID_FAKE_CARGO_LOG: fakeLog,
          VECTORGRID_RESTORE_PATH: restorePath,
        },
        shell: false,
        windowsHide: true,
      },
    )
    assert.equal(
      result.status,
      0,
      `in-process restoration probe failed:\n${String(result.stdout)}\n${String(result.stderr)}`,
    )
    assert.deepEqual(readJson(restorePath), {
      alpaca_api_key: 'RESTORE_ALPACA_CANARY',
      https_proxy: 'http://restore.invalid:4321',
      cargo_net_offline: 'restore-offline-value',
      rust_test_threads: '7',
    })
    assert.doesNotMatch(fs.readFileSync(fakeLog, 'utf8'), /RESTORE_ALPACA_CANARY/)
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true })
  }
})

test('managed environment is restored on the fail-closed exact-test path', () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'vectorgrid-live-safety-restore-fail-'))
  try {
    const outDir = path.join(tempRoot, 'evidence')
    const fakeLog = path.join(tempRoot, 'fake-cargo.log')
    const restorePath = path.join(tempRoot, 'restored.json')
    const wrapperPath = path.join(tempRoot, 'invoke-failure-in-process.ps1')
    const fakeCargoPath = writeFakeCargo(tempRoot, requiredTests.slice(1))
    fs.writeFileSync(
      wrapperPath,
      [
        '$ErrorActionPreference = "Stop"',
        '$env:ALPACA_API_KEY = "RESTORE_FAILURE_ALPACA_CANARY"',
        '$env:HTTPS_PROXY = "http://restore-failure.invalid:9876"',
        '$env:CARGO_NET_OFFLINE = "restore-failure-offline-value"',
        '$env:RUST_TEST_THREADS = "9"',
        '$harnessFailed = $false',
        'try {',
        '  & $env:VECTORGRID_HARNESS_PATH -OutDir $env:VECTORGRID_OUT_DIR -CargoExecutable $env:VECTORGRID_FAKE_CARGO_PATH -ContractSimulation',
        '} catch {',
        '  $harnessFailed = $true',
        '}',
        '[ordered]@{',
        '  harness_failed = $harnessFailed',
        '  alpaca_api_key = $env:ALPACA_API_KEY',
        '  https_proxy = $env:HTTPS_PROXY',
        '  cargo_net_offline = $env:CARGO_NET_OFFLINE',
        '  rust_test_threads = $env:RUST_TEST_THREADS',
        '} | ConvertTo-Json | Set-Content -Encoding UTF8 $env:VECTORGRID_RESTORE_PATH',
        '',
      ].join('\r\n'),
    )
    const result = spawnSync(
      'powershell.exe',
      ['-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', wrapperPath],
      {
        cwd: repoRoot,
        encoding: 'utf8',
        env: {
          ...process.env,
          VECTORGRID_HARNESS_PATH: harnessPath,
          VECTORGRID_OUT_DIR: outDir,
          VECTORGRID_FAKE_CARGO_PATH: fakeCargoPath,
          VECTORGRID_FAKE_CARGO_LOG: fakeLog,
          VECTORGRID_RESTORE_PATH: restorePath,
        },
        shell: false,
        windowsHide: true,
      },
    )
    assert.equal(
      result.status,
      0,
      `failure-path restoration probe failed:\n${String(result.stdout)}\n${String(result.stderr)}`,
    )
    assert.deepEqual(readJson(restorePath), {
      harness_failed: true,
      alpaca_api_key: 'RESTORE_FAILURE_ALPACA_CANARY',
      https_proxy: 'http://restore-failure.invalid:9876',
      cargo_net_offline: 'restore-failure-offline-value',
      rust_test_threads: '9',
    })
    const summary = readJson(path.join(outDir, 'live-safety-summary.json'))
    assert.equal(summary.status, 'failed')
    assert.equal(summary.authority_eligible, false)
    assert.doesNotMatch(fs.readFileSync(fakeLog, 'utf8'), /RESTORE_FAILURE_ALPACA_CANARY/)
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true })
  }
})

test('live safety PowerShell remains parser-clean', () => {
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
  const result = spawnSync(
    'powershell.exe',
    ['-NoProfile', '-NonInteractive', '-EncodedCommand', encoded],
    {
      cwd: repoRoot,
      encoding: 'utf8',
      env: {
        ...process.env,
        VECTORGRID_SCRIPT_UNDER_TEST: harnessPath,
      },
      shell: false,
      windowsHide: true,
    },
  )
  assert.equal(
    result.status,
    0,
    `PowerShell parser errors:\n${String(result.stderr || result.stdout).trim()}`,
  )
})
