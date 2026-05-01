#!/usr/bin/env node
import crypto from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { spawnSync } from 'node:child_process'

const EXPECTED_WORKSPACE_MEMBERS = [
  'src-tauri',
  'crates/app_core',
  'crates/strategies',
  'crates/optimizer',
]
const EXPECTED_VECTOR_TA_VERSION = '0.2.4'
const EXPECTED_VECTOR_TA_PATCH = 'vendor/vector-ta-0.2.4'
const EXPECTED_PACKAGE_MANAGER = 'npm@11.6.4'
const EXPECTED_NODE_ENGINE = '^20.19.0 || >=22.12.0'
const EXPECTED_CANONICAL_DIST = '../web/dist'
const EXPECTED_MIRROR_DIST = './web/dist'
const DOCS_CONTRACT_DIR = 'docs/feature_expansion_implementation_plans_2026-04-30'
const DOCS_CONTRACT_FILE = `${DOCS_CONTRACT_DIR}/13_manifest_toolchain_dependency_contract.md`

const CPU_ONLY_FORBIDDEN_DEFAULT_FEATURES = new Set([
  'cuda',
  'cuda-backtest-kernel',
  'cust',
  'dep:cust',
  'vector-ta/cuda',
  'ta_app_core/cuda',
  'ta_app_core/cuda-backtest-kernel',
  'nvtx-profiling',
])

function argValue(name, fallback = '') {
  const idx = process.argv.findIndex((arg) => arg === name)
  if (idx < 0) return fallback
  return process.argv[idx + 1] ?? fallback
}

function resolveRepoRoot() {
  const cwd = process.cwd()
  if (fs.existsSync(path.join(cwd, 'Cargo.toml')) && fs.existsSync(path.join(cwd, 'src-tauri'))) {
    return cwd
  }
  const parent = path.resolve(cwd, '..')
  if (fs.existsSync(path.join(parent, 'Cargo.toml')) && fs.existsSync(path.join(parent, 'src-tauri'))) {
    return parent
  }
  throw new Error(`could not resolve repo root from ${cwd}`)
}

const repoRoot = resolveRepoRoot()
const outPath = path.resolve(
  repoRoot,
  argValue('--out', path.join('output', 'qa', 'manifest-toolchain-contract.json')),
)

function toPosix(value) {
  return String(value).replace(/\\/g, '/')
}

function rel(filePath) {
  return toPosix(path.relative(repoRoot, filePath))
}

function readText(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), 'utf8')
}

function readJson(relativePath) {
  return JSON.parse(readText(relativePath))
}

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true })
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8')
}

function sha256File(filePath) {
  const hash = crypto.createHash('sha256')
  hash.update(fs.readFileSync(filePath))
  return hash.digest('hex')
}

function stripTomlComment(line) {
  let inString = false
  let quote = ''
  let escaped = false
  for (let idx = 0; idx < line.length; idx += 1) {
    const ch = line[idx]
    if (inString) {
      if (escaped) {
        escaped = false
      } else if (ch === '\\') {
        escaped = true
      } else if (ch === quote) {
        inString = false
        quote = ''
      }
      continue
    }
    if (ch === '"' || ch === "'") {
      inString = true
      quote = ch
      continue
    }
    if (ch === '#') return line.slice(0, idx)
  }
  return line
}

function splitTopLevel(value, delimiter) {
  const parts = []
  let current = ''
  let inString = false
  let quote = ''
  let escaped = false
  let squareDepth = 0
  let braceDepth = 0
  for (let idx = 0; idx < value.length; idx += 1) {
    const ch = value[idx]
    if (inString) {
      current += ch
      if (escaped) {
        escaped = false
      } else if (ch === '\\') {
        escaped = true
      } else if (ch === quote) {
        inString = false
        quote = ''
      }
      continue
    }
    if (ch === '"' || ch === "'") {
      inString = true
      quote = ch
      current += ch
      continue
    }
    if (ch === '[') squareDepth += 1
    if (ch === ']') squareDepth -= 1
    if (ch === '{') braceDepth += 1
    if (ch === '}') braceDepth -= 1
    if (ch === delimiter && squareDepth === 0 && braceDepth === 0) {
      parts.push(current.trim())
      current = ''
      continue
    }
    current += ch
  }
  if (current.trim().length > 0) parts.push(current.trim())
  return parts
}

