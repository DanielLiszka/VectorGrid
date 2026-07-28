import { createHash } from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  isolatedAppDataPath,
  isPathInside,
  requireReleaseProofEnvironment,
} from './releaseProofEvidence.js'

const supportDir = path.dirname(fileURLToPath(import.meta.url))
const fixturesDir = path.resolve(supportDir, '..', 'fixtures')
const tinyModelBase64Path = path.resolve(
  fixturesDir,
  'ml-signal-tiny-model.cbm.b64',
)
const featureSchemaFixturePath = path.resolve(
  fixturesDir,
  'ml-signal-feature-schema.json',
)

export const RELEASE_PROOF_ML_BUNDLE_ID = 'release-proof-catboost-v1'
export const RELEASE_PROOF_OPTIONS_DATASET_ID = 'release-proof-options-v1'
export const RELEASE_PROOF_TINY_MODEL_SHA256 =
  'c01edc9c7045176436612ed7a5a144908259621acbfdd35aa7aa04c71a6f2d9e'
export const OFFICIAL_CATBOOST_NATIVE_SHA256 =
  '668d8867940a0832accaad6004e249f11ecc48eb63eb1d65551cfdd37c1939ad'

export type SeededMlSignalBundle = {
  bundleDir: string
  manifestPath: string
  modelPath: string
  featureSchemaPath: string
  corruptLibraryPath: string
}

function sha256Bytes(bytes: Buffer): string {
  return createHash('sha256').update(bytes).digest('hex')
}

export function sha256File(filePath: string): string {
  return sha256Bytes(fs.readFileSync(filePath))
}

function requireOrdinaryFile(filePath: string, label: string): void {
  if (!fs.existsSync(filePath)) {
    throw new Error(`${label} is missing: ${filePath}`)
  }
  const stat = fs.lstatSync(filePath)
  if (!stat.isFile() || stat.isSymbolicLink()) {
    throw new Error(`${label} must be an ordinary file: ${filePath}`)
  }
}

function ensureOwnedDirectory(dir: string): void {
  const { isolationRoot } = requireReleaseProofEnvironment()
  if (!isPathInside(dir, isolationRoot)) {
    throw new Error(`Fixture directory escaped the isolated profile: ${dir}`)
  }
  fs.mkdirSync(dir, { recursive: true })
  const stat = fs.lstatSync(dir)
  if (!stat.isDirectory() || stat.isSymbolicLink()) {
    throw new Error(`Fixture directory must be ordinary: ${dir}`)
  }
}

function tinyModelBytes(): Buffer {
  requireOrdinaryFile(tinyModelBase64Path, 'Tiny CatBoost fixture')
  const encoded = fs.readFileSync(tinyModelBase64Path, 'utf8').replace(/\s+/g, '')
  const bytes = Buffer.from(encoded, 'base64')
  const observed = sha256Bytes(bytes)
  if (
    bytes.length !== 15_920 ||
    observed !== RELEASE_PROOF_TINY_MODEL_SHA256
  ) {
    throw new Error(
      `Tiny CatBoost fixture identity mismatch: ${bytes.length} bytes / ${observed}`,
    )
  }
  return bytes
}

function featureSchemaBytes(): Buffer {
  requireOrdinaryFile(featureSchemaFixturePath, 'ML feature-schema fixture')
  const bytes = fs.readFileSync(featureSchemaFixturePath)
  const parsed = JSON.parse(bytes.toString('utf8')) as {
    schema_version?: unknown
    strategy_kind?: unknown
    feature_names?: unknown[]
    schema_hash?: unknown
  }
  if (
    parsed.schema_version !== 1 ||
    parsed.strategy_kind !== 'MlSignal' ||
    !Array.isArray(parsed.feature_names) ||
    parsed.feature_names.length !== 390 ||
    parsed.schema_hash !== 'fnv1a64-f2a9a5997d2ec85b'
  ) {
    throw new Error('ML feature-schema fixture does not match the release-proof contract.')
  }
  return bytes
}

function bundlePaths(): SeededMlSignalBundle {
  const bundleDir = isolatedAppDataPath(
    'models',
    'ml_signal',
    RELEASE_PROOF_ML_BUNDLE_ID,
  )
  return {
    bundleDir,
    manifestPath: path.resolve(bundleDir, 'manifest.json'),
    modelPath: path.resolve(bundleDir, 'model.cbm'),
    featureSchemaPath: path.resolve(bundleDir, 'feature_schema.json'),
    corruptLibraryPath: path.resolve(bundleDir, 'native', 'catboostmodel.dll'),
  }
}

