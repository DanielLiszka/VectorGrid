import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import type { DesktopArtifactContext } from './artifacts.js'
import { ensureDir, repoRoot } from './appPaths.js'
import { writeTextFile } from './artifacts.js'

const APP_IDENTIFIER = 'dev.vectoralpha.vectorgrid'
const PROFILE_SCHEMA_VERSION = 2
const PROFILE_MARKER_NAME = '.vectorgrid-desktop-smoke-profile.json'
const PROFILE_MARKER_KIND = 'vectorgrid-desktop-smoke-profile'
const PROFILE_ATTESTATION_NAME = '.vectorgrid-qa-profile-attestation.json'
const PROFILE_ATTESTATION_KIND = 'vectorgrid-qa-profile-attestation'
const PROFILE_OVERRIDE_ENV = 'VECTORGRID_DESKTOP_SMOKE_APP_DATA_OVERRIDE'
const PROFILE_APP_DATA_ENV = 'VECTORGRID_DESKTOP_SMOKE_APP_DATA_DIR'
const MAX_SNAPSHOT_FILES = 25_000
const MAX_SNAPSHOT_BYTES = 2 * 1024 * 1024 * 1024

type EnvironmentSnapshot = {
  LOCALAPPDATA?: string
  APPDATA?: string
  TEMP?: string
  TMP?: string
  USERPROFILE?: string
  HOME?: string
  VECTORGRID_DESKTOP_SMOKE_ISOLATED_PROFILE_ROOT?: string
  VECTORGRID_DESKTOP_SMOKE_APP_DATA_OVERRIDE?: string
  VECTORGRID_DESKTOP_SMOKE_APP_DATA_DIR?: string
}

type WindowsKnownFolders = {
  localAppData: string
  roamingAppData: string
  userProfile: string
}

type ProfileMarker = {
  schemaVersion: typeof PROFILE_SCHEMA_VERSION
  kind: typeof PROFILE_MARKER_KIND
  runId: string
  appIdentifier: typeof APP_IDENTIFIER
  appDataDir: string
  createdAt: string
}

type ProfileAttestation = {
  schemaVersion: typeof PROFILE_SCHEMA_VERSION
  kind: typeof PROFILE_ATTESTATION_KIND
  runId: string
  markerCreatedAt: string
  originalIdentifier: typeof APP_IDENTIFIER
  effectiveIdentifier: string
  isolationRoot: string
  appDataDir: string
  resolvedPaths: {
    appConfigDir: string
    appDataDir: string
    appLocalDataDir: string
    appCacheDir: string
    appLogDir: string
    webviewDataDir: string
  }
}

type SnapshotEntry = {
  relativePath: string
  kind: 'directory' | 'file'
  sizeBytes: number
  sha256: string
}

type RootSnapshot = {
  root: string
  state: 'missing' | 'present'
  entries: SnapshotEntry[]
  fileCount: number
  totalBytes: number
}

export type DesktopProfileIsolation = {
  root: string
  localAppData: string
  roamingAppData: string
  temp: string
  userProfile: string
  appDataDir: string
  markerPath: string
  attestationPath: string
  markerCreatedAt: string
  evidencePath: string
  runId: string
  originalEnvironment: EnvironmentSnapshot
  windowsKnownFolders: WindowsKnownFolders | null
  protectedRoots: string[]
  before: RootSnapshot[]
}

export type DesktopProfileIsolationSummary = {
  enabled: boolean
  root: string
  realProfileUnchanged: boolean
  evidencePath: string
  status: 'not_enabled' | 'passed' | 'blocked'
  reason?: string
}