function splitTopLevelEquals(value) {
  let inString = false
  let quote = ''
  let escaped = false
  let squareDepth = 0
  let braceDepth = 0
  for (let idx = 0; idx < value.length; idx += 1) {
    const ch = value[idx]
    if (inString) {
      if (escaped) {
        escaped = false
      } else if (ch === '\\') {
        escaped = true
      } else if (ch === quote) {
        inString = false
        quote = ''
      }
      continue
    }
    if (ch === '"' || ch === "'") {
      inString = true
      quote = ch
      continue
    }
    if (ch === '[') squareDepth += 1
    if (ch === ']') squareDepth -= 1
    if (ch === '{') braceDepth += 1
    if (ch === '}') braceDepth -= 1
    if (ch === '=' && squareDepth === 0 && braceDepth === 0) {
      return [value.slice(0, idx).trim(), value.slice(idx + 1).trim()]
    }
  }
  return null
}

function tomlBalance(value) {
  let inString = false
  let quote = ''
  let escaped = false
  let squareDepth = 0
  let braceDepth = 0
  for (const ch of value) {
    if (inString) {
      if (escaped) {
        escaped = false
      } else if (ch === '\\') {
        escaped = true
      } else if (ch === quote) {
        inString = false
        quote = ''
      }
      continue
    }
    if (ch === '"' || ch === "'") {
      inString = true
      quote = ch
      continue
    }
    if (ch === '[') squareDepth += 1
    if (ch === ']') squareDepth -= 1
    if (ch === '{') braceDepth += 1
    if (ch === '}') braceDepth -= 1
  }
  return squareDepth + braceDepth
}

function parseTomlString(value) {
  const trimmed = value.trim()
  if (trimmed.startsWith('"')) return JSON.parse(trimmed)
  if (trimmed.startsWith("'")) return trimmed.slice(1, -1)
  return trimmed
}

function parseTomlValue(value) {
  const trimmed = value.trim()
  if (trimmed.startsWith('[') && trimmed.endsWith(']')) {
    const inner = trimmed.slice(1, -1)
    if (!inner.trim()) return []
    return splitTopLevel(inner, ',').map(parseTomlValue)
  }
  if (trimmed.startsWith('{') && trimmed.endsWith('}')) {
    const inner = trimmed.slice(1, -1)
    const output = {}
    for (const entry of splitTopLevel(inner, ',')) {
      const pair = splitTopLevelEquals(entry)
      if (!pair) continue
      output[pair[0]] = parseTomlValue(pair[1])
    }
    return output
  }
  if (trimmed.startsWith('"') || trimmed.startsWith("'")) return parseTomlString(trimmed)
  if (trimmed === 'true') return true
  if (trimmed === 'false') return false
  if (/^-?\d+(\.\d+)?$/.test(trimmed)) return Number(trimmed)
  return trimmed
}

function parseToml(relativePath) {
  const text = readText(relativePath)
  const sections = { '': {} }
  let section = ''
  let pending = null

  for (const rawLine of text.split(/\r?\n/)) {
    const line = stripTomlComment(rawLine).trim()
    if (!line) continue

    if (pending) {
      pending.value = `${pending.value} ${line}`
      if (tomlBalance(pending.value) <= 0) {
        sections[pending.section][pending.key] = parseTomlValue(pending.value)
        pending = null
      }
      continue
    }

    const sectionMatch = line.match(/^\[([^\]]+)\]$/)
    if (sectionMatch) {
      section = sectionMatch[1]
      sections[section] ??= {}
      continue
    }

    const pair = splitTopLevelEquals(line)
    if (!pair) continue
    const [key, value] = pair
    sections[section] ??= {}
    if (tomlBalance(value) > 0) {
      pending = { section, key, value }
      continue
    }
    sections[section][key] = parseTomlValue(value)
  }

  if (pending) {
    throw new Error(`unterminated TOML value in ${relativePath}: ${pending.key}`)
  }
  return { relativePath, sections, text }
}

