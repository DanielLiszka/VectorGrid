import assert from 'node:assert/strict'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import test from 'node:test'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.resolve(scriptDir, '..')
const soakScript = path.join(scriptDir, 'run_cpu_reliability_soak.ps1')

function read(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), 'utf8')
}

function runPlan(args, environment = process.env) {
  return spawnSync(
    'powershell',
    [
      '-NoProfile',
      '-ExecutionPolicy',
      'Bypass',
      '-File',
      soakScript,
      '-PlanOnly',
      ...args,
    ],
    {
      cwd: repoRoot,
      encoding: 'utf8',
      shell: false,
      env: environment,
    },
  )
}

function runRecovery(outDir, targetDir) {
  return spawnSync(
    'powershell',
    [
      '-NoProfile',
      '-ExecutionPolicy',
      'Bypass',
      '-File',
      soakScript,
      '-RecoverStaleOnly',
      '-OutDir',
      outDir,
      '-CargoTargetDir',
      targetDir,
    ],
    {
      cwd: repoRoot,
      encoding: 'utf8',
      shell: false,
    },
  )
}

function parseEmbeddedJson(stdout) {
  const start = stdout.indexOf('{')
  const end = stdout.lastIndexOf('}')
  assert.ok(start >= 0 && end > start, `missing JSON in output:\n${stdout}`)
  return JSON.parse(stdout.slice(start, end + 1))
}

test('plan-only uses an explicit validated Cargo target without mutating run output', () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'vectorgrid-soak-plan-'))
  const targetDir = path.join(tempRoot, 'cargo-target')
  try {
    const result = runPlan(['-Mode', 'short', '-CargoTargetDir', targetDir])
    assert.equal(
      result.status,
      0,
      `plan failed:\n${String(result.stdout)}\n${String(result.stderr)}`,
    )
    const plan = JSON.parse(result.stdout)
    assert.equal(plan.schema_version, 2)
    assert.equal(plan.mode, 'short')
    assert.equal(plan.cargo_target_root, path.resolve(targetDir))
    assert.equal(
      path.dirname(plan.cargo_target_dir),
      path.resolve(targetDir),
    )
    assert.match(path.basename(plan.cargo_target_dir), /^cpu-reliability-soak-/)
    assert.equal(plan.cargo_target_source, 'explicit-parameter')
    assert.equal(
      path.dirname(path.dirname(path.dirname(plan.binary_path_hint))),
      path.resolve(plan.cargo_target_dir),
    )
    assert.match(plan.cargo_target_triple, /^[A-Za-z0-9_.-]+$/)
    assert.equal(plan.operator_release_authority_requested, false)
    assert.equal(plan.provider_credentials_used, false)
    assert.equal(plan.provider_network_calls, false)
    assert.equal(fs.existsSync(path.dirname(plan.evidence_path)), false)
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true })
  }
})

test('short-mode minimum rows preserve multi-tile cancellation authority', () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'vectorgrid-soak-rows-'))
  const targetDir = path.join(tempRoot, 'cargo-target')
  try {
    const accepted = runPlan([
      '-Mode',
      'short',
      '-Rows',
      '2048',
      '-CargoTargetDir',
      targetDir,
    ])
    assert.equal(
      accepted.status,
      0,
      `minimum-row plan failed:\n${String(accepted.stdout)}\n${String(accepted.stderr)}`,
    )
    assert.equal(JSON.parse(accepted.stdout).rows, 2048)

    const rejected = runPlan([
      '-Mode',
      'short',
      '-Rows',
      '2047',
      '-CargoTargetDir',
      targetDir,
    ])
    assert.notEqual(rejected.status, 0)
    assert.match(
      `${String(rejected.stdout)}\n${String(rejected.stderr)}`,
      /Rows must be >= 2048.*multiple CPU tiles/i,
    )
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true })
  }
})