function writeMlManifest(
  target: SeededMlSignalBundle,
  corruptLibraryOverride: boolean,
): void {
  const manifest = {
    bundle_id: RELEASE_PROOF_ML_BUNDLE_ID,
    schema_version: 1,
    feature_schema_version: 1,
    model_family: 'catboost_binary_pair',
    model_format: 'cbm',
    long_model_path: 'model.cbm',
    short_model_path: 'model.cbm',
    feature_schema_path: 'feature_schema.json',
    notes:
      'Deterministic offline WDIO release-proof fixture; never promoted as a research model.',
    catboost_version: '1.2.10',
    catboost_model_interface_version: 'catboost_c_api_raw_formula_v1',
    model_long_sha256: RELEASE_PROOF_TINY_MODEL_SHA256,
    model_short_sha256: RELEASE_PROOF_TINY_MODEL_SHA256,
    feature_schema_hash: 'fnv1a64-f2a9a5997d2ec85b',
    probability_semantics: 'uncalibrated_sigmoid_fallback',
    calibration_required: false,
    score_scope_policy: 'full_dataset_allowed',
    score_contract_version: 1,
    score_contract: {
      score_contract_version: 1,
      score_scope_policy: 'full_dataset_allowed',
      walkforward_recommended_mode: 'full_dataset_test_fixture_only',
    },
    training_data_id: 'release-proof-synthetic-fixture',
    training_symbol_scope: 'test-only',
    trained_by: 'deterministic CatBoost fixture generator',
    created_at_utc: '2026-07-28T00:00:00Z',
    ...(corruptLibraryOverride
      ? { native_library_path: 'native/catboostmodel.dll' }
      : {}),
  }
  fs.writeFileSync(
    target.manifestPath,
    `${JSON.stringify(manifest, null, 2)}\n`,
    'utf8',
  )
}

export function seedMlSignalBundle(options: {
  includeModel: boolean
  corruptLibraryOverride: boolean
}): SeededMlSignalBundle {
  const target = bundlePaths()
  ensureOwnedDirectory(target.bundleDir)
  ensureOwnedDirectory(path.dirname(target.corruptLibraryPath))
  fs.writeFileSync(target.featureSchemaPath, featureSchemaBytes())
  if (options.includeModel) {
    fs.writeFileSync(target.modelPath, tinyModelBytes())
  } else if (fs.existsSync(target.modelPath)) {
    fs.rmSync(target.modelPath)
  }
  if (options.corruptLibraryOverride) {
    fs.writeFileSync(
      target.corruptLibraryPath,
      Buffer.from('not-a-loadable-catboost-library', 'utf8'),
    )
  } else if (fs.existsSync(target.corruptLibraryPath)) {
    fs.rmSync(target.corruptLibraryPath)
  }
  writeMlManifest(target, options.corruptLibraryOverride)
  return target
}

export function writeOptionsDatasetFixture(): string {
  const target = isolatedAppDataPath(
    'qa-fixtures',
    'release-proof-options.csv',
  )
  ensureOwnedDirectory(path.dirname(target))
  const header = [
    'root_symbol',
    'contract_id',
    'option_type',
    'expiration_date',
    'expiration_timestamp_ms',
    'strike',
    'multiplier',
    'timestamp_ms',
    'open',
    'high',
    'low',
    'close',
    'underlying_price',
    'volume',
    'open_interest',
    'session_id',
  ].join(',')
  const timestamps = new Set<number>()
  for (let timestamp = 0; timestamp <= 9_540; timestamp += 60) {
    timestamps.add(timestamp)
    timestamps.add(timestamp + 60_000)
    timestamps.add(timestamp + 120_000)
  }
  const rows = [header]
  for (const strike of [100, 105, 110, 115, 120]) {
    for (const optionType of ['call', 'put'] as const) {
      const contractId = `VG_${optionType === 'call' ? 'C' : 'P'}_${String(strike)}`
      for (const timestamp of [...timestamps].sort((left, right) => left - right)) {
        const phase = timestamp / 60
        const underlying = 109 + Math.sin(phase / 17) * 9
        const intrinsic = optionType === 'call'
          ? Math.max(underlying - strike, 0)
          : Math.max(strike - underlying, 0)
        const close = 1.25 + intrinsic * 0.2 + ((phase % 23) * 0.015)
        const open = Math.max(0.05, close - 0.03)
        const high = close + 0.08
        const low = Math.max(0.01, close - 0.08)
        rows.push([
          'VG',
          contractId,
          optionType,
          '1970-01-02',
          '86400000',
          String(strike),
          '100',
          String(timestamp),
          open.toFixed(6),
          high.toFixed(6),
          low.toFixed(6),
          close.toFixed(6),
          underlying.toFixed(6),
          '1000',
          '2000',
          '1970-01-01',
        ].join(','))
      }
    }
  }
  fs.writeFileSync(target, `${rows.join('\n')}\n`, 'utf8')
  return target
}
