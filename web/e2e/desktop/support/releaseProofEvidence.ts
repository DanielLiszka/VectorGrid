import { createHash } from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { ensureDir, repoRoot } from './appPaths.js'

let cachedBinaryIdentity: {
  path: string
  sizeBytes: number
  sha256: string
} | null = null

function normalized(value: string): string {
  let full = path.resolve(value)
  if (process.platform !== 'win32') return full
  full = full.replace(/\//g, '\\')
  if (full.startsWith('\\\\?\\UNC\\')) full = `\\\\${full.slice(8)}`
  else if (full.startsWith('\\\\?\\')) full = full.slice(4)
  return full.toLowerCase()
}

export function isPathInside(candidate: string, root: string): boolean {
  const candidateValue = normalized(candidate)
  const rootValue = normalized(root)
  return (
    candidateValue === rootValue ||
    candidateValue.startsWith(`${rootValue}${path.sep}`)
  )
}

export function isReleaseProofRun(): boolean {
  return process.env.VECTORGRID_DESKTOP_SMOKE_EXACT_PACKAGE === '1'
}

export function requireReleaseProofEnvironment(): {
  runId: string
  releaseVariant: 'cpu' | 'gpu-ma' | 'gpu-kernel'
  isolationRoot: string
  appDataDir: string
  desktopBinary: string
} {
  if (process.env.VECTORGRID_DESKTOP_SMOKE_EXACT_PACKAGE !== '1') {
    throw new Error(
      'This spec requires the exact-package release-proof wrapper.',
    )
  }
  const runId = String(process.env.VECTORGRID_DESKTOP_SMOKE_RUN_ID || '').trim()
  if (!/^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/.test(runId)) {
    throw new Error('The release-proof run id is missing or unsafe.')
  }
  const isolationRoot = String(
    process.env.VECTORGRID_DESKTOP_SMOKE_ISOLATED_PROFILE_ROOT || '',
  ).trim()
  if (!path.isAbsolute(isolationRoot)) {
    throw new Error('The release-proof isolation root must be absolute.')
  }
  const outputRoot = path.resolve(repoRoot, 'output')
  if (
    !isPathInside(isolationRoot, outputRoot) ||
    normalized(isolationRoot) === normalized(outputRoot)
  ) {
    throw new Error(
      `Isolation root escaped the repository output directory: ${isolationRoot}`,
    )
  }
  if (process.env.VECTORGRID_DESKTOP_SMOKE_APP_DATA_OVERRIDE !== '1') {
    throw new Error(
      'The exact-package QA app-data override handshake is not enabled.',
    )
  }
  const appDataDir = String(
    process.env.VECTORGRID_DESKTOP_SMOKE_APP_DATA_DIR || '',
  ).trim()
  const expectedAppDataDir = path.resolve(
    isolationRoot,
    'LocalAppData',
    'dev.vectoralpha.vectorgrid',
  )
  if (
    !path.isAbsolute(appDataDir) ||
    normalized(appDataDir) !== normalized(expectedAppDataDir) ||
    !isPathInside(appDataDir, isolationRoot)
  ) {
    throw new Error(
      `The exact-package app-data directory is not disposable: ${appDataDir}`,
    )
  }
  const desktopBinary = String(
    process.env.VECTORGRID_DESKTOP_SMOKE_BINARY || '',
  ).trim()
  if (!path.isAbsolute(desktopBinary) || !fs.existsSync(desktopBinary)) {
    throw new Error('The exact-package desktop binary is missing.')
  }
  const releaseVariant = String(
    process.env.VECTORGRID_DESKTOP_SMOKE_VARIANT || '',
  ).trim()
  if (
    releaseVariant !== 'cpu'
    && releaseVariant !== 'gpu-ma'
    && releaseVariant !== 'gpu-kernel'
  ) {
    throw new Error(
      `The exact-package release variant is missing or invalid: ${releaseVariant}`,
    )
  }
  return {
    runId,
    releaseVariant,
    isolationRoot: path.resolve(isolationRoot),
    appDataDir: path.resolve(appDataDir),
    desktopBinary: path.resolve(desktopBinary),
  }
}

export function releaseProofEvidenceRoot(): string {
  const { runId } = requireReleaseProofEnvironment()
  return ensureDir(
    path.resolve(
      repoRoot,
      'output',
      'wdio',
      'desktop-smoke',
      runId,
      'logs',
      'release-proof',
    ),
  )
}

export function releaseProofEvidencePath(fileName: string): string {
  const safeName = String(fileName || '').trim()
  if (!/^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/.test(safeName)) {
    throw new Error(
      `Unsafe release-proof evidence filename: ${JSON.stringify(fileName)}`,
    )
  }
  return path.resolve(releaseProofEvidenceRoot(), safeName)
}

export function isolatedAppDataPath(...segments: string[]): string {
  const { isolationRoot, appDataDir } = requireReleaseProofEnvironment()
  const target = path.resolve(appDataDir, ...segments)
  if (
    !isPathInside(target, appDataDir) ||
    !isPathInside(target, isolationRoot)
  ) {
    throw new Error(
      `Release-proof target escaped the isolated profile: ${target}`,
    )
  }
  return target
}

export function writeReleaseProofEvidence(
  fileName: string,
  value: unknown,
): string {
  const target = releaseProofEvidencePath(fileName)
  const environment = requireReleaseProofEnvironment()
  if (
    !value
    || typeof value !== 'object'
    || Array.isArray(value)
    || typeof (value as Record<string, unknown>).contract !== 'string'
    || Object.keys(value).length < 2
  ) {
    throw new Error(
      `Release-proof evidence must be a nonempty journey-specific object: ${fileName}`,
    )
  }
  if (!cachedBinaryIdentity) {
    const stat = fs.lstatSync(environment.desktopBinary)
    if (!stat.isFile() || stat.isSymbolicLink()) {
      throw new Error(
        `The exact-package binary must be an ordinary file: ${environment.desktopBinary}`,
      )
    }
    cachedBinaryIdentity = {
      path: environment.desktopBinary,
      sizeBytes: stat.size,
      sha256: createHash('sha256')
        .update(fs.readFileSync(environment.desktopBinary))
        .digest('hex'),
    }
  }
  const journeyId = fileName.replace(/-proof\.json$/i, '')
  if (!journeyId || journeyId === fileName) {
    throw new Error(
      `Release-proof evidence must use a *-proof.json filename: ${fileName}`,
    )
  }
  const document = {
    schemaVersion: 2,
    contract: 'vectorgrid-release-proof-journey-v2',
    journeyId,
    status: 'passed',
    generatedAt: new Date().toISOString(),
    runId: environment.runId,
    releaseVariant: environment.releaseVariant,
    binaryIdentity: cachedBinaryIdentity,
    evidence: value,
  }
  fs.writeFileSync(
    target,
    `${JSON.stringify(document, null, 2)}\n`,
    {
      encoding: 'utf8',
      flag: 'wx',
    },
  )
  return target
}