test('source-manifest-only exercises PS5-compatible complete build-input hashing', () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'vectorgrid-soak-source-'))
  const targetDir = path.join(tempRoot, 'cargo-target')
  try {
    const result = spawnSync(
      'powershell',
      [
        '-NoProfile',
        '-ExecutionPolicy',
        'Bypass',
        '-File',
        soakScript,
        '-SourceManifestOnly',
        '-CargoTargetDir',
        targetDir,
      ],
      {
        cwd: repoRoot,
        encoding: 'utf8',
        shell: false,
      },
    )
    assert.equal(
      result.status,
      0,
      `source manifest check failed:\n${String(result.stdout)}\n${String(result.stderr)}`,
    )
    const summary = JSON.parse(result.stdout)
    assert.equal(
      summary.kind,
      'vectorgrid_cpu_reliability_soak_source_manifest_check',
    )
    assert.equal(summary.status, 'passed')
    assert.ok(summary.file_count > 100)
    assert.match(summary.aggregate_sha256, /^[a-f0-9]{64}$/)
    assert.equal(summary.includes_rust_toolchain, true)
    assert.equal(summary.includes_cargo_config, true)
    assert.equal(summary.includes_windows_tauri_config, true)
    assert.equal(summary.includes_tauri_resources, true)
    assert.equal(summary.frontend_dist_effective, false)
    assert.equal(summary.context_asset_mode, 'dev_url_no_embedded_frontend_assets')
    assert.equal(summary.includes_vendored_vector_ta, true)
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true })
  }
})

test('build-input probe inventories external Cargo config and ambient Tauri overrides without building', () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'vectorgrid-soak-inputs-'))
  const targetDir = path.join(tempRoot, 'cargo-target')
  const cargoHome = path.join(tempRoot, 'cargo-home')
  fs.mkdirSync(cargoHome, { recursive: true })
  fs.writeFileSync(
    path.join(cargoHome, 'config.toml'),
    '[build]\njobs = 1\n',
  )
  try {
    const result = spawnSync(
      'powershell',
      [
        '-NoProfile',
        '-ExecutionPolicy',
        'Bypass',
        '-File',
        soakScript,
        '-BuildInputProbeOnly',
        '-CargoTargetDir',
        targetDir,
      ],
      {
        cwd: repoRoot,
        encoding: 'utf8',
        shell: false,
        env: {
          ...process.env,
          CARGO_HOME: cargoHome,
          TAURI_CONFIG: '{"productName":"untrusted override"}',
          TAURI_ENV_TARGET_TRIPLE: 'untrusted-target',
        },
      },
    )
    assert.equal(
      result.status,
      0,
      `build-input probe failed:\n${String(result.stdout)}\n${String(result.stderr)}`,
    )
    const probe = JSON.parse(result.stdout)
    assert.equal(probe.kind, 'vectorgrid_cpu_reliability_soak_build_input_probe')
    assert.equal(probe.status, 'passed')
    assert.equal(probe.tauri_config_override_present_before_isolation, true)
    assert.equal(probe.tauri_target_override_present_before_isolation, true)
    assert.equal(
      probe.cargo_configuration.external_existing_file_count,
      1,
    )
    assert.equal(probe.cargo_configuration.release_authority_eligible, false)
    assert.equal(probe.release_authority_eligible, false)
    assert.match(
      probe.cargo_configuration.aggregate_sha256,
      /^[a-f0-9]{64}$/,
    )
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true })
  }
})