function asFeatureList(value) {
  return Array.isArray(value) ? value.map(String) : []
}

function dependencyVersion(value) {
  if (typeof value === 'string') return value
  if (value && typeof value === 'object' && typeof value.version === 'string') return value.version
  return null
}

function dependencyOptional(value) {
  return Boolean(value && typeof value === 'object' && value.optional === true)
}

function stableValue(value) {
  if (Array.isArray(value)) return value.map(stableValue)
  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .map((key) => [key, stableValue(value[key])]),
    )
  }
  return value ?? null
}

function stableWindowShape(windowConfig) {
  return stableValue(windowConfig ?? {})
}

function comparableTauriShape(config) {
  return {
    productName: config.productName ?? null,
    version: config.version ?? null,
    identifier: config.identifier ?? null,
    build: {
      beforeDevCommand: config.build?.beforeDevCommand ?? null,
      beforeBuildCommand: config.build?.beforeBuildCommand ?? null,
      devUrl: config.build?.devUrl ?? null,
    },
    app: {
      withGlobalTauri: config.app?.withGlobalTauri ?? null,
      macOSPrivateApi: config.app?.macOSPrivateApi ?? null,
      security: {
        capabilities: config.app?.security?.capabilities ?? [],
        csp: config.app?.security?.csp ?? null,
        devCsp: config.app?.security?.devCsp ?? null,
      },
      windows: Array.isArray(config.app?.windows) ? config.app.windows.map(stableWindowShape) : [],
    },
    plugins: {
      updater: {
        active: config.plugins?.updater?.active ?? null,
        endpoints: config.plugins?.updater?.endpoints ?? [],
        pubkey: config.plugins?.updater?.pubkey ?? null,
      },
    },
  }
}

function normalizeBundleTargets(targets) {
  if (!targets) return []
  if (Array.isArray(targets)) return targets.map(String).sort()
  if (typeof targets === 'string') return [targets]
  return []
}

function normalizeBundleResources(configRelativePath, resources) {
  if (!resources) return []
  const configDir = path.dirname(path.join(repoRoot, configRelativePath))
  if (Array.isArray(resources)) {
    return resources.map((source) => ({
      source: path.resolve(configDir, source),
      source_relative: rel(path.resolve(configDir, source)),
      target: null,
    }))
  }
  if (typeof resources === 'object') {
    return Object.entries(resources)
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([source, target]) => {
        const absoluteSource = path.resolve(configDir, source)
        return {
          source: absoluteSource,
          source_relative: rel(absoluteSource),
          target: target ?? null,
        }
      })
  }
  return []
}

function normalizeWindowsBundle(configRelativePath, windowsConfig) {
  if (!windowsConfig || typeof windowsConfig !== 'object') return null
  const configDir = path.dirname(path.join(repoRoot, configRelativePath))
  const nsis = windowsConfig.nsis && typeof windowsConfig.nsis === 'object' ? windowsConfig.nsis : {}
  return stableValue({
    webviewInstallMode: windowsConfig.webviewInstallMode ?? null,
    nsis: {
      displayLanguageSelector: nsis.displayLanguageSelector ?? null,
      installerIcon:
        typeof nsis.installerIcon === 'string'
          ? path.resolve(configDir, nsis.installerIcon)
          : null,
    },
  })
}

function windowsConfigShape(configRelativePath, config) {
  return {
    schema: config.$schema ?? null,
    bundle: {
      targets: normalizeBundleTargets(config.bundle?.targets ?? null),
      resources: normalizeBundleResources(configRelativePath, config.bundle?.resources ?? null),
      windows: normalizeWindowsBundle(configRelativePath, config.bundle?.windows ?? null),
    },
  }
}