function normalizeForComparison(value: string): string {
  let full = path.resolve(value)
  if (process.platform !== 'win32') return full
  full = full.replace(/\//g, '\\')
  if (full.startsWith('\\\\?\\UNC\\')) full = `\\\\${full.slice(8)}`
  else if (full.startsWith('\\\\?\\')) full = full.slice(4)
  return full.toLowerCase()
}

function isPathInside(candidate: string, root: string): boolean {
  const candidateValue = normalizeForComparison(candidate)
  const rootValue = normalizeForComparison(root)
  return (
    candidateValue === rootValue ||
    candidateValue.startsWith(`${rootValue}${path.sep}`)
  )
}

function assertOrdinaryDirectory(candidate: string, purpose: string): void {
  const stat = fs.lstatSync(candidate)
  if (!stat.isDirectory() || stat.isSymbolicLink()) {
    throw new Error(
      `${purpose} must be an ordinary directory, not a file, symlink, or junction: ${candidate}`,
    )
  }
}

function assertNoReparseAncestors(candidate: string, root: string): void {
  const relative = path.relative(root, candidate)
  if (relative.startsWith('..') || path.isAbsolute(relative)) {
    throw new Error(
      `Desktop isolation path escaped its allowed root: ${candidate}`,
    )
  }
  let current = path.resolve(root)
  for (const segment of relative.split(path.sep).filter(Boolean)) {
    if (fs.existsSync(current)) {
      assertOrdinaryDirectory(current, 'Desktop isolation ancestor')
    }
    current = path.resolve(current, segment)
  }
  if (fs.existsSync(current)) {
    assertOrdinaryDirectory(current, 'Desktop isolation root')
  }
}

function readOwnedMarker(
  markerPath: string,
  runId: string,
  root: string,
): ProfileMarker {
  if (!fs.existsSync(markerPath)) {
    throw new Error(
      `Refusing to reuse an unmarked desktop isolation root. Expected ownership marker: ${markerPath}`,
    )
  }
  const stat = fs.lstatSync(markerPath)
  if (!stat.isFile() || stat.isSymbolicLink()) {
    throw new Error(
      `Desktop isolation ownership marker must be an ordinary file: ${markerPath}`,
    )
  }
  const marker = JSON.parse(
    fs.readFileSync(markerPath, 'utf8'),
  ) as Partial<ProfileMarker>
  const expectedAppDataDir = path.resolve(root, 'LocalAppData', APP_IDENTIFIER)
  if (
    marker.schemaVersion !== PROFILE_SCHEMA_VERSION ||
    marker.kind !== PROFILE_MARKER_KIND ||
    marker.runId !== runId ||
    marker.appIdentifier !== APP_IDENTIFIER ||
    !marker.createdAt ||
    !marker.appDataDir ||
    normalizeForComparison(marker.appDataDir) !==
      normalizeForComparison(expectedAppDataDir)
  ) {
    throw new Error(
      `Refusing to reuse desktop isolation root without an exact matching ownership marker: ${markerPath}`,
    )
  }
  return marker as ProfileMarker
}

function assertOwnedTreeHasNoReparsePoints(root: string): void {
  const pending = [root]
  while (pending.length > 0) {
    const current = pending.pop()
    if (!current) continue
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const candidate = path.resolve(current, entry.name)
      const stat = fs.lstatSync(candidate)
      if (stat.isSymbolicLink()) {
        throw new Error(
          `Refusing recursive desktop-profile teardown across a symlink or junction: ${candidate}`,
        )
      }
      if (stat.isDirectory()) {
        pending.push(candidate)
        continue
      }
      if (!stat.isFile()) {
        throw new Error(
          `Refusing recursive desktop-profile teardown across an unsupported entry: ${candidate}`,
        )
      }
    }
  }
}

function removeOwnedRoot(
  root: string,
  markerPath: string,
  runId: string,
): void {
  if (!fs.existsSync(root)) return
  assertOrdinaryDirectory(root, 'Desktop isolation root')
  readOwnedMarker(markerPath, runId, root)
  assertOwnedTreeHasNoReparsePoints(root)
  fs.rmSync(root, {
    recursive: true,
    force: true,
    maxRetries: 4,
    retryDelay: 250,
  })
}

function snapshotFile(filePath: string): { sizeBytes: number; sha256: string } {
  const bytes = fs.readFileSync(filePath)
  return {
    sizeBytes: bytes.byteLength,
    sha256: createHash('sha256').update(bytes).digest('hex'),
  }
}