test(
  'toolchain probe resolves rustup-selected binaries from the repository CWD without building',
  { skip: process.platform !== 'win32' },
  () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'vectorgrid-soak-toolchain-'))
    const mockBin = path.join(tempRoot, 'mock-bin')
    const sysroot = path.join(tempRoot, 'mock-sysroot')
    const sysrootBin = path.join(sysroot, 'bin')
    const targetLib = path.join(
      sysroot,
      'lib',
      'rustlib',
      'x86_64-pc-windows-msvc',
      'lib',
    )
    const targetDir = path.join(tempRoot, 'cargo-target')
    fs.mkdirSync(mockBin, { recursive: true })
    fs.mkdirSync(sysrootBin, { recursive: true })
    fs.mkdirSync(targetLib, { recursive: true })
    fs.writeFileSync(
      path.join(mockBin, 'rustup.cmd'),
      [
        '@echo off',
        'if "%1"=="which" if "%2"=="cargo" (',
        '  echo %MOCK_CARGO_PATH%',
        '  exit /b 0',
        ')',
        'if "%1"=="which" if "%2"=="rustc" (',
        '  echo %MOCK_RUSTC_PATH%',
        '  exit /b 0',
        ')',
        'exit /b 9',
        '',
      ].join('\r\n'),
    )
    fs.writeFileSync(
      path.join(sysrootBin, 'cargo.cmd'),
      [
        '@echo off',
        'if "%1"=="--version" (',
        '  echo cargo 1.99.0 mock',
        '  echo cwd:%CD%',
        '  exit /b 0',
        ')',
        'exit /b 9',
        '',
      ].join('\r\n'),
    )
    fs.writeFileSync(
      path.join(sysrootBin, 'rustc.cmd'),
      [
        '@echo off',
        'if "%1"=="--version" (',
        '  echo rustc 1.99.0 mock',
        '  echo host: x86_64-pc-windows-msvc',
        '  echo cwd:%CD%',
        '  exit /b 0',
        ')',
        'if "%1"=="--print" if "%2"=="sysroot" (',
        '  echo %MOCK_RUSTC_SYSROOT%',
        '  exit /b 0',
        ')',
        'exit /b 9',
        '',
      ].join('\r\n'),
    )
    fs.writeFileSync(path.join(targetLib, 'libstd-mock.rlib'), 'mock target stdlib')

    try {
      const result = spawnSync(
        'powershell',
        [
          '-NoProfile',
          '-ExecutionPolicy',
          'Bypass',
          '-File',
          soakScript,
          '-ToolchainProbeOnly',
          '-CargoTargetDir',
          targetDir,
        ],
        {
          cwd: tempRoot,
          encoding: 'utf8',
          shell: false,
          env: {
            ...process.env,
            PATH: `${mockBin}${path.delimiter}${process.env.PATH ?? ''}`,
            MOCK_RUSTC_SYSROOT: sysroot,
            MOCK_CARGO_PATH: path.join(sysrootBin, 'cargo.cmd'),
            MOCK_RUSTC_PATH: path.join(sysrootBin, 'rustc.cmd'),
          },
        },
      )
      assert.equal(
        result.status,
        0,
        `toolchain probe failed:\n${String(result.stdout)}\n${String(result.stderr)}`,
      )
      const probe = JSON.parse(result.stdout)
      assert.equal(probe.kind, 'vectorgrid_cpu_reliability_soak_toolchain_probe')
      assert.equal(probe.status, 'passed')
      assert.equal(
        path.resolve(probe.toolchain.cargo_invocation_path).toLowerCase(),
        path.resolve(mockBin, 'rustup.cmd').toLowerCase(),
      )
      assert.equal(
        path.resolve(probe.toolchain.rustc_invocation_path).toLowerCase(),
        path.resolve(mockBin, 'rustup.cmd').toLowerCase(),
      )
      assert.equal(
        path.resolve(probe.toolchain.cargo_path).toLowerCase(),
        path.resolve(sysrootBin, 'cargo.cmd').toLowerCase(),
      )
      assert.equal(
        path.resolve(probe.toolchain.rustc_path).toLowerCase(),
        path.resolve(sysrootBin, 'rustc.cmd').toLowerCase(),
      )
      for (const version of [
        probe.toolchain.cargo_version_verbose,
        probe.toolchain.rustc_version_verbose,
      ]) {
        const cwdLine = version
          .split(/\r?\n/)
          .find((line) => line.toLowerCase().startsWith('cwd:'))
        assert.ok(cwdLine, `missing repository CWD attestation in ${version}`)
        assert.equal(
          path.resolve(cwdLine.slice(4)).toLowerCase(),
          path.resolve(repoRoot).toLowerCase(),
        )
      }
      assert.match(probe.toolchain.cargo_sha256, /^[a-f0-9]{64}$/)
      assert.match(probe.toolchain.rustc_sha256, /^[a-f0-9]{64}$/)
      assert.equal(probe.toolchain.host_target_triple, 'x86_64-pc-windows-msvc')
      assert.ok(probe.toolchain.input_manifest.file_count >= 3)
      assert.match(
        probe.toolchain.input_manifest.aggregate_sha256,
        /^[a-f0-9]{64}$/,
      )
      assert.equal(fs.existsSync(targetDir), false)
    } finally {
      fs.rmSync(tempRoot, { recursive: true, force: true })
    }
  },
)

