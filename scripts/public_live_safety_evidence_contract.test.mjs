import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import crypto from 'node:crypto'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import test from 'node:test'
import { fileURLToPath } from 'node:url'
import {
  loadPublicLiveSafetyContract,
  verifyPublicLiveSafetyEvidence,
} from './public_live_safety_evidence_contract.mjs'

const contractCliPath = fileURLToPath(
  new URL('./public_live_safety_evidence_contract.mjs', import.meta.url),
)

function sha256(filePath) {
  return crypto.createHash('sha256').update(fs.readFileSync(filePath)).digest('hex')
}

function writeJson(filePath, value) {
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8')
}

function fixture() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'vectorgrid-public-live-safety-'))
  const outDir = path.join(root, 'evidence')
  fs.mkdirSync(outDir)
  const cargoPath = path.join(root, 'cargo.exe')
  fs.writeFileSync(cargoPath, Buffer.from('reviewed-cargo-executable-fixture'))

  const contract = loadPublicLiveSafetyContract()
  const coverage = [...contract.coverage].map(([category, requiredTests]) => ({
    category,
    passed: true,
    tests: requiredTests.map((name) => ({
      name,
      status: 'ok',
      passed: true,
    })),
  }))
  const stdoutPath = path.join(outDir, 'live-safety-cargo.stdout.log')
  fs.writeFileSync(
    stdoutPath,
    [
      `running ${String(contract.required_test_count)} tests`,
      ...contract.required_tests.map((name) => `test ${name} ... ok`),
      `test result: ok. ${String(contract.required_test_count)} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`,
      '',
    ].join('\n'),
    'utf8',
  )
  fs.writeFileSync(path.join(outDir, 'live-safety-cargo.stderr.log'), '', 'utf8')

  const liveSummary = {
    schema_version: 2,
    report_kind: 'live-execution-safety-authority',
    run_mode: 'authority',
    generated_at_utc: '2026-07-28T12:06:00.000Z',
    status: 'passed',
    passed: true,
    authority_eligible: true,
    error: null,
    authority_scope: {
      claim: 'offline unit-contract authority fixture',
      excludes: [],
    },
    isolation: {
      cargo_offline: true,
      dependency_lock_enforced: true,
      credential_environment_cleared: true,
      credential_environment_names: [
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
      ],
      credential_test_contract: 'synthetic fixture',
      external_http_proxy_trap_configured: true,
      external_http_proxy: 'http://127.0.0.1:9',
      loopback_mock_hosts: ['localhost', '127.0.0.1', '::1'],
      rust_test_threads: 1,
      operating_system_network_isolation: false,
      proxy_bypassing_clients_excluded: true,
    },
    cargo: {
      invocation_count: 1,
      requested_executable: 'cargo',
      resolved_executable_path: cargoPath,
      executable_sha256: sha256(cargoPath),
      arguments: [
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
      ],
      feature_contract:
        'verified explicit desktop-ui,custom-protocol CPU feature set; default and cuda features disabled',
      feature_contract_verified: true,
      exit_code: 0,
      stdout_log: 'live-safety-cargo.stdout.log',
      stderr_log: 'live-safety-cargo.stderr.log',
    },
    required_test_count: contract.required_test_count,
    observed_test_count: contract.required_test_count,
    source_contract_stable: true,
    source_contract_before: contract.source_contract,
    source_contract_after: contract.source_contract,
    coverage,
  }
  writeJson(path.join(outDir, 'live-safety-summary.json'), liveSummary)

  const candidateSummaryPath = path.join(outDir, 'summary.json')
  const candidateSummary = {
    schema_version: 1,
    generated_at_utc: '2026-07-28T12:11:00.000Z',
    variant: 'cpu',
    status: 'passed',
    out_dir: outDir,
    include_execution_safety_gate: true,
    include_live_safety_harness: true,
    artifacts: {
      live_safety: path.join(outDir, 'live-safety-summary.json'),
    },
    steps: [
      {
        name: 'Execution safety gate',
        command: `powershell -NoProfile -ExecutionPolicy Bypass -File ${path.join(
          process.cwd(),
          'scripts',
          'run_execution_safety_gate.ps1',
        )}`,
        started_at_utc: '2026-07-28T11:50:00.000Z',
        finished_at_utc: '2026-07-28T12:00:00.000Z',
        passed: true,
      },
      {
        name: 'Live execution safety harness',
        command: `powershell -NoProfile -ExecutionPolicy Bypass -File ${path.join(
          process.cwd(),
          'scripts',
          'run_live_execution_safety_harness.ps1',
        )} -OutDir ${outDir}`,
        started_at_utc: '2026-07-28T12:00:00.000Z',
        finished_at_utc: '2026-07-28T12:10:00.000Z',
        passed: true,
      },
    ],
  }
  writeJson(candidateSummaryPath, candidateSummary)

  return {
    root,
    outDir,
    candidateSummaryPath,
    options: {
      outDir,
      candidateSummaryPath,
      variant: 'cpu',
    },
  }
}

function withFixture(callback) {
  const value = fixture()
  try {
    return callback(value)
  } finally {
    fs.rmSync(value.root, { recursive: true, force: true })
  }
}