function snapshotRoot(root: string): RootSnapshot {
  if (!fs.existsSync(root)) {
    return {
      root,
      state: 'missing',
      entries: [],
      fileCount: 0,
      totalBytes: 0,
    }
  }
  assertOrdinaryDirectory(root, 'Protected real-profile root')
  const entries: SnapshotEntry[] = []
  let fileCount = 0
  let totalBytes = 0

  const visit = (dir: string): void => {
    const children = fs
      .readdirSync(dir, { withFileTypes: true })
      .sort((left, right) => left.name.localeCompare(right.name))
    for (const child of children) {
      const fullPath = path.resolve(dir, child.name)
      const relativePath = path.relative(root, fullPath).replace(/\\/g, '/')
      const stat = fs.lstatSync(fullPath)
      if (stat.isSymbolicLink()) {
        throw new Error(
          `Protected real-profile snapshot refuses symlinks or junctions: ${fullPath}`,
        )
      }
      if (stat.isDirectory()) {
        entries.push({
          relativePath,
          kind: 'directory',
          sizeBytes: 0,
          sha256: '',
        })
        visit(fullPath)
        continue
      }
      if (!stat.isFile()) {
        throw new Error(
          `Protected real-profile snapshot found an unsupported entry: ${fullPath}`,
        )
      }
      fileCount += 1
      if (fileCount > MAX_SNAPSHOT_FILES) {
        throw new Error(
          `Protected real-profile snapshot exceeded ${String(MAX_SNAPSHOT_FILES)} files at ${root}.`,
        )
      }
      const digest = snapshotFile(fullPath)
      totalBytes += digest.sizeBytes
      if (totalBytes > MAX_SNAPSHOT_BYTES) {
        throw new Error(
          `Protected real-profile snapshot exceeded ${String(MAX_SNAPSHOT_BYTES)} bytes at ${root}.`,
        )
      }
      entries.push({
        relativePath,
        kind: 'file',
        sizeBytes: digest.sizeBytes,
        sha256: digest.sha256,
      })
    }
  }

  visit(root)
  return {
    root,
    state: 'present',
    entries,
    fileCount,
    totalBytes,
  }
}

function captureOriginalEnvironment(): EnvironmentSnapshot {
  return {
    LOCALAPPDATA: process.env.LOCALAPPDATA,
    APPDATA: process.env.APPDATA,
    TEMP: process.env.TEMP,
    TMP: process.env.TMP,
    USERPROFILE: process.env.USERPROFILE,
    HOME: process.env.HOME,
    VECTORGRID_DESKTOP_SMOKE_ISOLATED_PROFILE_ROOT:
      process.env.VECTORGRID_DESKTOP_SMOKE_ISOLATED_PROFILE_ROOT,
    VECTORGRID_DESKTOP_SMOKE_APP_DATA_OVERRIDE:
      process.env.VECTORGRID_DESKTOP_SMOKE_APP_DATA_OVERRIDE,
    VECTORGRID_DESKTOP_SMOKE_APP_DATA_DIR:
      process.env.VECTORGRID_DESKTOP_SMOKE_APP_DATA_DIR,
  }
}

function readWindowsKnownFolders(): WindowsKnownFolders | null {
  if (process.platform !== 'win32') return null
  const script = String.raw`
$source = @"
using System;
using System.Runtime.InteropServices;
public static class VectorGridKnownFolderProbe {
  [DllImport("shell32.dll")]
  private static extern int SHGetKnownFolderPath(
    [MarshalAs(UnmanagedType.LPStruct)] Guid rfid,
    uint flags,
    IntPtr token,
    out IntPtr path
  );
  public static string Get(string id) {
    IntPtr pointer;
    int result = SHGetKnownFolderPath(new Guid(id), 0, IntPtr.Zero, out pointer);
    if (result != 0) throw new InvalidOperationException("SHGetKnownFolderPath failed: " + result);
    try { return Marshal.PtrToStringUni(pointer); }
    finally { Marshal.FreeCoTaskMem(pointer); }
  }
}
"@
Add-Type -TypeDefinition $source
[ordered]@{
  localAppData = [VectorGridKnownFolderProbe]::Get("F1B32785-6FBA-4FCF-9D55-7B8E7F157091")
  roamingAppData = [VectorGridKnownFolderProbe]::Get("3EB685DB-65F9-4CF6-A03A-E3EF65729F3D")
  userProfile = [VectorGridKnownFolderProbe]::Get("5E6C858F-0E22-4760-9AFE-EA3317B67173")
} | ConvertTo-Json -Compress
`
  const encodedCommand = Buffer.from(script, 'utf16le').toString('base64')
  const result = spawnSync(
    'powershell.exe',
    ['-NoProfile', '-NonInteractive', '-EncodedCommand', encodedCommand],
    {
      encoding: 'utf8',
      env: { ...process.env },
      windowsHide: true,
      timeout: 10_000,
    },
  )
  if (result.status !== 0) {
    throw new Error(
      `Unable to resolve Windows known folders safely: ${String(result.stderr || result.stdout).trim()}`,
    )
  }
  let value: Partial<WindowsKnownFolders>
  try {
    value = JSON.parse(
      String(result.stdout || '').trim(),
    ) as Partial<WindowsKnownFolders>
  } catch (error) {
    throw new Error(
      `Windows known-folder probe returned malformed JSON: ${String(error)}`,
    )
  }
  const folders: WindowsKnownFolders = {
    localAppData: String(value.localAppData || '').trim(),
    roamingAppData: String(value.roamingAppData || '').trim(),
    userProfile: String(value.userProfile || '').trim(),
  }
  if (
    !folders.localAppData ||
    !folders.roamingAppData ||
    !folders.userProfile
  ) {
    throw new Error(
      'Windows known-folder probe returned an incomplete profile.',
    )
  }
  return folders
}