test('plan-only honors CARGO_TARGET_DIR when no explicit override is supplied', () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'vectorgrid-soak-env-'))
  const targetDir = path.join(tempRoot, 'environment-target')
  try {
    const result = runPlan(
      ['-Mode', 'short'],
      { ...process.env, CARGO_TARGET_DIR: targetDir },
    )
    assert.equal(
      result.status,
      0,
      `environment-target plan failed:\n${String(result.stdout)}\n${String(result.stderr)}`,
    )
    const plan = JSON.parse(result.stdout)
    assert.equal(plan.cargo_target_root, path.resolve(targetDir))
    assert.equal(path.dirname(plan.cargo_target_dir), path.resolve(targetDir))
    assert.equal(plan.cargo_target_source, 'inherited-environment')
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true })
  }
})

test('short SkipBuild requires an explicit target-contained prebuilt path', () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'vectorgrid-soak-prebuilt-'))
  const targetDir = path.join(tempRoot, 'cargo-target')
  const binaryPath = path.join(
    targetDir,
    'release',
    process.platform === 'win32'
      ? 'ta_cpu_reliability_soak.exe'
      : 'ta_cpu_reliability_soak',
  )
  try {
    const missing = runPlan([
      '-Mode',
      'short',
      '-SkipBuild',
      '-CargoTargetDir',
      targetDir,
    ])
    assert.notEqual(missing.status, 0)
    assert.match(
      `${String(missing.stdout)}\n${String(missing.stderr)}`,
      /requires an explicit PrebuiltBinaryPath/,
    )

    const explicit = runPlan([
      '-Mode',
      'short',
      '-SkipBuild',
      '-CargoTargetDir',
      targetDir,
      '-PrebuiltBinaryPath',
      binaryPath,
    ])
    assert.equal(
      explicit.status,
      0,
      `prebuilt plan failed:\n${String(explicit.stdout)}\n${String(explicit.stderr)}`,
    )
    const plan = JSON.parse(explicit.stdout)
    assert.equal(plan.skip_build, true)
    assert.equal(plan.binary_path_hint, path.resolve(binaryPath))
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true })
  }
})

test('unsafe filesystem-root Cargo targets fail closed', () => {
  const filesystemRoot = path.parse(repoRoot).root
  const result = runPlan([
    '-Mode',
    'short',
    '-CargoTargetDir',
    filesystemRoot,
  ])
  assert.notEqual(result.status, 0)
  assert.match(
    `${String(result.stdout)}\n${String(result.stderr)}`,
    /must not be a filesystem root/i,
  )
})

test('reparse-point Cargo targets fail closed', (t) => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'vectorgrid-soak-reparse-'))
  const realTarget = path.join(tempRoot, 'real-target')
  const linkedTarget = path.join(tempRoot, 'linked-target')
  fs.mkdirSync(realTarget)
  try {
    try {
      fs.symlinkSync(
        realTarget,
        linkedTarget,
        process.platform === 'win32' ? 'junction' : 'dir',
      )
    } catch (error) {
      if (['EPERM', 'EACCES', 'ENOTSUP'].includes(error?.code)) {
        t.skip(`directory reparse-point creation is unavailable: ${error.code}`)
        return
      }
      throw error
    }
    const result = runPlan([
      '-Mode',
      'short',
      '-CargoTargetDir',
      linkedTarget,
    ])
    assert.notEqual(result.status, 0)
    assert.match(
      `${String(result.stdout)}\n${String(result.stderr)}`,
      /reparse-point path or ancestor/i,
    )
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true })
  }
})