function hasResourceMapping(resources, expectedSourceRelative, expectedTarget) {
  return resources.some(
    (entry) =>
      entry.source_relative === expectedSourceRelative &&
      toPosix(entry.target ?? '') === expectedTarget,
  )
}

function runGitCheckIgnored(relativePath) {
  const result = spawnSync('git', ['check-ignore', '-q', relativePath], {
    cwd: repoRoot,
    encoding: 'utf8',
  })
  if (result.error) {
    return { checked: false, ignored: null, error: result.error.message }
  }
  if (result.status === 0) return { checked: true, ignored: true, error: null }
  if (result.status === 1) return { checked: true, ignored: false, error: null }
  return {
    checked: false,
    ignored: null,
    error: String(result.stderr || `git check-ignore exited with status ${result.status}`).trim(),
  }
}

const checks = []
function record(id, ok, message, details = {}) {
  checks.push({ id, status: ok ? 'passed' : 'failed', message, details })
}

function recordWarning(id, message, details = {}) {
  checks.push({ id, status: 'warning', message, details })
}

function arrayEquals(left, right) {
  return (
    Array.isArray(left) &&
    left.length === right.length &&
    left.every((value, index) => value === right[index])
  )
}

const manifestPaths = {
  workspace: 'Cargo.toml',
  tauri: 'src-tauri/Cargo.toml',
  appCore: 'crates/app_core/Cargo.toml',
  strategies: 'crates/strategies/Cargo.toml',
  optimizer: 'crates/optimizer/Cargo.toml',
  vectorTaVendor: `${EXPECTED_VECTOR_TA_PATCH}/Cargo.toml`,
}

const cargo = Object.fromEntries(
  Object.entries(manifestPaths)
    .filter(([, relativePath]) => fs.existsSync(path.join(repoRoot, relativePath)))
    .map(([key, relativePath]) => [key, parseToml(relativePath)]),
)

for (const [key, relativePath] of Object.entries(manifestPaths)) {
  record(
    `manifest.${key}.exists`,
    Boolean(cargo[key]),
    `${relativePath} exists`,
    { path: relativePath },
  )
}

const workspace = cargo.workspace?.sections.workspace ?? {}
record(
  'cargo.workspace.members',
  arrayEquals(workspace.members, EXPECTED_WORKSPACE_MEMBERS),
  'Cargo workspace members match the contract',
  { expected: EXPECTED_WORKSPACE_MEMBERS, actual: workspace.members ?? null },
)
record('cargo.workspace.resolver', workspace.resolver === '2', 'Cargo workspace uses resolver 2', {
  expected: '2',
  actual: workspace.resolver ?? null,
})

const vectorPatch = cargo.workspace?.sections['patch.crates-io']?.['vector-ta'] ?? null
const vectorPatchPath = vectorPatch && typeof vectorPatch === 'object' ? vectorPatch.path : null
record(
  'vector-ta.patch.path',
  vectorPatchPath === EXPECTED_VECTOR_TA_PATCH,
  'Root Cargo.toml patches vector-ta to the expected vendored path',
  { expected: EXPECTED_VECTOR_TA_PATCH, actual: vectorPatchPath },
)
record(
  'vector-ta.patch.exists',
  typeof vectorPatchPath === 'string' && fs.existsSync(path.join(repoRoot, vectorPatchPath, 'Cargo.toml')),
  'Vendored vector-ta patch Cargo.toml exists',
  { path: vectorPatchPath },
)