function requireSafeWindowsKnownFolderIsolation(
  root: string,
  folders: WindowsKnownFolders | null,
): void {
  if (!folders) return
  const capturedByCleanup = Object.entries(folders)
    .filter(([, candidate]) => isPathInside(candidate, root))
    .map(([name, candidate]) => `${name}=${candidate}`)
  if (capturedByCleanup.length === 0) return
  throw new Error(
    [
      'Refusing to launch the exact package because active Windows known folders are inside the sentinel-owned cleanup root.',
      `Captured folders: ${capturedByCleanup.join(', ')}.`,
      `Recursive teardown of ${root} could delete the active Windows profile.`,
      'Use a sentinel root outside every active Windows known folder; product app-data isolation is handled by the explicit QA handshake.',
    ].join(' '),
  )
}

function protectedRealProfileRoots(
  original: EnvironmentSnapshot,
  isolationRoot: string,
  folders: WindowsKnownFolders | null,
): string[] {
  const candidates = [
    folders?.localAppData
      ? path.resolve(folders.localAppData, APP_IDENTIFIER)
      : original.LOCALAPPDATA
        ? path.resolve(original.LOCALAPPDATA, APP_IDENTIFIER)
        : '',
    folders?.roamingAppData
      ? path.resolve(folders.roamingAppData, APP_IDENTIFIER)
      : original.APPDATA
        ? path.resolve(original.APPDATA, APP_IDENTIFIER)
        : '',
    original.TEMP ? path.resolve(original.TEMP, 'vectorgrid') : '',
  ].filter(Boolean)
  const unique = new Map<string, string>()
  for (const candidate of candidates) {
    if (isPathInside(candidate, isolationRoot)) continue
    unique.set(normalizeForComparison(candidate), candidate)
  }
  return [...unique.values()].sort((left, right) => left.localeCompare(right))
}

function applyIsolatedEnvironment(profile: DesktopProfileIsolation): void {
  process.env.LOCALAPPDATA = profile.localAppData
  process.env.APPDATA = profile.roamingAppData
  process.env.TEMP = profile.temp
  process.env.TMP = profile.temp
  process.env.USERPROFILE = profile.userProfile
  process.env.HOME = profile.userProfile
  process.env.VECTORGRID_DESKTOP_SMOKE_ISOLATED_PROFILE_ROOT = profile.root
  process.env[PROFILE_OVERRIDE_ENV] = '1'
  process.env[PROFILE_APP_DATA_ENV] = profile.appDataDir
}

export function activateDesktopProfileEnvironment(
  profile: DesktopProfileIsolation | null,
): void {
  if (!profile) return
  assertOrdinaryDirectory(profile.root, 'Desktop isolation root')
  readOwnedMarker(profile.markerPath, profile.runId, profile.root)
  assertOrdinaryDirectory(
    profile.appDataDir,
    'Desktop isolated app-data directory',
  )
  applyIsolatedEnvironment(profile)
}