test('stale recovery removes only stopped marker-owned isolation and refuses active owners', () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'vectorgrid-soak-recovery-'))
  const outDir = path.join(tempRoot, 'evidence')
  const targetDir = path.join(tempRoot, 'cargo-target')
  const stoppedRoot = path.join(
    outDir,
    'stopped-run',
    'vectorgrid-soak-stopped-run',
  )
  fs.mkdirSync(stoppedRoot, { recursive: true })
  fs.writeFileSync(
    path.join(stoppedRoot, '.vectorgrid-cpu-reliability-soak-owner.json'),
    JSON.stringify({
      schema_version: 1,
      kind: 'vectorgrid_cpu_reliability_soak_isolation',
      pid: 2_147_483_647,
      nonce: 'stopped-owner',
    }),
  )
  const stoppedTarget = path.join(
    targetDir,
    'cpu-reliability-soak-stopped-target',
  )
  fs.mkdirSync(stoppedTarget, { recursive: true })
  fs.writeFileSync(
    path.join(
      stoppedTarget,
      '.vectorgrid-cpu-reliability-soak-target-owner.json',
    ),
    JSON.stringify({
      schema_version: 1,
      kind: 'vectorgrid_cpu_reliability_soak_cargo_target',
      run_id: 'stopped-target',
      pid: 2_147_483_647,
      nonce: 'stopped-target-owner',
    }),
  )
  try {
    const recovered = runRecovery(outDir, targetDir)
    assert.equal(
      recovered.status,
      0,
      `recovery failed:\n${String(recovered.stdout)}\n${String(recovered.stderr)}`,
    )
    const summary = parseEmbeddedJson(recovered.stdout)
    assert.equal(summary.kind, 'vectorgrid_cpu_reliability_soak_recovery')
    assert.equal(summary.recovered_stale_isolations, 1)
    assert.equal(summary.recovered_stale_cargo_targets, 1)
    assert.equal(fs.existsSync(stoppedRoot), false)
    assert.equal(fs.existsSync(stoppedTarget), false)

    const activeRoot = path.join(
      outDir,
      'active-run',
      'vectorgrid-soak-active-run',
    )
    fs.mkdirSync(activeRoot, { recursive: true })
    fs.writeFileSync(
      path.join(activeRoot, '.vectorgrid-cpu-reliability-soak-owner.json'),
      JSON.stringify({
        schema_version: 1,
        kind: 'vectorgrid_cpu_reliability_soak_isolation',
        pid: process.pid,
        nonce: 'active-owner',
      }),
    )
    const refused = runRecovery(outDir, targetDir)
    assert.notEqual(refused.status, 0)
    assert.match(
      `${String(refused.stdout)}\n${String(refused.stderr)}`,
      /owner is still running/i,
    )
    assert.equal(fs.existsSync(activeRoot), true)
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true })
  }
})

test('operator authority is six hours, release-only, freshly built, and duration terminated', () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'vectorgrid-soak-operator-'))
  const targetDir = path.join(tempRoot, 'cargo-target')
  try {
    const valid = runPlan([
      '-Mode',
      'operator',
      '-Hours',
      '6',
      '-CargoTargetDir',
      targetDir,
    ])
    assert.equal(
      valid.status,
      0,
      `operator plan failed:\n${String(valid.stdout)}\n${String(valid.stderr)}`,
    )
    const plan = JSON.parse(valid.stdout)
    assert.equal(plan.duration_seconds, 21_600)
    assert.equal(plan.minimum_operator_duration_seconds, 21_600)
    assert.equal(plan.minimum_operator_measured_cycles, 30)
    assert.equal(plan.maximum_operator_sample_gap_ms, 900_000)
    assert.equal(plan.fresh_build_required, true)
    assert.equal(plan.operator_release_authority_requested, true)

    for (const [label, args, pattern] of [
      ['short duration', ['-Mode', 'operator', '-Hours', '5'], /Hours >= 6/],
      ['skip build', ['-Mode', 'operator', '-SkipBuild'], /fresh build/],
      [
        'development profile',
        ['-Mode', 'operator', '-BuildProfile', 'dev'],
        /BuildProfile=release/,
      ],
      [
        'explicit cycle limit',
        ['-Mode', 'operator', '-Cycles', '100'],
        /does not allow an explicit Cycles limit/,
      ],
      [
        'undersized workload',
        ['-Mode', 'operator', '-Rows', '512'],
        /Rows >= 20000/,
      ],
      [
        'weak cancellation cadence',
        ['-Mode', 'operator', '-CancelEvery', '3'],
        /CancelEvery=2/,
      ],
      [
        'noncanonical warmup',
        ['-Mode', 'operator', '-WarmupCycles', '2'],
        /WarmupCycles=1/,
      ],
    ]) {
      const result = runPlan([...args, '-CargoTargetDir', targetDir])
      assert.notEqual(result.status, 0, `${label} unexpectedly passed`)
      assert.match(
        `${String(result.stdout)}\n${String(result.stderr)}`,
        pattern,
        `${label} failed for the wrong reason`,
      )
    }
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true })
  }
})