const vectorDependencyChecks = []
for (const [key, manifest] of Object.entries(cargo)) {
  if (key === 'workspace' || key === 'vectorTaVendor') continue
  const dependencies = manifest.sections.dependencies ?? {}
  const vectorDep = dependencies['vector-ta'] ?? null
  const wrongImportDep = dependencies.vector_ta ?? null
  vectorDependencyChecks.push({
    manifest: manifest.relativePath,
    version: dependencyVersion(vectorDep),
    has_hyphenated_dependency: vectorDep != null,
    has_underscore_dependency: wrongImportDep != null,
  })
}
record(
  'vector-ta.dependencies.present',
  vectorDependencyChecks.every((entry) => entry.has_hyphenated_dependency),
  'Workspace crates use the hyphenated vector-ta dependency name',
  { dependencies: vectorDependencyChecks },
)
record(
  'vector-ta.dependencies.no-underscore-name',
  vectorDependencyChecks.every((entry) => !entry.has_underscore_dependency),
  'Cargo manifests do not declare vector_ta as a dependency name',
  { dependencies: vectorDependencyChecks },
)
record(
  'vector-ta.dependencies.version',
  vectorDependencyChecks.every((entry) => entry.version === EXPECTED_VECTOR_TA_VERSION),
  'Workspace vector-ta dependency versions are consistent',
  { expected: EXPECTED_VECTOR_TA_VERSION, dependencies: vectorDependencyChecks },
)

const vendorPackage = cargo.vectorTaVendor?.sections.package ?? {}
record(
  'vector-ta.vendor.version',
  vendorPackage.name === 'vector-ta' && vendorPackage.version === EXPECTED_VECTOR_TA_VERSION,
  'Vendored vector-ta package identity matches dependency versions',
  {
    expected: { name: 'vector-ta', version: EXPECTED_VECTOR_TA_VERSION },
    actual: { name: vendorPackage.name ?? null, version: vendorPackage.version ?? null },
  },
)

const vendorDefaultFeatures = asFeatureList(cargo.vectorTaVendor?.sections.features?.default)
record(
  'vector-ta.vendor.default-features',
  vendorDefaultFeatures.length === 0,
  'Vendored vector-ta default features are CPU-only',
  { actual: vendorDefaultFeatures },
)

const appCoreFeatures = cargo.appCore?.sections.features ?? {}
const appCoreDefault = asFeatureList(appCoreFeatures.default)
const appCoreCuda = asFeatureList(appCoreFeatures.cuda)
const appCoreCudaKernel = asFeatureList(appCoreFeatures['cuda-backtest-kernel'])
const appCoreCust = cargo.appCore?.sections.dependencies?.cust ?? null

record('features.app-core.default-empty', appCoreDefault.length === 0, 'app_core default features are empty', {
  actual: appCoreDefault,
})
record(
  'features.app-core.cuda-forwarding',
  appCoreCuda.includes('vector-ta/cuda') && appCoreCuda.includes('dep:cust'),
  'app_core cuda feature forwards to vector-ta/cuda and optional cust',
  { actual: appCoreCuda },
)
record(
  'features.app-core.cuda-kernel-implies-cuda',
  appCoreCudaKernel.includes('cuda'),
  'app_core cuda-backtest-kernel implies cuda',
  { actual: appCoreCudaKernel },
)
record(
  'features.app-core.cust-optional',
  dependencyOptional(appCoreCust),
  'app_core cust dependency is optional',
  { actual: appCoreCust },
)

const tauriFeatures = cargo.tauri?.sections.features ?? {}
const tauriDefault = asFeatureList(tauriFeatures.default)
const tauriCuda = asFeatureList(tauriFeatures.cuda)
const tauriCudaKernel = asFeatureList(tauriFeatures['cuda-backtest-kernel'])
const tauriNvtx = asFeatureList(tauriFeatures['nvtx-profiling'])
const tauriCust = cargo.tauri?.sections.dependencies?.cust ?? null

record(
  'features.tauri.default-custom-protocol-only',
  arrayEquals(tauriDefault, ['custom-protocol']),
  'src-tauri default features only enable custom-protocol',
  { actual: tauriDefault },
)
record(
  'features.tauri.cuda-forwarding',
  tauriCuda.includes('ta_app_core/cuda') && tauriCuda.includes('vector-ta/cuda') && tauriCuda.includes('cust'),
  'src-tauri cuda feature forwards to app_core, vector-ta, and optional cust',
  { actual: tauriCuda },
)
record(
  'features.tauri.cuda-kernel-forwarding',
  tauriCudaKernel.includes('cuda') && tauriCudaKernel.includes('ta_app_core/cuda-backtest-kernel'),
  'src-tauri cuda-backtest-kernel implies cuda and forwards to app_core',
  { actual: tauriCudaKernel },
)
record(
  'features.tauri.nvtx-forwarding',
  tauriNvtx.includes('ta_app_core/nvtx-profiling'),
  'src-tauri nvtx-profiling forwards to app_core',
  { actual: tauriNvtx },
)
record(
  'features.tauri.cust-optional',
  dependencyOptional(tauriCust),
  'src-tauri cust dependency is optional',
  { actual: tauriCust },
)