export function restoreDesktopProfileEnvironment(
  profile: DesktopProfileIsolation,
): void {
  restoreEnvironmentSnapshot(profile.originalEnvironment)
}

function restoreEnvironmentSnapshot(snapshot: EnvironmentSnapshot): void {
  for (const [name, value] of Object.entries(snapshot)) {
    if (value == null) delete process.env[name]
    else process.env[name] = value
  }
}

export function prepareDesktopProfileIsolation(
  artifactContext: DesktopArtifactContext,
): DesktopProfileIsolation | null {
  const requestedRoot = String(
    process.env.VECTORGRID_DESKTOP_SMOKE_ISOLATED_PROFILE_ROOT || '',
  ).trim()
  if (!requestedRoot) return null
  if (!path.isAbsolute(requestedRoot)) {
    throw new Error(
      'VECTORGRID_DESKTOP_SMOKE_ISOLATED_PROFILE_ROOT must be absolute.',
    )
  }

  const root = path.resolve(requestedRoot)
  const allowedRoot = path.resolve(repoRoot, 'output')
  if (
    !isPathInside(root, allowedRoot) ||
    normalizeForComparison(root) === normalizeForComparison(allowedRoot)
  ) {
    throw new Error(
      `Desktop isolation root must be a child of the repository output directory: ${root}`,
    )
  }
  const windowsKnownFolders = readWindowsKnownFolders()
  requireSafeWindowsKnownFolderIsolation(root, windowsKnownFolders)
  assertNoReparseAncestors(root, allowedRoot)
  const runId = artifactContext.runId
  const markerPath = path.resolve(root, PROFILE_MARKER_NAME)
  const localAppData = path.resolve(root, 'LocalAppData')
  const roamingAppData = path.resolve(root, 'RoamingAppData')
  const temp = path.resolve(root, 'Temp')
  const userProfile = path.resolve(root, 'UserProfile')
  const appDataDir = path.resolve(localAppData, APP_IDENTIFIER)
  const attestationPath = path.resolve(appDataDir, PROFILE_ATTESTATION_NAME)
  if (fs.existsSync(root)) {
    removeOwnedRoot(root, markerPath, runId)
  }

  ensureDir(root)
  const marker: ProfileMarker = {
    schemaVersion: PROFILE_SCHEMA_VERSION,
    kind: PROFILE_MARKER_KIND,
    runId,
    appIdentifier: APP_IDENTIFIER,
    appDataDir,
    createdAt: new Date().toISOString(),
  }
  fs.writeFileSync(markerPath, `${JSON.stringify(marker, null, 2)}\n`, {
    encoding: 'utf8',
    flag: 'wx',
  })

  const originalEnvironment = captureOriginalEnvironment()
  try {
    const profile: DesktopProfileIsolation = {
      root,
      localAppData: ensureDir(localAppData),
      roamingAppData: ensureDir(roamingAppData),
      temp: ensureDir(temp),
      userProfile: ensureDir(userProfile),
      appDataDir: ensureDir(appDataDir),
      markerPath,
      attestationPath,
      markerCreatedAt: marker.createdAt,
      evidencePath: path.resolve(
        artifactContext.logsDir,
        'profile-isolation.json',
      ),
      runId,
      originalEnvironment,
      windowsKnownFolders,
      protectedRoots: protectedRealProfileRoots(
        originalEnvironment,
        root,
        windowsKnownFolders,
      ),
      before: [],
    }
    profile.before = profile.protectedRoots.map(snapshotRoot)
    return profile
  } catch (error) {
    removeOwnedRoot(root, markerPath, runId)
    restoreEnvironmentSnapshot(originalEnvironment)
    throw error
  }
}

export function desktopDriverEnvironment(
  profile: DesktopProfileIsolation | null,
): NodeJS.ProcessEnv {
  if (!profile) return { ...process.env }
  const environment: NodeJS.ProcessEnv = {
    ...process.env,
    LOCALAPPDATA: profile.localAppData,
    APPDATA: profile.roamingAppData,
    TEMP: profile.temp,
    TMP: profile.temp,
    USERPROFILE: profile.userProfile,
    HOME: profile.userProfile,
    VECTORGRID_DESKTOP_SMOKE_ISOLATED_PROFILE_ROOT: profile.root,
    VECTORGRID_DESKTOP_SMOKE_APP_DATA_OVERRIDE: '1',
    VECTORGRID_DESKTOP_SMOKE_APP_DATA_DIR: profile.appDataDir,
  }
  // A clean-profile proof must not silently inherit developer/provider credentials.
  // Windows Credential Manager entries are read-only and asserted empty by the spec.
  delete environment.POLYGON_KEY
  delete environment.POLYGON_API_KEY
  delete environment.ALPACA_API_KEY
  delete environment.ALPACA_API_SECRET
  return environment
}