test('wrapper freezes offline provenance and interruption-recovery controls', () => {
  const wrapper = read('scripts/run_cpu_reliability_soak.ps1')
  assert.match(wrapper, /Enter-BenchmarkLock -Context "CPU reliability soak build and run"/)
  assert.match(wrapper, /Exit-BenchmarkLock -Handle \$benchmarkLock/)
  assert.match(wrapper, /"build",\s*"--locked",\s*"--offline"/)
  assert.match(wrapper, /\$env:CARGO_TARGET_DIR = \$resolvedCargoTargetDir/)
  assert.match(wrapper, /Assert-NoReparseAncestors/)
  assert.match(wrapper, /FileAttributes\]::ReparsePoint/)
  assert.match(wrapper, /Recover-StaleSoakIsolations/)
  assert.match(wrapper, /Recover-StaleSoakCargoTargets/)
  assert.match(wrapper, /Remove-OwnedSoakIsolation/)
  assert.match(wrapper, /Remove-OwnedSoakCargoTarget/)
  assert.match(wrapper, /vectorgrid_cpu_reliability_soak_isolation/)
  assert.match(wrapper, /Fresh evidence path unexpectedly already exists/)
  assert.match(wrapper, /evidence predates this invocation/)
  assert.match(wrapper, /Get-SoakSourceManifest/)
  assert.match(wrapper, /vendor\/vector-ta-0\.2\.4/)
  assert.doesNotMatch(wrapper, /\[Convert\]::ToHexString/)
  assert.doesNotMatch(wrapper, /\[System\.IO\.Path\]::GetRelativePath/)
  assert.match(wrapper, /source inputs changed during the build/)
  assert.match(wrapper, /--message-format=json-render-diagnostics/)
  assert.match(wrapper, /Cargo did not emit exactly one ta_cpu_reliability_soak executable artifact/)
  assert.match(wrapper, /evidence configuration differs from the launched contract/)
  assert.match(wrapper, /workflow-integrity evidence was incomplete/)
  assert.match(wrapper, /binary changed during execution/)
  assert.match(wrapper, /launcher manifest changed during execution/)
  assert.match(wrapper, /provenance\.launcher_manifest\.matched_expected/)
  assert.match(wrapper, /compiler_override_environment_cleared/)
  assert.match(wrapper, /profile_override_environment_cleared/)
  assert.match(wrapper, /VECTORBT_CPU_RAM_BUDGET_MB = "64"/)
  assert.match(wrapper, /Get-SoakToolchainProvenance/)
  assert.match(wrapper, /Get-SoakCargoConfigurationProvenance/)
  assert.match(wrapper, /Release-authority soak refuses Cargo configuration outside/)
  assert.match(wrapper, /"--target-dir", \$resolvedCargoTargetDir/)
  assert.match(wrapper, /"--target", \$cargoTargetTriple/)
  assert.match(wrapper, /\$cargoExecutablePath = \[string\]\$toolchainProvenance\.cargo_path/)
  assert.match(wrapper, /Fresh isolated Cargo target unexpectedly reused/)
  assert.match(wrapper, /TAURI_CONFIG/)
  assert.match(wrapper, /TAURI_ENV_TARGET_TRIPLE/)
  assert.match(wrapper, /dev_url_no_embedded_frontend_assets/)
  assert.match(wrapper, /"RUSTC_WORKSPACE_WRAPPER"/)
  assert.match(wrapper, /"CARGO_ENCODED_RUSTFLAGS"/)
  assert.match(wrapper, /\^CARGO_TARGET_\.\+_\(RUSTFLAGS\|LINKER\)\$/)
  assert.match(wrapper, /POLYGON_API_KEY/)
  assert.match(wrapper, /HTTP_PROXY = "http:\/\/127\.0\.0\.1:9"/)
  assert.match(wrapper, /VECTORGRID_DESKTOP_SMOKE_APP_DATA_OVERRIDE = "1"/)
})