const cpuDefaultSurfaces = [
  { manifest: 'src-tauri/Cargo.toml', default_features: tauriDefault },
  { manifest: 'crates/app_core/Cargo.toml', default_features: appCoreDefault },
]
const cpuDefaultViolations = cpuDefaultSurfaces.flatMap((entry) =>
  entry.default_features
    .filter((feature) => CPU_ONLY_FORBIDDEN_DEFAULT_FEATURES.has(feature))
    .map((feature) => ({ manifest: entry.manifest, feature })),
)
record(
  'features.cpu-only.default-posture',
  cpuDefaultViolations.length === 0,
  'Default features do not opt into CUDA, NVTX, or cust',
  { forbidden: [...CPU_ONLY_FORBIDDEN_DEFAULT_FEATURES].sort(), violations: cpuDefaultViolations },
)

const webPackage = readJson('web/package.json')
const webPackageLock = readJson('web/package-lock.json')
record(
  'node.package-manager',
  webPackage.packageManager === EXPECTED_PACKAGE_MANAGER,
  'web/package.json pins the expected npm packageManager',
  { expected: EXPECTED_PACKAGE_MANAGER, actual: webPackage.packageManager ?? null },
)
record(
  'node.engine',
  webPackage.engines?.node === EXPECTED_NODE_ENGINE,
  'web/package.json declares the expected Node engine range',
  { expected: EXPECTED_NODE_ENGINE, actual: webPackage.engines?.node ?? null },
)
record(
  'node.package-lock.identity',
  webPackageLock.name === webPackage.name &&
    webPackageLock.version === webPackage.version &&
    Number(webPackageLock.lockfileVersion) >= 3,
  'web/package-lock.json matches package identity and uses lockfileVersion >= 3',
  {
    package_json: { name: webPackage.name, version: webPackage.version },
    package_lock: {
      name: webPackageLock.name ?? null,
      version: webPackageLock.version ?? null,
      lockfileVersion: webPackageLock.lockfileVersion ?? null,
    },
  },
)
record(
  'node.package-lock.root-engine',
  webPackageLock.packages?.['']?.engines?.node === webPackage.engines?.node,
  'web/package-lock root package carries the same Node engine range',
  {
    package_json: webPackage.engines?.node ?? null,
    package_lock: webPackageLock.packages?.['']?.engines?.node ?? null,
  },
)

const canonicalTauri = readJson('src-tauri/tauri.conf.json')
const mirrorTauri = readJson('tauri.conf.json')
record(
  'tauri.frontend-dist.canonical',
  canonicalTauri.build?.frontendDist === EXPECTED_CANONICAL_DIST,
  'Canonical Tauri config uses the src-tauri-relative frontendDist',
  { expected: EXPECTED_CANONICAL_DIST, actual: canonicalTauri.build?.frontendDist ?? null },
)
record(
  'tauri.frontend-dist.mirror',
  mirrorTauri.build?.frontendDist === EXPECTED_MIRROR_DIST,
  'Root Tauri config mirror uses the repo-root-relative frontendDist',
  { expected: EXPECTED_MIRROR_DIST, actual: mirrorTauri.build?.frontendDist ?? null },
)
record(
  'tauri.root-mirror.shared-fields',
  JSON.stringify(comparableTauriShape(canonicalTauri)) === JSON.stringify(comparableTauriShape(mirrorTauri)),
  'Root Tauri config mirror is synced on shared fields',
  {
    canonical: comparableTauriShape(canonicalTauri),
    mirror: comparableTauriShape(mirrorTauri),
  },
)