export function cleanupDesktopProfileIsolation(
  profile: DesktopProfileIsolation | null,
): void {
  if (!profile) return
  try {
    removeOwnedRoot(profile.root, profile.markerPath, profile.runId)
  } finally {
    restoreDesktopProfileEnvironment(profile)
  }
}

function readProfileAttestation(
  profile: DesktopProfileIsolation,
): ProfileAttestation {
  if (!fs.existsSync(profile.attestationPath)) {
    throw new Error(
      `The exact package did not create its QA profile attestation: ${profile.attestationPath}`,
    )
  }
  const stat = fs.lstatSync(profile.attestationPath)
  if (!stat.isFile() || stat.isSymbolicLink()) {
    throw new Error(
      `QA profile attestation must be an ordinary file: ${profile.attestationPath}`,
    )
  }
  let value: Partial<ProfileAttestation>
  try {
    value = JSON.parse(
      fs.readFileSync(profile.attestationPath, 'utf8'),
    ) as Partial<ProfileAttestation>
  } catch (error) {
    throw new Error(`QA profile attestation is malformed: ${String(error)}`)
  }
  if (
    value.schemaVersion !== PROFILE_SCHEMA_VERSION ||
    value.kind !== PROFILE_ATTESTATION_KIND ||
    value.runId !== profile.runId ||
    value.markerCreatedAt !== profile.markerCreatedAt ||
    value.originalIdentifier !== APP_IDENTIFIER ||
    !value.resolvedPaths
  ) {
    throw new Error(
      'QA profile attestation identity does not match the active harness run.',
    )
  }

  const attestation = value as ProfileAttestation
  const expectedAppData = normalizeForComparison(profile.appDataDir)
  const expectedLogDir = normalizeForComparison(
    path.resolve(profile.appDataDir, 'logs'),
  )
  const expectedWebviewDir = normalizeForComparison(
    path.resolve(profile.appDataDir, 'webview2'),
  )
  const appDataPaths = [
    attestation.effectiveIdentifier,
    attestation.appDataDir,
    attestation.resolvedPaths.appConfigDir,
    attestation.resolvedPaths.appDataDir,
    attestation.resolvedPaths.appLocalDataDir,
    attestation.resolvedPaths.appCacheDir,
  ]
  if (
    normalizeForComparison(attestation.isolationRoot) !==
      normalizeForComparison(profile.root) ||
    appDataPaths.some(
      (candidate) => normalizeForComparison(candidate) !== expectedAppData,
    ) ||
    normalizeForComparison(attestation.resolvedPaths.appLogDir) !==
      expectedLogDir ||
    normalizeForComparison(attestation.resolvedPaths.webviewDataDir) !==
      expectedWebviewDir
  ) {
    throw new Error(
      'QA profile attestation reports a path outside the approved isolation contract.',
    )
  }

  const attestedPaths = [
    attestation.appDataDir,
    ...Object.values(attestation.resolvedPaths),
  ]
  for (const candidate of attestedPaths) {
    if (!path.isAbsolute(candidate) || !isPathInside(candidate, profile.root)) {
      throw new Error(
        `QA profile attestation escaped the sentinel-owned root: ${candidate}`,
      )
    }
  }
  return attestation
}

