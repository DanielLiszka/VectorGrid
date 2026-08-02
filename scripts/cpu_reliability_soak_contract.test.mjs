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

test('repo gates execute vendored tests without absorbing upstream formatting policy', () => {
  const qualityGate = read('scripts/run_repo_quality_gate.ps1')
  const backendGate = read('scripts/run_backend_release_gate.ps1')

  for (const gate of [qualityGate, backendGate]) {
    assert.doesNotMatch(gate, /cargo fmt --all/)
    for (const packageName of [
      'ta_desktop_demo_app',
      'ta_app_core',
      'ta_strategies',
      'ta_optimizer',
    ]) {
      assert.match(gate, new RegExp(`-p ${packageName}`))
    }
  }
  assert.match(
    qualityGate,
    /"clippy",[\s\S]*"--workspace",[\s\S]*"--exclude", "embed-resource"/,
  )
  assert.match(qualityGate, /cargo check --workspace/)
  assert.match(qualityGate, /cargo test --workspace -j 1/)
})

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
    assert.equal(summary.includes_vendored_embed_resource, true)
    assert.equal(summary.includes_vendored_vector_ta, true)
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true })
  }
})

test('vendored embed-resource pins rc only through the attested opt-in override', () => {
  const workspaceManifest = read('Cargo.toml')
  const localPatches = read('vendor/embed-resource-3.0.6/LOCAL_PATCHES.md')
  const resourceCompiler = read(
    'vendor/embed-resource-3.0.6/src/windows_msvc.rs',
  )
  assert.match(
    workspaceManifest,
    /members\s*=\s*\[[\s\S]*"vendor\/embed-resource-3\.0\.6"/,
  )
  assert.match(
    workspaceManifest,
    /embed-resource\s*=\s*\{\s*path\s*=\s*"vendor\/embed-resource-3\.0\.6"\s*\}/,
  )
  assert.doesNotMatch(
    workspaceManifest,
    /exclude\s*=\s*\[[\s\S]*"vendor\/embed-resource-3\.0\.6"/,
  )
  assert.match(localPatches, /upstream git revision/)
  assert.match(localPatches, /reparse point on either the file or any ancestor/)
  assert.match(localPatches, /workspace member/)
  assert.match(localPatches, /EMBED_RESOURCE_RC_ATTESTATION_NONCE/)
  assert.match(localPatches, /VECTORGRID_EMBED_RESOURCE_RC_ATTESTATION/)
  assert.match(localPatches, /After `rc\.exe` exits successfully/)
  assert.match(localPatches, /Failed compiler invocations never emit/)
  assert.match(resourceCompiler, /EMBED_RESOURCE_RC_PATH/)
  assert.match(resourceCompiler, /EMBED_RESOURCE_RC_ATTESTATION_NONCE/)
  assert.match(resourceCompiler, /VECTORGRID_EMBED_RESOURCE_RC_ATTESTATION/)
  assert.match(resourceCompiler, /cargo:rerun-if-env-changed=/)
  assert.match(resourceCompiler, /env::var_os\(RESOURCE_COMPILER_OVERRIDE_ENV\)/)
  assert.match(
    resourceCompiler,
    /env::var_os\(RESOURCE_COMPILER_ATTESTATION_NONCE_ENV\)/,
  )
  assert.match(
    resourceCompiler,
    /RESOURCE_COMPILER_ATTESTATION_NONCE_ENV} requires {RESOURCE_COMPILER_OVERRIDE_ENV/,
  )
  assert.match(resourceCompiler, /must be a 32-character lowercase hexadecimal nonce/)
  assert.match(resourceCompiler, /!path\.is_absolute\(\)/)
  assert.match(resourceCompiler, /fs::symlink_metadata\(&path\)/)
  assert.match(resourceCompiler, /path_has_reparse_ancestor\(&path\)/)
  assert.match(resourceCompiler, /FILE_ATTRIBUTE_REPARSE_POINT/)
  assert.match(
    resourceCompiler,
    /explicit_resource_compiler_rejects_a_reparse_ancestor_when_supported/,
  )
  assert.match(resourceCompiler, /Command::new\(&resource_compiler\)/)
  const successfulExitCheck = resourceCompiler.indexOf('.success()')
  const successMarkerEmission = resourceCompiler.indexOf(
    'cargo:rustc-env={RESOURCE_COMPILER_ATTESTATION_RUSTC_ENV}={value}',
  )
  assert.ok(successfulExitCheck >= 0)
  assert.ok(successMarkerEmission > successfulExitCheck)
  assert.match(
    resourceCompiler,
    /resource_compiler_attestation_nonce_and_value_are_canonical/,
  )
  assert.match(
    resourceCompiler,
    /find_windows_sdk_tool_impl\("rc\.exe"\)\.unwrap_or_else/,
  )
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
  assert.match(wrapper, /\$runtimeClearedPinnedNativeEnvironmentNames/)
  assert.match(wrapper, /Pinned native environment must contain exactly one PATH entry/)
  assert.match(
    wrapper,
    /!\[string\]::Equals\(\[string\]\$_, "PATH", \[StringComparison\]::OrdinalIgnoreCase\)/,
  )
  assert.match(wrapper, /Ambient PATH was not restored exactly after the native build/)
  assert.match(
    wrapper,
    /\$targetSpecificBuildEnvNames \+ \$nativeBuildOverrideEnvNames \+\s+\$runtimeClearedPinnedNativeEnvironmentNames/,
  )
  assert.match(wrapper, /VECTORBT_CPU_RAM_BUDGET_MB = "64"/)
  assert.match(wrapper, /Get-SoakToolchainProvenance/)
  assert.match(wrapper, /Get-SoakCargoConfigurationProvenance/)
  assert.match(wrapper, /Release-authority soak refuses Cargo configuration outside/)
  assert.match(wrapper, /"--target-dir", \$resolvedCargoTargetDir/)
  assert.match(wrapper, /"--target", \$cargoTargetTriple/)
  assert.match(wrapper, /\$cargoExecutablePath = \[string\]\$toolchainProvenance\.cargo_path/)
  assert.match(wrapper, /Fresh isolated Cargo target unexpectedly reused/)
  assert.match(wrapper, /Get-SoakNativeInputManifest/)
  assert.match(
    wrapper,
    /schema_version = 3\s+kind = "vectorgrid_cpu_reliability_soak_native_inputs"/,
  )
  assert.match(
    wrapper,
    /sha256\(path_environment_tab_name_tab_value_sha256_then_root_tab_label_tab_absolute_path_then_file_tab_root_label_tab_relative_path_tab_bytes_tab_sha256_lf\)/,
  )
  assert.match(wrapper, /"path_environment`t\{0\}`t\{1\}"/)
  assert.match(wrapper, /"EXTERNAL_INCLUDE",\s*"IFCPATH",\s*"INCLUDE",\s*"LIB",\s*"LIBPATH"/)
  assert.match(wrapper, /if \(\$name -eq "IFCPATH"\)/)
  assert.match(wrapper, /Get-SoakExistingPathListDirectories/)
  assert.match(wrapper, /Test-SoakFullyQualifiedPath/)
  assert.match(wrapper, /ConvertTo-SoakAsciiLowerInvariant/)
  assert.match(wrapper, /StringComparer\]::Ordinal/)
  assert.match(wrapper, /if \(!\$directories\.ContainsKey\(\$key\)\)/)
  assert.match(wrapper, /environment_path_\{0:D4\}/)
  assert.match(wrapper, /"root`t\{0\}`t\{1\}"/)
  assert.match(wrapper, /vs_common7_tools/)
  assert.match(wrapper, /VsDevCmd\.bat/)
  assert.match(wrapper, /Visual Studio vsdevcmd tree/)
  assert.match(wrapper, /windows_sdk_bin/)
  assert.match(wrapper, /windows_sdk_include/)
  assert.match(wrapper, /windows_sdk_lib/)
  assert.match(wrapper, /windows_sdk_references/)
  assert.match(wrapper, /windows_sdk_union_metadata/)
  assert.match(wrapper, /-ConfigurePinnedEnvironment/)
  assert.match(wrapper, /vcvarsall\.bat/)
  assert.match(
    wrapper,
    /pinned_cmd_vcvarsall_plus_absolute_target_linker_plus_success_attested_rc_plus_locked_offline_build_chain/,
  )
  assert.match(wrapper, /CARGO_TARGET_\{0\}_LINKER/)
  assert.match(wrapper, /target_linker_path_pinned = \$true/)
  assert.match(wrapper, /target_linker_path_pinned_during_build/)
  assert.match(
    wrapper,
    /schema_version = 6\s+kind = "vectorgrid_cpu_reliability_soak_native_build_provenance"/,
  )
  assert.match(wrapper, /Resolve-SoakPathExecutable/)
  assert.match(wrapper, /vectorgrid_cpu_reliability_soak_path_tool_resolution/)
  assert.match(wrapper, /first_existing_ordinary_executable_in_recorded_path_order/)
  assert.match(wrapper, /path_tool_resolution = \$pathToolResolution/)
  assert.doesNotMatch(wrapper, /path_prefix_directories/)
  assert.match(wrapper, /Canonical Windows command processor/)
  assert.match(wrapper, /name = "cmd"/)
  assert.match(wrapper, /COMSPEC/)
  assert.match(wrapper, /EMBED_RESOURCE_RC_PATH/)
  assert.match(wrapper, /EMBED_RESOURCE_RC_ATTESTATION_NONCE/)
  assert.match(wrapper, /VECTORGRID_EMBED_RESOURCE_RC_ATTESTATION/)
  assert.match(wrapper, /resource_compiler_execution_pinned = \$true/)
  assert.match(wrapper, /resource_compiler_execution = \$resourceCompilerExecution/)
  assert.match(
    wrapper,
    /resource_compiler_success_attested = \$null -ne \$ResourceCompilerInvocation/,
  )
  assert.match(wrapper, /resource_compiler_write_delete_lock_required = \$true/)
  assert.match(wrapper, /resource_compiler_invocation = \$ResourceCompilerInvocation/)
  assert.match(wrapper, /vendor\/embed-resource-3\.0\.6/)
  assert.match(wrapper, /Get-SoakRegistryPackageTreeManifest/)
  assert.match(wrapper, /Get-SoakEmbedResourceCargoResolution/)
  assert.match(
    wrapper,
    /"metadata",\s*"--locked",\s*"--offline",\s*"--format-version",\s*"1",\s*"--filter-platform",\s*\$HostTargetTriple,\s*"--no-default-features",\s*"--features",\s*"ta_desktop_demo_app\/maintainer-tools"/,
  )
  assert.match(wrapper, /vectorgrid_cpu_reliability_soak_windows_build_dependency_chain/)
  assert.match(wrapper, /root_package_name_match_count = 1/)
  assert.match(wrapper, /tauri_build_package_name_match_count = 1/)
  assert.match(wrapper, /embed_resource_package_name_match_count = 1/)
  assert.match(wrapper, /tauri_winres_package_name_match_count = 1/)
  assert.match(wrapper, /source_tree = \$tauriBuildTree/)
  assert.match(wrapper, /source_tree = \$tauriWinresTree/)
  assert.match(wrapper, /root_to_tauri_build_declared_edge_match_count = 1/)
  assert.match(wrapper, /root_to_tauri_build_resolved_edge_match_count = 1/)
  assert.match(wrapper, /tauri_build_to_winres_declared_edge_match_count = 1/)
  assert.match(wrapper, /tauri_build_to_winres_resolved_edge_match_count = 1/)
  assert.match(wrapper, /winres_to_embed_declared_edge_match_count = 1/)
  assert.match(wrapper, /winres_to_embed_resolved_edge_match_count = 1/)
  assert.match(wrapper, /declared_dependency_name = "tauri-build"/)
  assert.match(wrapper, /resolved_dependency_name = "tauri_build"/)
  assert.match(wrapper, /declared_dependency_name = "tauri-winres"/)
  assert.match(wrapper, /resolved_dependency_name = "tauri_winres"/)
  assert.match(wrapper, /declared_dependency_name = "embed-resource"/)
  assert.match(wrapper, /resolved_dependency_name = "embed_resource"/)
  assert.match(wrapper, /declared_requirement = "\^3"/)
  assert.match(wrapper, /cargo_resolution_attested = \$true/)
  assert.match(wrapper, /cargo_resolution = \$cargoResolution/)
  assert.match(wrapper, /"cargo_resolution`t\$\(\$cargoResolution\.aggregate_sha256\)"/)
  assert.match(wrapper, /Locked\/offline Cargo metadata resolution changed during the build/)
  assert.match(wrapper, /resource_compiler_candidates/)
  assert.match(wrapper, /\$sdkVersionDirectories/)
  assert.match(wrapper, /\[System\.IO\.FileMode\]::Open/)
  assert.match(wrapper, /\[System\.IO\.FileAccess\]::Read/)
  assert.match(wrapper, /\[System\.IO\.FileShare\]::Read/)
  assert.match(wrapper, /deny-write\/delete lease/)
  assert.match(wrapper, /build-script-executed/)
  assert.match(wrapper, /rootBuildScriptEvents\.Count -ne 1/)
  assert.match(wrapper, /markerEvents\.Count -ne 1/)
  assert.match(
    wrapper,
    /Cargo did not emit one success-attested resource-compiler marker on the exact app build script/,
  )
  assert.match(
    wrapper,
    /\[string\]\$message\.package_id -eq\s*\[string\]\$nativeBuildProvenanceBefore\.cargo_resolution\.root_package\.package_id/,
  )
  assert.match(wrapper, /pre_execution_aggregate_sha256/)
  assert.match(wrapper, /ToUpperInvariant\(\)/)
  assert.match(wrapper, /case-insensitively unique/)
  assert.match(wrapper, /\(\?i:_\?VC_\|VCLIB_\|__DOTNET_\|__VCVARS\|VCVARS_\)/)
  assert.match(wrapper, /_\{0,2\}VSCMD/)
  assert.match(wrapper, /EXTERNAL_INCLUDE/)
  assert.match(wrapper, /Framework40Version/)
  assert.match(wrapper, /__DOTNET_ADD_64BIT/)
  assert.match(wrapper, /__DOTNET_PREFERRED_BITNESS/)
  assert.match(wrapper, /ICU4X_DATA_DIR/)
  assert.match(wrapper, /REMOVE_UNUSED_COMMANDS/)
  assert.match(wrapper, /__TAURI_WORKSPACE__/)
  assert.match(wrapper, /RUSTC_STAGE/)
  assert.match(wrapper, /LZMA_API_STATIC/)
  assert.match(wrapper, /STATIC_VCRUNTIME/)
  assert.match(wrapper, /CXXSTDLIB/)
  assert.match(wrapper, /RANLIBFLAGS/)
  assert.match(wrapper, /"PKG_CONFIG"/)
  assert.match(wrapper, /VCPKGRS/)
  assert.match(wrapper, /NO_VCPKG/)
  assert.match(wrapper, /LIBSQLITE3_SYS_USE_PKG_CONFIG/)
  assert.match(wrapper, /ZSTD_SYS_USE_PKG_CONFIG/)
  assert.match(wrapper, /RING_PREGENERATE_ASM/)
  assert.match(wrapper, /VCToolsRedistDir/)
  assert.match(wrapper, /VCIDEInstallDir/)
  assert.match(wrapper, /VS170COMNTOOLS/)
  assert.match(wrapper, /FrameworkDir64/)
  assert.match(wrapper, /\^VSCMD_/)
  assert.match(wrapper, /\$launcherFeatures = \[object\[\]\]@\("maintainer-tools"\)/)
  assert.match(
    wrapper,
    /Native compiler\/linker\/Windows SDK inputs changed during the build/,
  )
  assert.match(wrapper, /TAURI_CONFIG/)
  assert.match(wrapper, /TAURI_ENV_TARGET_TRIPLE/)
  assert.match(wrapper, /dev_url_no_embedded_frontend_assets/)
  assert.match(wrapper, /"RUSTC_WORKSPACE_WRAPPER"/)
  assert.match(wrapper, /"CARGO_ENCODED_RUSTFLAGS"/)
  assert.match(wrapper, /\^CARGO_TARGET_\.\+_\(RUSTFLAGS\|LINKER\)\$/)
  assert.match(wrapper, /POLYGON_API_KEY/)
  assert.match(wrapper, /HTTP_PROXY = "http:\/\/127\.0\.0\.1:9"/)
  assert.match(wrapper, /VECTORGRID_DESKTOP_SMOKE_APP_DATA_OVERRIDE = "1"/)
  assert.match(wrapper, /compiled_resource_compiler_attestation/)
  assert.match(wrapper, /matched_native_invocation/)
  assert.match(
    wrapper,
    /Compiled resource-compiler attestation does not match the exact native invocation/,
  )
})

test('native evidence uses full-run online trends and fail-closed operator authority', () => {
  const runner = read('src-tauri/src/bin/ta_cpu_reliability_soak.rs')
  const nativeInputValidator = runner.slice(
    runner.indexOf('fn validate_native_input_manifest'),
    runner.indexOf('fn collect_ordinary_tree_files'),
  )
  const nativeBuildValidator = runner.slice(
    runner.indexOf('fn validate_native_build_provenance'),
    runner.indexOf('fn validate_launcher_manifest'),
  )
  const nativeOverrideValidator = runner.slice(
    runner.indexOf('fn native_build_override_environment_is_clear'),
    runner.indexOf('fn build_environment_is_attested'),
  )
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
  assert.match(runner, /validate_native_input_manifest/)
  assert.match(nativeInputValidator, /Some\(3\)/)
  assert.match(
    nativeInputValidator,
    /sha256\(path_environment_tab_name_tab_value_sha256_then_root_tab_label_tab_absolute_path_then_file_tab_root_label_tab_relative_path_tab_bytes_tab_sha256_lf\)/,
  )
  assert.match(nativeInputValidator, /path_environment\\t\{name\}\\t\{value_sha256\}/)
  assert.match(nativeInputValidator, /root\\t\{label\}\\t\{path_text\}/)
  assert.match(
    nativeInputValidator,
    /file\\t\{root_label\}\\t\{relative\}\\t\{expected_bytes\}\\t\{expected_sha256\}/,
  )
  assert.match(runner, /existing_native_path_list_directories/)
  assert.match(
    runner,
    /native_input_absolute_path\(&root\.0, relative_path\)\?/,
  )
  assert.match(runner, /"EXTERNAL_INCLUDE", "IFCPATH", "INCLUDE", "LIB", "LIBPATH"/)
  assert.match(runner, /"vs_common7_tools"/)
  assert.match(runner, /VsDevCmd\.bat/)
  assert.match(runner, /Visual Studio vsdevcmd tree/)
  assert.match(runner, /environment_path_\{index:04\}/)
  assert.match(runner, /validate_embed_resource_cargo_resolution/)
  assert.match(nativeBuildValidator, /Some\(6\)/)
  const nativePreExecutionRows = [
    'host_target_triple\\t{}',
    'system_root\\t{}',
    'visual_studio_installation\\t{}',
    'vc_tools_version\\t{}',
    'windows_sdk_root\\t{}',
    'windows_sdk_version\\t{}',
    'tools\\t{}',
    'path_tool_resolution\\t{}',
    'resource_compiler_execution\\t{}',
    'resource_compiler_candidates\\t{}',
    'cargo_resolution\\t{}',
    'native_inputs\\t{}',
    'vcvars_environment_names\\t{}',
    'environment\\t{}',
  ]
  let priorNativeRowIndex = -1
  for (const row of nativePreExecutionRows) {
    const rowIndex = nativeBuildValidator.indexOf(row)
    assert.ok(rowIndex > priorNativeRowIndex, `native canonical row order: ${row}`)
    priorNativeRowIndex = rowIndex
  }
  assert.match(runner, /vectorgrid_cpu_reliability_soak_windows_build_dependency_chain/)
  assert.match(runner, /expected_path_package_id/)
  assert.match(runner, /tauri-build@2\.5\.6/)
  assert.match(runner, /vendor\/embed-resource-3\.0\.6\/Cargo\.toml/)
  assert.match(runner, /vendor\/embed-resource-3\.0\.6\/LOCAL_PATCHES\.md/)
  assert.match(runner, /tauri-winres@0\.3\.5/)
  assert.match(runner, /Cargo build-chain edge is invalid/)
  assert.match(runner, /validate_registry_package_tree/)
  assert.match(runner, /Cargo registry package file changed/)
  assert.match(runner, /cargo_resolution_attested/)
  assert.match(runner, /resource_compiler_candidate_set_attested/)
  assert.match(
    runner,
    /pinned_cmd_vcvarsall_plus_absolute_target_linker_plus_success_attested_rc_plus_locked_offline_build_chain/,
  )
  assert.match(runner, /cargo_target_linker_environment_name/)
  assert.match(runner, /target_linker_path_pinned/)
  assert.match(
    runner,
    /absolute Cargo target-linker environment does not match attested link\.exe/,
  )
  assert.match(runner, /parse_windows_sdk_version/)
  assert.match(runner, /enumerate_resource_compiler_candidates/)
  assert.match(
    runner,
    /native resource compiler candidate evidence omits an installed SDK candidate/,
  )
  assert.match(runner, /resource_compiler_execution_pinned/)
  assert.match(runner, /resource_compiler_success_attested/)
  assert.match(runner, /resource_compiler_write_delete_lock_required/)
  assert.match(runner, /EMBED_RESOURCE_RC_PATH/)
  assert.match(runner, /EMBED_RESOURCE_RC_ATTESTATION_NONCE/)
  assert.match(runner, /VECTORGRID_EMBED_RESOURCE_RC_ATTESTATION/)
  assert.match(
    runner,
    /option_env!\("VECTORGRID_EMBED_RESOURCE_RC_ATTESTATION"\)/,
  )
  assert.match(runner, /validate_compiled_resource_compiler_attestation/)
  assert.match(runner, /compiled_resource_compiler_attestation/)
  assert.match(runner, /matched_native_invocation/)
  assert.match(
    runner,
    /compiled resource-compiler attestation does not match the exact native invocation/,
  )
  assert.match(runner, /resource compiler success invocation does not match the exact app build/)
  assert.match(runner, /read_access_share_read_only_denies_write_delete/)
  assert.match(runner, /native pre-execution aggregate mismatch/)
  assert.match(runner, /pinned COMSPEC does not match attested system cmd\.exe/)
  assert.match(runner, /"cmd"/)
  assert.match(runner, /native resource compiler override environment hash/)
  assert.match(runner, /validate_path_tool_resolution/)
  assert.match(runner, /resolve_recorded_path_executable/)
  assert.match(runner, /first_existing_ordinary_executable_in_recorded_path_order/)
  assert.doesNotMatch(runner, /path_prefix_directories/)
  assert.match(runner, /native_build_override_environment_is_clear/)
  assert.match(nativeBuildValidator, /\*name != name\.to_ascii_uppercase\(\)/)
  assert.match(nativeOverrideValidator, /upper\.starts_with\("__DOTNET_"\)/)
  assert.match(nativeOverrideValidator, /upper\.starts_with\("VC_"\)/)
  assert.match(nativeOverrideValidator, /upper\.starts_with\("_VC_"\)/)
  assert.match(nativeOverrideValidator, /upper\.starts_with\("VCLIB_"\)/)
  assert.match(nativeOverrideValidator, /upper\.starts_with\("__VCVARS"\)/)
  assert.match(nativeOverrideValidator, /upper\.starts_with\("VCVARS_"\)/)
  assert.match(runner, /windows_ordinal_cmp/)
  assert.match(runner, /ICU4X_DATA_DIR/)
  assert.match(runner, /REMOVE_UNUSED_COMMANDS/)
  assert.match(runner, /__TAURI_WORKSPACE__/)
  assert.match(runner, /RUSTC_STAGE/)
  assert.match(runner, /LZMA_API_STATIC/)
  assert.match(runner, /STATIC_VCRUNTIME/)
  assert.match(runner, /CXXSTDLIB/)
  assert.match(runner, /RANLIBFLAGS/)
  assert.match(runner, /VCPKGRS_/)
  assert.match(runner, /_NO_VCPKG/)
  assert.match(runner, /LIBSQLITE3_SYS_USE_PKG_CONFIG/)
  assert.match(runner, /native resource compiler candidate SHA-256 changed/)
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