const canonicalWindows = readJson('src-tauri/tauri.windows.conf.json')
const mirrorWindows = readJson('tauri.windows.conf.json')
const canonicalWindowsShape = windowsConfigShape('src-tauri/tauri.windows.conf.json', canonicalWindows)
const mirrorWindowsShape = windowsConfigShape('tauri.windows.conf.json', mirrorWindows)
record(
  'tauri.windows-mirror.shared-fields',
  JSON.stringify(canonicalWindowsShape) === JSON.stringify(mirrorWindowsShape),
  'Root Windows Tauri config mirror is synced on normalized shared fields',
  { canonical: canonicalWindowsShape, mirror: mirrorWindowsShape },
)

for (const [label, shape] of [
  ['canonical', canonicalWindowsShape],
  ['mirror', mirrorWindowsShape],
]) {
  record(
    `tauri.windows.${label}.target-nsis`,
    shape.bundle.targets.includes('nsis'),
    `${label} Windows Tauri config includes the nsis target`,
    { targets: shape.bundle.targets },
  )
  record(
    `tauri.windows.${label}.webview-install-mode`,
    shape.bundle.windows?.webviewInstallMode?.type === 'downloadBootstrapper',
    `${label} Windows Tauri config declares explicit WebView2 install behavior`,
    { actual: shape.bundle.windows?.webviewInstallMode ?? null },
  )
  record(
    `tauri.windows.${label}.sample-resource-mapping`,
    hasResourceMapping(
      shape.bundle.resources,
      'src-tauri/resources/sample-data/vectorgrid_synthetic_ohlcv.csv',
      'sample-data/vectorgrid_synthetic_ohlcv.csv',
    ),
    `${label} Windows Tauri config maps the synthetic sample CSV`,
    { resources: shape.bundle.resources },
  )
  record(
    `tauri.windows.${label}.catboost-resource-mapping`,
    hasResourceMapping(
      shape.bundle.resources,
      'src-tauri/resources/native/catboostmodel.dll',
      'native/catboostmodel.dll',
    ),
    `${label} Windows Tauri config maps the CatBoost native DLL`,
    { resources: shape.bundle.resources },
  )
  const iconPath = shape.bundle.windows?.nsis?.installerIcon ?? ''
  record(
    `tauri.windows.${label}.installer-icon-exists`,
    typeof iconPath === 'string' && fs.existsSync(iconPath),
    `${label} Windows Tauri config installer icon exists`,
    { path: typeof iconPath === 'string' ? rel(iconPath) : null },
  )
}

const requiredResources = [
  {
    id: 'sample-csv',
    path: 'src-tauri/resources/sample-data/vectorgrid_synthetic_ohlcv.csv',
    role: 'Simple Mode clean-machine first-run path',
  },
  {
    id: 'catboost-dll',
    path: 'src-tauri/resources/native/catboostmodel.dll',
    role: 'ML Signal native CPU fallback resource',
  },
  {
    id: 'double-crossover-ptx',
    path: 'crates/app_core/src/kernels/double_crossover.prebuilt.ptx',
    role: 'GPU-kernel route resource evidence',
  },
  {
    id: 'options-expression-ptx',
    path: 'crates/app_core/src/kernels/options_expression.prebuilt.ptx',
    role: 'Tracked options-expression GPU resource',
  },
]

const resourceSummary = requiredResources.map((resource) => {
  const absolutePath = path.join(repoRoot, resource.path)
  const exists = fs.existsSync(absolutePath)
  return {
    ...resource,
    exists,
    sha256: exists ? sha256File(absolutePath) : null,
    bytes: exists ? fs.statSync(absolutePath).size : null,
  }
})
for (const resource of resourceSummary) {
  record(
    `resources.${resource.id}.exists`,
    resource.exists,
    `${resource.path} exists`,
    { path: resource.path, role: resource.role, sha256: resource.sha256, bytes: resource.bytes },
  )
}