test('native evidence uses full-run online trends and fail-closed operator authority', () => {
  const runner = read('src-tauri/src/bin/ta_cpu_reliability_soak.rs')
  assert.match(runner, /const MIN_RELIABILITY_ROWS: usize = 2_048/)
  assert.match(runner, /if cli\.rows < MIN_RELIABILITY_ROWS/)
  assert.match(runner, /const MIN_OPERATOR_DURATION_SECONDS: u64 = 6 \* 60 \* 60/)
  assert.match(runner, /const MIN_OPERATOR_MEASURED_CYCLES: usize = 30/)
  assert.match(runner, /const MAX_OPERATOR_SAMPLE_GAP_MS: u64 = 15 \* 60 \* 1_000/)
  assert.match(runner, /struct TrendAccumulator/)
  assert.match(runner, /expected_samples: usize/)
  assert.match(runner, /sum_products_xy: f64/)
  assert.match(runner, /full-run growth and online least-squares/)
  assert.match(runner, /measured_samples == trend\.expected_samples/)
  assert.match(runner, /monitoring_tracker\.push\(&sample, config\.warmup_cycles\)/)
  assert.match(runner, /duration_contract_passed/)
  assert.match(runner, /operator_sampling_contract_passed/)
  assert.match(runner, /cancellation_probes_expected > 0/)
  assert.match(runner, /struct CancelOnPartialProgress/)
  assert.match(runner, /payload\.processed_pairs > 0/)
  assert.match(runner, /payload\.processed_pairs < payload\.total_pairs/)
  assert.match(runner, /cancellation_optimization_request/)
  assert.match(runner, /positive_partial_progress_probes_observed/)
  assert.doesNotMatch(runner, /to_ascii_lowercase\(\)\.contains\("cancel"\)/)
  assert.match(runner, /const EXPECTED_OPTIMIZATION_COMBINATIONS: usize = 240/)
  assert.match(runner, /candidates\.len\(\) != config\.validation_candidates/)
  assert.match(runner, /validation\.candidates\.len\(\) != config\.validation_candidates/)
  assert.match(runner, /struct OwnedStorageTracker/)
  assert.match(runner, /combined_owned_storage_stats/)
  assert.match(runner, /config\.isolation_root\.as_path\(\)/)
  assert.match(runner, /directory_count: TrendAccumulator/)
  assert.match(runner, /MAX_OWNED_STORAGE_DIRECTORY_GROWTH/)
  assert.match(runner, /max_residual_artifact_or_temp_files/)
  assert.match(runner, /operator_release_eligible/)
  assert.match(runner, /!config\.cargo_artifact_fresh/)
  assert.match(runner, /schema_version: 2/)
  assert.match(
    runner,
    /load_execution_provenance\(&config, benchmark_mutex_contention_verified\)/,
  )
  assert.match(runner, /verify_inherited_benchmark_mutex_contention/)
  assert.match(runner, /"mutex_contention_verified": benchmark_mutex_contention_verified/)
  assert.match(runner, /validate_source_manifest/)
  assert.match(runner, /validate_launcher_manifest/)
  assert.match(runner, /validate_cargo_configuration_provenance/)
  assert.match(runner, /validate_toolchain_input_manifest/)
  assert.match(runner, /compiler_override_environment_is_clear/)
  assert.match(runner, /compiler override environment must be cleared/)
  assert.match(runner, /metadata_is_reparse/)
  assert.match(runner, /ensure_no_reparse_tree/)
  assert.match(runner, /build_isolated_app\(&requested_app_data_dir\)/)
  assert.match(runner, /let app_state = app\.state::<AppState>\(\)/)
  assert.match(runner, /app_state\.load_price_data/)
  assert.match(runner, /LoadSource::CsvParsedAndCached/)
  assert.match(runner, /LoadSource::InMemoryBaseReuse/)
  assert.match(runner, /state\.get_candles_for_run/)
  assert.match(runner, /state\.try_begin_run\(\)/)
  assert.match(runner, /state\.cancel_token\(\)/)
  assert.match(runner, /app_state\.invalidate_dataset/)
  assert.match(runner, /"single_tauri_managed_app_state_full_run"/)
  assert.match(runner, /context\.config_mut\(\)\.identifier = app_data_dir/)
  assert.match(runner, /canonical_resolved_app_data_dir != canonical_requested_app_data_dir/)
  assert.match(runner, /source manifest aggregate mismatch/)
  assert.match(runner, /cargo_artifact_path != executable/)
  assert.match(runner, /external_http_proxy_is_trapped/)
  assert.match(runner, /credential_environment_is_clear/)
  assert.match(runner, /repo_dotenv_credential_fallback_disabled/)
})
