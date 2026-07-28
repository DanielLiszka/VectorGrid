import crypto from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.resolve(scriptDir, '..')
const defaultConfigPath = path.resolve(scriptDir, 'release_proof_journeys.json')

function fail(message) {
  throw new Error(message)
}

function normalizedPath(value) {
  let full = path.resolve(value)
  if (process.platform !== 'win32') return full
  full = full.replace(/\//g, '\\')
  if (full.startsWith('\\\\?\\UNC\\')) full = `\\\\${full.slice(8)}`
  else if (full.startsWith('\\\\?\\')) full = full.slice(4)
  return full.toLowerCase()
}

function samePath(left, right) {
  return normalizedPath(left) === normalizedPath(right)
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

function readOrdinaryFileSnapshot(
  filePath,
  label,
  maximumBytes = Number.MAX_SAFE_INTEGER,
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
    if (opened.size <= 0 || opened.size > maximumBytes) {
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
      size: opened.size,
      sha256: crypto.createHash('sha256').update(bytes).digest('hex'),
    }
  } finally {
    if (handle !== undefined) fs.closeSync(handle)
  }
}

function readJsonSnapshot(filePath, label, maximumBytes = 4 * 1024 * 1024) {
  const snapshot = readOrdinaryFileSnapshot(filePath, label, maximumBytes)
  try {
    return {
      ...snapshot,
      value: JSON.parse(snapshot.bytes.toString('utf8')),
    }
  } catch (error) {
    fail(`${label} is not valid JSON: ${filePath}: ${String(error)}`)
  }
}

function parseSafeSegment(value, label) {
  const normalized = String(value ?? '').trim()
  if (!/^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/.test(normalized)) {
    fail(`${label} must be one filename-safe segment.`)
  }
  return normalized
}

function parseSha256(value, label) {
  const normalized = String(value ?? '').trim().toLowerCase()
  if (!/^[a-f0-9]{64}$/.test(normalized)) fail(`${label} must be a SHA-256 hex digest.`)
  return normalized
}

function parsePositiveInteger(value, label) {
  const number = Number(value)
  if (!Number.isSafeInteger(number) || number <= 0) fail(`${label} must be a positive integer.`)
  return number
}

export function loadReleaseProofJourneyConfig(configPath = defaultConfigPath) {
  const resolved = path.resolve(configPath)
  const configSnapshot = readJsonSnapshot(
    resolved,
    'Release-proof journey allowlist',
  )
  const config = configSnapshot.value
  if (
    Number(config.schema_version) !== 1
    || config.desktop_suite !== 'release-proof'
    || !Array.isArray(config.journeys)
    || config.journeys.length === 0
  ) {
    fail('Release-proof journey allowlist has an unsupported schema.')
  }
  const ids = new Set()
  const specs = new Set()
  const evidenceFiles = new Set()
  const evidenceContracts = new Set()
  const journeys = config.journeys.map((raw, index) => {
    const id = parseSafeSegment(raw?.id, `journeys[${String(index)}].id`)
    const evidenceFile = parseSafeSegment(
      raw?.evidence_file,
      `journeys[${String(index)}].evidence_file`,
    )
    const evidenceContract = parseSafeSegment(
      raw?.evidence_contract,
      `journeys[${String(index)}].evidence_contract`,
    )
    const spec = String(raw?.spec ?? '').trim().replace(/\\/g, '/')
    if (
      !spec.startsWith('web/e2e/desktop/specs/')
      || !spec.endsWith('.spec.ts')
      || path.isAbsolute(spec)
      || spec.split('/').includes('..')
    ) {
      fail(`journeys[${String(index)}].spec is not a safe desktop spec path.`)
    }
    if (evidenceFile !== `${id}-proof.json`) {
      fail(
        `journeys[${String(index)}].evidence_file must be derived exactly from its journey id.`,
      )
    }
    if (
      ids.has(id)
      || specs.has(spec)
      || evidenceFiles.has(evidenceFile)
      || evidenceContracts.has(evidenceContract)
    ) {
      fail(`Release-proof journey allowlist contains a duplicate mapping: ${id}`)
    }
    ids.add(id)
    specs.add(spec)
    evidenceFiles.add(evidenceFile)
    evidenceContracts.add(evidenceContract)
    return {
      id,
      spec,
      evidence_file: evidenceFile,
      evidence_contract: evidenceContract,
    }
  })
  return {
    path: resolved,
    sha256: configSnapshot.sha256,
    schema_version: 1,
    desktop_suite: 'release-proof',
    journeys,
  }
}

function validateBinaryIdentity(identity, expected, label) {
  if (!identity || typeof identity !== 'object') fail(`${label} has no binaryIdentity object.`)
  const actualPath = String(identity.path ?? '').trim()
  const actualSha256 = parseSha256(identity.sha256, `${label}.binaryIdentity.sha256`)
  const actualSize = parsePositiveInteger(
    identity.sizeBytes,
    `${label}.binaryIdentity.sizeBytes`,
  )
  if (!samePath(actualPath, expected.path)) {
    fail(`${label} belongs to a foreign packaged binary path: ${actualPath}`)
  }
  if (actualSha256 !== expected.sha256 || actualSize !== expected.size_bytes) {
    fail(`${label} belongs to a stale or foreign packaged binary identity.`)
  }
}

function validateJourneyFile({
  journey,
  evidenceRoot,
  runId,
  variant,
  binaryIdentity,
}) {
  const evidencePath = path.resolve(evidenceRoot, journey.evidence_file)
  if (path.dirname(evidencePath) !== path.resolve(evidenceRoot)) {
    fail(`Journey evidence escaped its evidence root: ${journey.evidence_file}`)
  }
  const snapshot = readJsonSnapshot(
    evidencePath,
    `Journey evidence "${journey.id}"`,
  )
  const value = snapshot.value
  if (
    Number(value.schemaVersion) !== 2
    || value.contract !== 'vectorgrid-release-proof-journey-v2'
    || value.status !== 'passed'
    || value.runId !== runId
    || value.releaseVariant !== variant
    || value.journeyId !== journey.id
  ) {
    fail(
      `Journey evidence "${journey.id}" has a stale, failed, foreign-run, or unsupported contract.`,
    )
  }
  if (
    typeof value.generatedAt !== 'string'
    || !Number.isFinite(Date.parse(value.generatedAt))
    || !Object.hasOwn(value, 'evidence')
    || value.evidence === null
    || typeof value.evidence !== 'object'
    || Array.isArray(value.evidence)
    || value.evidence.contract !== journey.evidence_contract
    || Object.keys(value.evidence).length < 2
  ) {
    fail(
      `Journey evidence "${journey.id}" has no valid timestamped, journey-specific evidence payload.`,
    )
  }
  validateBinaryIdentity(value.binaryIdentity, binaryIdentity, `Journey evidence "${journey.id}"`)
  return {
    id: journey.id,
    spec: journey.spec,
    evidence_file: journey.evidence_file,
    evidence_contract: journey.evidence_contract,
    status: 'passed',
    schema_version: 2,
    run_id: runId,
    release_variant: variant,
    binary_identity: binaryIdentity,
    evidence_size_bytes: snapshot.size,
    evidence_sha256: snapshot.sha256,
  }
}

function validateUnexpectedProofFiles(evidenceRoot, allowedFiles, manifestFileName) {
  const unexpected = fs
    .readdirSync(evidenceRoot, { withFileTypes: true })
    .filter((entry) => {
      if (entry.name === manifestFileName) return false
      return entry.name.endsWith('-proof.json') && !allowedFiles.has(entry.name)
    })
    .map((entry) => entry.name)
    .sort()
  if (unexpected.length > 0) {
    fail(`Release-proof evidence root contains non-allowlisted proof files: ${unexpected.join(', ')}`)
  }
}

function validatedInputs(options) {
  const evidenceRoot = path.resolve(options.evidenceRoot)
  requireOrdinaryDirectory(evidenceRoot, 'Release-proof evidence root')
  const runId = parseSafeSegment(options.runId, 'run-id')
  const variant = parseSafeSegment(options.variant, 'variant')
  if (!['cpu', 'gpu-ma', 'gpu-kernel'].includes(variant)) {
    fail('variant must be cpu, gpu-ma, or gpu-kernel.')
  }
  const binaryPath = path.resolve(options.binaryPath)
  const binarySnapshot = readOrdinaryFileSnapshot(
    binaryPath,
    'Exact packaged binary',
  )
  const binaryIdentity = {
    path: binaryPath,
    size_bytes: parsePositiveInteger(options.binarySizeBytes, 'binary-size-bytes'),
    sha256: parseSha256(options.binarySha256, 'binary-sha256'),
  }
  if (
    binarySnapshot.size !== binaryIdentity.size_bytes
    || binarySnapshot.sha256 !== binaryIdentity.sha256
  ) {
    fail('The supplied packaged-binary identity does not match the current exact binary.')
  }
  const config = loadReleaseProofJourneyConfig(options.configPath)
  const manifestPath = path.resolve(options.manifestPath)
  if (
    path.dirname(manifestPath) !== evidenceRoot
    || path.basename(manifestPath) !== 'release-proof-evidence-manifest.json'
  ) {
    fail(
      'Release-proof evidence manifest must use its canonical filename directly under the evidence root.',
    )
  }
  return {
    evidenceRoot,
    runId,
    variant,
    binaryIdentity,
    config,
    manifestPath,
  }
}

function buildValidatedManifest(options) {
  const inputs = validatedInputs(options)
  const manifestFileName = path.basename(inputs.manifestPath)
  const allowedFiles = new Set(inputs.config.journeys.map((journey) => journey.evidence_file))
  validateUnexpectedProofFiles(inputs.evidenceRoot, allowedFiles, manifestFileName)
  const journeys = inputs.config.journeys.map((journey) =>
    validateJourneyFile({
      journey,
      evidenceRoot: inputs.evidenceRoot,
      runId: inputs.runId,
      variant: inputs.variant,
      binaryIdentity: inputs.binaryIdentity,
    }),
  )
  return {
    inputs,
    manifest: {
      schema_version: 1,
      contract: 'vectorgrid-release-proof-evidence-manifest-v1',
      status: 'passed',
      desktop_suite: 'release-proof',
      run_id: inputs.runId,
      release_variant: inputs.variant,
      binary_identity: inputs.binaryIdentity,
      journey_allowlist_path: inputs.config.path,
      journey_allowlist_sha256: inputs.config.sha256,
      required_journey_count: journeys.length,
      journeys,
    },
  }
}

function writeJsonAtomic(filePath, value) {
  if (fs.existsSync(filePath)) {
    fail(`Refusing to overwrite an existing release-proof evidence manifest: ${filePath}`)
  }
  const tempPath = `${filePath}.tmp-${String(process.pid)}-${crypto.randomBytes(6).toString('hex')}`
  try {
    fs.writeFileSync(tempPath, `${JSON.stringify(value, null, 2)}\n`, {
      encoding: 'utf8',
      flag: 'wx',
    })
    fs.renameSync(tempPath, filePath)
  } finally {
    if (fs.existsSync(tempPath)) fs.rmSync(tempPath)
  }
}

export function writeReleaseProofEvidenceManifest(options) {
  const { inputs, manifest } = buildValidatedManifest(options)
  writeJsonAtomic(inputs.manifestPath, manifest)
  const snapshot = readOrdinaryFileSnapshot(
    inputs.manifestPath,
    'Release-proof evidence manifest',
  )
  return {
    manifest,
    manifest_path: inputs.manifestPath,
    manifest_size_bytes: snapshot.size,
    manifest_sha256: snapshot.sha256,
  }
}

export function verifyReleaseProofEvidenceManifest(options) {
  const { inputs, manifest: expected } = buildValidatedManifest(options)
  const actualSnapshot = readJsonSnapshot(
    inputs.manifestPath,
    'Release-proof evidence manifest',
  )
  const actual = actualSnapshot.value
  const stableExpected = JSON.stringify(expected)
  const stableActual = JSON.stringify(actual)
  if (stableActual !== stableExpected) {
    fail('Release-proof evidence manifest no longer matches the allowlist and current evidence files.')
  }
  return {
    manifest: actual,
    manifest_path: inputs.manifestPath,
    manifest_size_bytes: actualSnapshot.size,
    manifest_sha256: actualSnapshot.sha256,
  }
}

function parseArgs(argv) {
  const result = {}
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index]
    if (!token.startsWith('--')) fail(`Unexpected argument: ${token}`)
    const key = token.slice(2)
    const value = argv[index + 1]
    if (!value || value.startsWith('--')) fail(`Missing value for --${key}`)
    result[key] = value
    index += 1
  }
  return result
}

function cliOptions(raw) {
  const required = [
    'mode',
    'evidence-root',
    'run-id',
    'variant',
    'binary-path',
    'binary-size-bytes',
    'binary-sha256',
    'manifest-path',
  ]
  for (const name of required) {
    if (!raw[name]) fail(`Missing required --${name}.`)
  }
  return {
    mode: raw.mode,
    evidenceRoot: raw['evidence-root'],
    runId: raw['run-id'],
    variant: raw.variant,
    binaryPath: raw['binary-path'],
    binarySizeBytes: raw['binary-size-bytes'],
    binarySha256: raw['binary-sha256'],
    manifestPath: raw['manifest-path'],
    configPath: raw['config'] || defaultConfigPath,
  }
}

const invokedPath = process.argv[1] ? path.resolve(process.argv[1]) : ''
if (invokedPath && samePath(invokedPath, fileURLToPath(import.meta.url))) {
  try {
    const options = cliOptions(parseArgs(process.argv.slice(2)))
    const result =
      options.mode === 'write'
        ? writeReleaseProofEvidenceManifest(options)
        : options.mode === 'verify'
          ? verifyReleaseProofEvidenceManifest(options)
          : fail('--mode must be write or verify.')
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`)
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`)
    process.exitCode = 1
  }
}