for (const [label, shape] of [
  ['canonical', canonicalWindowsShape],
  ['mirror', mirrorWindowsShape],
]) {
  const missingResourceSources = shape.bundle.resources.filter((entry) => !fs.existsSync(entry.source))
  record(
    `tauri.windows.${label}.resource-sources-exist`,
    missingResourceSources.length === 0,
    `${label} Windows Tauri config resource sources exist`,
    { missing: missingResourceSources },
  )
}

const gitignoreText = readText('.gitignore')
const docsRequiredPatterns = [`!/${DOCS_CONTRACT_DIR}/`, `!/${DOCS_CONTRACT_DIR}/**`]
const missingDocsPatterns = docsRequiredPatterns.filter((pattern) => !gitignoreText.includes(pattern))
record(
  'docs.feature-expansion.unignored-patterns',
  missingDocsPatterns.length === 0,
  'feature expansion docs folder has required .gitignore unignore patterns',
  { required: docsRequiredPatterns, missing: missingDocsPatterns },
)

const docsCheck = runGitCheckIgnored(DOCS_CONTRACT_FILE)
if (docsCheck.checked) {
  record(
    'docs.feature-expansion.git-check-ignore',
    docsCheck.ignored === false,
    'git check-ignore reports the manifest contract doc as unignored',
    { path: DOCS_CONTRACT_FILE, ignored: docsCheck.ignored },
  )
} else {
  recordWarning('docs.feature-expansion.git-check-ignore', 'could not run git check-ignore', {
    path: DOCS_CONTRACT_FILE,
    error: docsCheck.error,
  })
}

const failures = checks.filter((check) => check.status === 'failed')
const warnings = checks.filter((check) => check.status === 'warning')
const report = {
  schema_version: 1,
  generated_at_utc: new Date().toISOString(),
  status: failures.length === 0 ? 'passed' : 'failed',
  repo_root: repoRoot,
  output_path: outPath,
  checks,
  summary: {
    passed: checks.filter((check) => check.status === 'passed').length,
    failed: failures.length,
    warnings: warnings.length,
  },
  vector_ta: {
    expected_version: EXPECTED_VECTOR_TA_VERSION,
    expected_patch: EXPECTED_VECTOR_TA_PATCH,
    dependencies: vectorDependencyChecks,
    vendor_package: {
      name: vendorPackage.name ?? null,
      version: vendorPackage.version ?? null,
      default_features: vendorDefaultFeatures,
    },
  },
  node: {
    package_manager: webPackage.packageManager ?? null,
    engines_node: webPackage.engines?.node ?? null,
    package_lock: {
      lockfileVersion: webPackageLock.lockfileVersion ?? null,
      root_engines_node: webPackageLock.packages?.['']?.engines?.node ?? null,
    },
  },
  tauri: {
    canonical_config: 'src-tauri/tauri.conf.json',
    mirror_config: 'tauri.conf.json',
    canonical_windows_config: 'src-tauri/tauri.windows.conf.json',
    mirror_windows_config: 'tauri.windows.conf.json',
  },
  resources: resourceSummary,
  cpu_only_default_static_parse: {
    forbidden_default_features: [...CPU_ONLY_FORBIDDEN_DEFAULT_FEATURES].sort(),
    surfaces: cpuDefaultSurfaces,
    limitations: [
      'This check validates manifest feature wiring only; it does not execute cargo check or prove runtime resource-loading behavior.',
    ],
  },
}

writeJson(outPath, report)

console.log(`Manifest/toolchain contract: ${report.status}`)
console.log(`  checks: ${report.summary.passed} passed, ${report.summary.failed} failed, ${report.summary.warnings} warnings`)
console.log(`  report: ${outPath}`)

if (warnings.length > 0) {
  console.warn('Warnings:')
  for (const warning of warnings) console.warn(`  - [${warning.id}] ${warning.message}`)
}

if (failures.length > 0) {
  console.error('Failures:')
  for (const failure of failures) console.error(`  - [${failure.id}] ${failure.message}`)
  process.exit(1)
}