export function writeDesktopProfileIsolationBlockedEvidence(
  artifactContext: DesktopArtifactContext,
  error: unknown,
): DesktopProfileIsolationSummary {
  const reason = String(error instanceof Error ? error.message : error)
  const requestedRoot = String(
    process.env.VECTORGRID_DESKTOP_SMOKE_ISOLATED_PROFILE_ROOT || '',
  ).trim()
  const evidencePath = path.resolve(
    artifactContext.logsDir,
    'profile-isolation.json',
  )
  writeTextFile(
    evidencePath,
    `${JSON.stringify(
      {
        schemaVersion: PROFILE_SCHEMA_VERSION,
        generatedAt: new Date().toISOString(),
        runId: artifactContext.runId,
        status: 'blocked',
        runtimeLaunched: false,
        requestedIsolationRoot: requestedRoot,
        realProfileMutationPrevented: true,
        reason,
        requiredEnvironment:
          'The exact package must accept the schema-2 marker/root/app-data/run-id QA handshake and write a matching setup attestation, or run on a separately provisioned disposable Windows runner with a non-destructive lifecycle adapter.',
        unsafeSubstituteRejected:
          'Environment-only known-folder spoofing, incomplete override handshakes, missing app attestations, and active Windows known folders beneath a recursively deleted sentinel root are rejected.',
      },
      null,
      2,
    )}\n`,
  )
  return {
    enabled: false,
    root: requestedRoot,
    realProfileUnchanged: true,
    evidencePath,
    status: 'blocked',
    reason,
  }
}

export function finalizeDesktopProfileIsolation(
  profile: DesktopProfileIsolation | null,
): DesktopProfileIsolationSummary {
  if (!profile) {
    return {
      enabled: false,
      root: '',
      realProfileUnchanged: true,
      evidencePath: '',
      status: 'not_enabled',
    }
  }

  let after: RootSnapshot[] = []
  let comparisonError = ''
  let realProfileUnchanged = false
  let appAttestation: ProfileAttestation | null = null
  let attestationError = ''
  try {
    appAttestation = readProfileAttestation(profile)
  } catch (error) {
    attestationError = String(error instanceof Error ? error.message : error)
  }
  try {
    after = profile.protectedRoots.map(snapshotRoot)
    realProfileUnchanged =
      JSON.stringify(profile.before) === JSON.stringify(after)
  } catch (error) {
    comparisonError = String(error instanceof Error ? error.message : error)
  }

  const evidence = {
    schemaVersion: PROFILE_SCHEMA_VERSION,
    status: 'pending',
    generatedAt: new Date().toISOString(),
    runId: profile.runId,
    isolationRoot: profile.root,
    markerPath: profile.markerPath,
    markerCreatedAt: profile.markerCreatedAt,
    isolatedEnvironment: {
      localAppData: profile.localAppData,
      roamingAppData: profile.roamingAppData,
      temp: profile.temp,
      userProfile: profile.userProfile,
      appDataDir: profile.appDataDir,
    },
    windowsKnownFolders: profile.windowsKnownFolders,
    protectedRoots: profile.protectedRoots,
    realProfileUnchanged,
    comparisonError,
    appAttestation,
    attestationError,
    effectiveIsolationProven: !attestationError && appAttestation != null,
    before: profile.before,
    after,
    teardown: 'pending',
  }

  let teardownError = ''
  try {
    removeOwnedRoot(profile.root, profile.markerPath, profile.runId)
    evidence.teardown = 'completed'
  } catch (error) {
    teardownError = String(error instanceof Error ? error.message : error)
    evidence.teardown = 'failed'
  } finally {
    restoreDesktopProfileEnvironment(profile)
  }
  evidence.status =
    !comparisonError &&
    realProfileUnchanged &&
    !attestationError &&
    appAttestation != null &&
    !teardownError
      ? 'passed'
      : 'failed'
  writeTextFile(
    profile.evidencePath,
    `${JSON.stringify({ ...evidence, teardownError }, null, 2)}\n`,
  )

  if (comparisonError) {
    throw new Error(`Real-profile comparison failed: ${comparisonError}`)
  }
  if (!realProfileUnchanged) {
    throw new Error(
      `The isolated desktop run changed the real VectorGrid profile. See ${profile.evidencePath}`,
    )
  }
  if (teardownError) {
    throw new Error(
      `Isolated desktop profile teardown failed: ${teardownError}`,
    )
  }
  if (attestationError) {
    throw new Error(
      `The exact package did not prove QA app-data isolation: ${attestationError}. See ${profile.evidencePath}`,
    )
  }
  return {
    enabled: true,
    root: profile.root,
    realProfileUnchanged,
    evidencePath: profile.evidencePath,
    status: 'passed',
  }
}