function mutateJson(filePath, mutation) {
  const value = JSON.parse(fs.readFileSync(filePath, 'utf8'))
  mutation(value)
  writeJson(filePath, value)
}

test('fresh authority evidence binds both safety gates, exact Cargo, sources, and 128 tests', () => {
  withFixture(({ options }) => {
    const verified = verifyPublicLiveSafetyEvidence(options)
    assert.equal(verified.schema_version, 1)
    assert.equal(verified.contract, 'vectorgrid-public-live-safety-evidence-v1')
    assert.equal(verified.status, 'passed')
    assert.equal(verified.variant, 'cpu')
    assert.equal(verified.authority.execution_safety_gate, 'passed')
    assert.equal(verified.authority.live_safety_harness, 'passed')
    assert.equal(verified.authority.authority_eligible, true)
    assert.equal(verified.authority.required_test_count, 128)
    assert.match(verified.evidence.live_safety_summary.sha256, /^[a-f0-9]{64}$/)
    assert.match(verified.source_contract_sha256, /^[a-f0-9]{64}$/)
  })
})

test('CLI emits the same passing public authority contract', () => {
  withFixture(({ options }) => {
    const result = spawnSync(
      process.execPath,
      [
        contractCliPath,
        '--out-dir',
        options.outDir,
        '--candidate-summary-path',
        options.candidateSummaryPath,
        '--variant',
        options.variant,
      ],
      {
        cwd: path.resolve(path.dirname(contractCliPath), '..'),
        encoding: 'utf8',
        shell: false,
        windowsHide: true,
      },
    )
    assert.equal(result.status, 0, String(result.stderr))
    const verified = JSON.parse(result.stdout)
    assert.equal(verified.contract, 'vectorgrid-public-live-safety-evidence-v1')
    assert.equal(verified.status, 'passed')
  })
})

test('diagnostic, ineligible, stale, or custom-Cargo live evidence fails closed', () => {
  const mutations = [
    {
      label: 'diagnostic mode',
      apply(summary) {
        summary.report_kind = 'live-execution-safety-contract-simulation'
        summary.run_mode = 'contract_simulation'
      },
    },
    {
      label: 'ineligible authority',
      apply(summary) {
        summary.authority_eligible = false
      },
    },
    {
      label: 'stale timestamp',
      apply(summary) {
        summary.generated_at_utc = '2026-07-27T12:06:00.000Z'
      },
    },
    {
      label: 'custom Cargo arguments',
      apply(summary) {
        summary.cargo.arguments.push('--features', 'cuda')
      },
    },
  ]
  for (const mutation of mutations) {
    withFixture(({ outDir, options }) => {
      mutateJson(path.join(outDir, 'live-safety-summary.json'), mutation.apply)
      assert.throws(
        () => verifyPublicLiveSafetyEvidence(options),
        /authority|current candidate|Cargo arguments|diagnostic|eligible/i,
        mutation.label,
      )
    })
  }
})

test('missing logs, incomplete coverage, and source drift fail closed', () => {
  withFixture(({ outDir, options }) => {
    fs.rmSync(path.join(outDir, 'live-safety-cargo.stdout.log'))
    assert.throws(
      () => verifyPublicLiveSafetyEvidence(options),
      /stdout log is missing/i,
    )
  })

  withFixture(({ outDir, options }) => {
    mutateJson(path.join(outDir, 'live-safety-summary.json'), (summary) => {
      summary.coverage[0].tests[0].passed = false
      summary.coverage[0].tests[0].status = 'FAILED'
    })
    assert.throws(
      () => verifyPublicLiveSafetyEvidence(options),
      /required test did not pass/i,
    )
  })

  withFixture(({ outDir, options }) => {
    mutateJson(path.join(outDir, 'live-safety-summary.json'), (summary) => {
      summary.source_contract_after[0].sha256 = '0'.repeat(64)
    })
    assert.throws(
      () => verifyPublicLiveSafetyEvidence(options),
      /source|current repo/i,
    )
  })
})

test('candidate must record both unskipped authority steps with safe invocations', () => {
  const mutations = [
    {
      label: 'execution flag absent',
      apply(candidate) {
        candidate.include_execution_safety_gate = false
      },
    },
    {
      label: 'execution step absent',
      apply(candidate) {
        candidate.steps = candidate.steps.filter(
          (step) => step.name !== 'Execution safety gate',
        )
      },
    },
    {
      label: 'live simulation argument',
      apply(candidate) {
        candidate.steps[1].command += ' -ContractSimulation'
      },
    },
    {
      label: 'execution skip argument',
      apply(candidate) {
        candidate.steps[0].command += ' -SkipRust'
      },
    },
    {
      label: 'profile-dependent live invocation',
      apply(candidate) {
        candidate.steps[1].command = candidate.steps[1].command.replace(
          '-NoProfile ',
          '',
        )
      },
    },
  ]
  for (const mutation of mutations) {
    withFixture(({ candidateSummaryPath, options }) => {
      mutateJson(candidateSummaryPath, mutation.apply)
      assert.throws(
        () => verifyPublicLiveSafetyEvidence(options),
        /authority flags|exactly one|forbidden diagnostic argument|missing required argument/i,
        mutation.label,
      )
    })
  }
})
