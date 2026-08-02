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
  'vendor/embed-resource-3.0.6',
]
const EXPECTED_VECTOR_TA_VERSION = '0.2.4'
const EXPECTED_VECTOR_TA_PATCH = 'vendor/vector-ta-0.2.4'
const EXPECTED_EMBED_RESOURCE_VERSION = '3.0.6'
const EXPECTED_EMBED_RESOURCE_PATCH = 'vendor/embed-resource-3.0.6'
const EXPECTED_EMBED_RESOURCE_UPSTREAM_GIT_SHA =
  'f1c7e7acf94ddbd19e6c3b683ba6138c4c9f03b4'
const EXPECTED_PACKAGE_MANAGER = 'npm@11.6.4'
const EXPECTED_NODE_ENGINE = '^20.19.0 || >=22.12.0'
const EXPECTED_CANONICAL_DIST = '../web/dist'
const EXPECTED_MIRROR_DIST = './web/dist'
const CATBOOST_NATIVE_BASELINE_PATH = 'scripts/baselines/catboost-native-v1.2.10.json'
const EXPECTED_TAURI_SUPPORT_BINS = [
  'ta_app_path_faceoff',
  'ta_campaign_perf',
  'ta_campaign_restart_qa',
  'ta_cpu_reliability_soak',
  'ta_double_ma_validation_cost',
  'ta_headless_validate',
  'ta_options_dataset_download',
  'ta_options_dataset_import',
  'ta_options_expression_compare',
  'ta_options_expression_feasibility',
  'ta_options_expression_lifecycle',
  'ta_options_expression_smoke',
  'ta_options_parent_artifact',
  'ta_perf_authority_support',
  'ta_profile_route_matrix',
  'ta_refactor_snapshots',
  'ta_release_readiness',
  'ta_workflow_regression',
]
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

function readCargoMetadata() {
  const result = spawnSync('cargo', ['metadata', '--locked', '--offline', '--format-version', '1'], {
    cwd: repoRoot,
    encoding: 'utf8',
    shell: false,
    maxBuffer: 64 * 1024 * 1024,
  })
  if (result.error) throw result.error
  if ((result.status ?? 1) !== 0) {
    throw new Error(`cargo metadata failed: ${String(result.stderr || result.stdout || '').trim()}`)
  }
  return JSON.parse(result.stdout)
}

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
  embedResourceVendor: `${EXPECTED_EMBED_RESOURCE_PATCH}/Cargo.toml`,
}

const cargo = Object.fromEntries(
  Object.entries(manifestPaths)
    .filter(([, relativePath]) => fs.existsSync(path.join(repoRoot, relativePath)))
    .map(([key, relativePath]) => [key, parseToml(relativePath)]),
)
const cargoMetadata = readCargoMetadata()
const tauriMetadataPackage = cargoMetadata.packages.find(
  (pkg) => pkg.name === 'ta_desktop_demo_app',
)
const tauriBinTargets = (tauriMetadataPackage?.targets ?? [])
  .filter((target) => Array.isArray(target.kind) && target.kind.includes('bin'))
  .sort((left, right) => left.name.localeCompare(right.name))

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

const embedResourcePatch =
  cargo.workspace?.sections['patch.crates-io']?.['embed-resource'] ?? null
const embedResourcePatchPath =
  embedResourcePatch && typeof embedResourcePatch === 'object'
    ? embedResourcePatch.path
    : null
record(
  'embed-resource.patch.path',
  embedResourcePatchPath === EXPECTED_EMBED_RESOURCE_PATCH,
  'Root Cargo.toml patches embed-resource to the expected vendored path',
  {
    expected: EXPECTED_EMBED_RESOURCE_PATCH,
    actual: embedResourcePatchPath,
  },
)
record(
  'embed-resource.patch.exists',
  typeof embedResourcePatchPath === 'string' &&
    fs.existsSync(path.join(repoRoot, embedResourcePatchPath, 'Cargo.toml')),
  'Vendored embed-resource patch Cargo.toml exists',
  { path: embedResourcePatchPath },
)
const embedResourceVendorPackage =
  cargo.embedResourceVendor?.sections.package ?? {}
record(
  'embed-resource.vendor.version',
  embedResourceVendorPackage.name === 'embed-resource' &&
    embedResourceVendorPackage.version === EXPECTED_EMBED_RESOURCE_VERSION,
  'Vendored embed-resource package identity matches the pinned patch',
  {
    expected: {
      name: 'embed-resource',
      version: EXPECTED_EMBED_RESOURCE_VERSION,
    },
    actual: {
      name: embedResourceVendorPackage.name ?? null,
      version: embedResourceVendorPackage.version ?? null,
    },
  },
)
const embedResourceVcsInfoPath =
  `${EXPECTED_EMBED_RESOURCE_PATCH}/.cargo_vcs_info.json`
const embedResourceOriginalManifestPath =
  `${EXPECTED_EMBED_RESOURCE_PATCH}/Cargo.toml.orig`
const embedResourceVcsInfoExists = fs.existsSync(
  path.join(repoRoot, embedResourceVcsInfoPath),
)
const embedResourceVcsInfo = embedResourceVcsInfoExists
  ? readJson(embedResourceVcsInfoPath)
  : null
record(
  'embed-resource.vendor.upstream-revision',
  embedResourceVcsInfo?.git?.sha1 ===
    EXPECTED_EMBED_RESOURCE_UPSTREAM_GIT_SHA &&
    embedResourceVcsInfo?.path_in_vcs === '',
  'Vendored embed-resource records the expected published upstream revision',
  {
    expected: {
      git_sha1: EXPECTED_EMBED_RESOURCE_UPSTREAM_GIT_SHA,
      path_in_vcs: '',
    },
    actual: embedResourceVcsInfo,
    path: embedResourceVcsInfoPath,
  },
)
for (const [id, provenancePath] of [
  ['cargo-vcs-info', embedResourceVcsInfoPath],
  ['original-manifest', embedResourceOriginalManifestPath],
]) {
  const ignoredCheck = runGitCheckIgnored(provenancePath)
  record(
    `embed-resource.vendor.provenance.${id}`,
    fs.existsSync(path.join(repoRoot, provenancePath)) &&
      ignoredCheck.checked &&
      ignoredCheck.ignored === false,
    `Vendored embed-resource ${id} provenance exists and is not ignored`,
    {
      path: provenancePath,
      exists: fs.existsSync(path.join(repoRoot, provenancePath)),
      ignored: ignoredCheck.ignored,
      check_error: ignoredCheck.error,
    },
  )
}

const embedResourceMetadataPackages = cargoMetadata.packages.filter(
  (pkg) => pkg.name === 'embed-resource',
)
const embedResourceMetadataPackage =
  embedResourceMetadataPackages.length === 1
    ? embedResourceMetadataPackages[0]
    : null
const expectedEmbedResourceManifestPath = fs.realpathSync.native(
  path.join(repoRoot, EXPECTED_EMBED_RESOURCE_PATCH, 'Cargo.toml'),
)
const actualEmbedResourceManifestPath =
  typeof embedResourceMetadataPackage?.manifest_path === 'string'
    ? fs.realpathSync.native(embedResourceMetadataPackage.manifest_path)
    : null
record(
  'embed-resource.metadata.identity',
  embedResourceMetadataPackages.length === 1 &&
    embedResourceMetadataPackage?.version === EXPECTED_EMBED_RESOURCE_VERSION,
  'Resolved Cargo metadata contains exactly one embed-resource package at the pinned version',
  {
    expected: {
      count: 1,
      version: EXPECTED_EMBED_RESOURCE_VERSION,
    },
    actual: embedResourceMetadataPackages.map((pkg) => ({
      id: pkg.id,
      version: pkg.version,
      source: pkg.source,
      manifest_path: pkg.manifest_path,
    })),
  },
)
record(
  'embed-resource.metadata.path-source',
  embedResourceMetadataPackage?.source === null &&
    actualEmbedResourceManifestPath === expectedEmbedResourceManifestPath,
  'Resolved embed-resource is the canonical vendored path package',
  {
    expected: {
      source: null,
      manifest_path: expectedEmbedResourceManifestPath,
    },
    actual: {
      source: embedResourceMetadataPackage?.source ?? null,
      manifest_path: actualEmbedResourceManifestPath,
    },
  },
)

const tauriWinresMetadataPackages = cargoMetadata.packages.filter(
  (pkg) => pkg.name === 'tauri-winres',
)
const tauriWinresMetadataPackage =
  tauriWinresMetadataPackages.length === 1
    ? tauriWinresMetadataPackages[0]
    : null
const expectedCratesIoRegistrySource =
  'registry+https://github.com/rust-lang/crates.io-index'
record(
  'tauri-winres.metadata.identity',
  tauriWinresMetadataPackages.length === 1 &&
    tauriWinresMetadataPackage?.version === '0.3.5' &&
    tauriWinresMetadataPackage?.source === expectedCratesIoRegistrySource,
  'Resolved Cargo metadata contains the exact crates.io tauri-winres package',
  {
    expected: {
      count: 1,
      version: '0.3.5',
      source: expectedCratesIoRegistrySource,
    },
    actual: tauriWinresMetadataPackages.map((pkg) => ({
      id: pkg.id,
      version: pkg.version,
      source: pkg.source,
      manifest_path: pkg.manifest_path,
    })),
  },
)

const tauriWinresResolveNode = (cargoMetadata.resolve?.nodes ?? []).find(
  (node) => node.id === tauriWinresMetadataPackage?.id,
)
const tauriWinresEmbedEdges = (tauriWinresResolveNode?.deps ?? []).filter(
  (dependency) => dependency.name === 'embed_resource',
)
const tauriWinresEmbedEdge =
  tauriWinresEmbedEdges.length === 1 ? tauriWinresEmbedEdges[0] : null
const tauriWinresEmbedKinds = tauriWinresEmbedEdge?.dep_kinds ?? []
record(
  'tauri-winres.resolve.embed-resource',
  tauriWinresEmbedEdges.length === 1 &&
    tauriWinresEmbedEdge?.pkg === embedResourceMetadataPackage?.id &&
    tauriWinresEmbedKinds.length === 1 &&
    tauriWinresEmbedKinds[0]?.kind === null &&
    tauriWinresEmbedKinds[0]?.target === null,
  'Resolved tauri-winres dependency edge points to the vendored embed-resource package for all targets',
  {
    expected: {
      dependency_name: 'embed_resource',
      package_id: embedResourceMetadataPackage?.id ?? null,
      dep_kinds: [{ kind: null, target: null }],
    },
    actual: tauriWinresEmbedEdges,
  },
)

const vectorDependencyChecks = []
for (const [key, manifest] of Object.entries(cargo)) {
  if (
    key === 'workspace' ||
    key === 'vectorTaVendor' ||
    key === 'embedResourceVendor'
  ) {
    continue
  }
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
const tauriMaintainerTools = asFeatureList(tauriFeatures['maintainer-tools'])
const tauriCuda = asFeatureList(tauriFeatures.cuda)
const tauriCudaKernel = asFeatureList(tauriFeatures['cuda-backtest-kernel'])
const tauriNvtx = asFeatureList(tauriFeatures['nvtx-profiling'])
const tauriCust = cargo.tauri?.sections.dependencies?.cust ?? null

record(
  'features.tauri.default-desktop-runtime',
  arrayEquals(tauriDefault, ['custom-protocol', 'desktop-ui']),
  'src-tauri default features enable the custom protocol and the required desktop runtime',
  { actual: tauriDefault },
)
record(
  'features.tauri.maintainer-tools-non-default',
  Object.prototype.hasOwnProperty.call(tauriFeatures, 'maintainer-tools') &&
    tauriMaintainerTools.length === 0 &&
    !tauriDefault.includes('maintainer-tools'),
  'src-tauri declares an opt-in maintainer-tools gate that is excluded from product defaults',
  { actual: tauriMaintainerTools, defaults: tauriDefault },
)
const mainTauriBin = tauriBinTargets.find((target) => target.name === 'ta_desktop_demo_app')
const supportTauriBins = tauriBinTargets.filter((target) => target.name !== 'ta_desktop_demo_app')
record(
  'targets.tauri.main-desktop-gate',
  arrayEquals(mainTauriBin?.['required-features'], ['desktop-ui']),
  'the product desktop binary is gated only by desktop-ui',
  { actual: mainTauriBin?.['required-features'] ?? null },
)
record(
  'targets.tauri.support-bin-inventory',
  arrayEquals(
    supportTauriBins.map((target) => target.name),
    EXPECTED_TAURI_SUPPORT_BINS,
  ),
  'the explicit Tauri support-binary inventory matches the release contract',
  {
    expected: EXPECTED_TAURI_SUPPORT_BINS,
    actual: supportTauriBins.map((target) => target.name),
  },
)
record(
  'targets.tauri.support-bins-maintainer-only',
  supportTauriBins.length === EXPECTED_TAURI_SUPPORT_BINS.length &&
    supportTauriBins.every((target) =>
      arrayEquals(target['required-features'], ['maintainer-tools']),
    ),
  'every non-product Tauri binary is gated by maintainer-tools',
  {
    actual: supportTauriBins.map((target) => ({
      name: target.name,
      required_features: target['required-features'] ?? null,
    })),
  },
)
record(
  'features.tauri.cuda-forwarding',
  tauriCuda.includes('ta_app_core/cuda') &&
    tauriCuda.includes('vector-ta/cuda') &&
    !tauriCuda.includes('cust') &&
    !tauriCuda.includes('dep:cust'),
  'src-tauri cuda feature forwards to app_core/vector-ta without owning cust directly',
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
  'features.tauri.no-direct-cust',
  tauriCust === null,
  'src-tauri does not own a direct cust dependency; app_core owns optional CUDA runtime binding',
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
  record(
    `tauri.windows.${label}.catboost-license-resource-mapping`,
    hasResourceMapping(
      shape.bundle.resources,
      'src-tauri/resources/licenses/CatBoost-LICENSE.txt',
      'licenses/CatBoost-LICENSE.txt',
    ),
    `${label} Windows Tauri config maps the CatBoost license`,
    { resources: shape.bundle.resources },
  )
  record(
    `tauri.windows.${label}.lightweight-charts-license-resource-mapping`,
    hasResourceMapping(
      shape.bundle.resources,
      'src-tauri/resources/licenses/TradingView-Lightweight-Charts-LICENSE.txt',
      'licenses/TradingView-Lightweight-Charts-LICENSE.txt',
    ),
    `${label} Windows Tauri config maps the TradingView Lightweight Charts license`,
    { resources: shape.bundle.resources },
  )
  record(
    `tauri.windows.${label}.lightweight-charts-notice-resource-mapping`,
    hasResourceMapping(
      shape.bundle.resources,
      'src-tauri/resources/licenses/TradingView-Lightweight-Charts-NOTICE.txt',
      'licenses/TradingView-Lightweight-Charts-NOTICE.txt',
    ),
    `${label} Windows Tauri config maps the TradingView Lightweight Charts notice`,
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
    role: 'Advanced workbench clean-machine first-run path',
  },
  {
    id: 'catboost-dll',
    path: 'src-tauri/resources/native/catboostmodel.dll',
    role: 'ML Signal native CPU fallback resource',
  },
  {
    id: 'catboost-license',
    path: 'src-tauri/resources/licenses/CatBoost-LICENSE.txt',
    role: 'Required third-party license notice for the packaged CatBoost native resource',
  },
  {
    id: 'lightweight-charts-license',
    path: 'src-tauri/resources/licenses/TradingView-Lightweight-Charts-LICENSE.txt',
    role: 'Apache-2.0 license for the packaged Lightweight Charts frontend dependency',
  },
  {
    id: 'lightweight-charts-notice',
    path: 'src-tauri/resources/licenses/TradingView-Lightweight-Charts-NOTICE.txt',
    role: 'Required TradingView attribution notice for Lightweight Charts 5.1.0',
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

const catboostBaselinePath = path.join(repoRoot, CATBOOST_NATIVE_BASELINE_PATH)
const catboostBaselineExists = fs.existsSync(catboostBaselinePath)
record(
  'resources.catboost-baseline.exists',
  catboostBaselineExists,
  `${CATBOOST_NATIVE_BASELINE_PATH} exists`,
  { path: CATBOOST_NATIVE_BASELINE_PATH },
)
if (catboostBaselineExists) {
  const baseline = readJson(CATBOOST_NATIVE_BASELINE_PATH)
  const catboostDll = resourceSummary.find((resource) => resource.id === 'catboost-dll')
  const catboostLicense = resourceSummary.find((resource) => resource.id === 'catboost-license')
  const expectedDllSha256 = String(baseline.asset_sha256 ?? '').toLowerCase()
  const expectedLicenseSha256 = String(baseline.license_sha256 ?? '').toLowerCase()
  record(
    'resources.catboost-dll.matches-reviewed-baseline',
    catboostDll?.exists === true &&
      catboostDll.sha256 === expectedDllSha256 &&
      catboostDll.bytes === Number(baseline.asset_size_bytes),
    'Bundled CatBoost DLL matches the reviewed upstream release asset',
    {
      version: baseline.version ?? null,
      expected_sha256: expectedDllSha256,
      actual_sha256: catboostDll?.sha256 ?? null,
      expected_bytes: baseline.asset_size_bytes ?? null,
      actual_bytes: catboostDll?.bytes ?? null,
      release_url: baseline.release_url ?? null,
    },
  )
  record(
    'resources.catboost-license.matches-reviewed-baseline',
    catboostLicense?.exists === true && catboostLicense.sha256 === expectedLicenseSha256,
    'Bundled CatBoost license matches the reviewed upstream tag',
    {
      license: baseline.license ?? null,
      expected_sha256: expectedLicenseSha256,
      actual_sha256: catboostLicense?.sha256 ?? null,
      license_url: baseline.license_url ?? null,
    },
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

const packagedSmokeText = readText('scripts/run_packaged_smoke.ps1')
const releaseManifestText = readText('scripts/generate_release_manifest.ps1')
const releaseCandidateText = readText('scripts/run_release_candidate_gate.ps1')
const publicReleaseText = readText('scripts/run_public_release_gate.ps1')
const provenanceText = readText('scripts/capture_release_provenance.ps1')
const cpuCiWorkflowText = readText('.github/workflows/cpu-quality.yml')
const dependabotText = readText('.github/dependabot.yml')
const cpuCiGateText = readText('scripts/run_cpu_ci_gate.ps1')
const ciDocsText = readText('docs/CI.md')
const gitignoreText = readText('.gitignore')
const outputParityPack = readJson('scripts/headless_output_parity_pack.json')
const releaseCandidateSmokeArgs =
  releaseCandidateText.match(/\$smokeArgs\s*=\s*@\(([\s\S]*?)\r?\n\s*\)/)?.[1] ?? ''
record(
  'release.packaged-smoke.extracts-staged-bundle',
  packagedSmokeText.includes('function Expand-NsisBundleProduct') &&
    packagedSmokeText.includes('Resolve-ManifestBundleArtifact') &&
    packagedSmokeText.includes('binaryOrigin = "extracted-bundle"'),
  'bundle-mode smoke resolves and extracts the staged installer product',
)
record(
  'release.packaged-smoke.rejects-raw-override',
  packagedSmokeText.includes('-BinaryPath cannot override a bundle manifest') &&
    !releaseCandidateSmokeArgs.includes('"-BinaryPath"'),
  'bundle-mode smoke cannot be redirected to the raw target executable',
)
record(
  'release.candidate-desktop-suite-contract',
  releaseCandidateText.includes(
    '[ValidateSet("smoke", "smoke-expanded", "workflow", "clean-start", "route-matrix", "release-proof")]',
  ) &&
    releaseCandidateText.includes('[string]$DesktopSuite = "smoke"') &&
    releaseCandidateText.includes('$PSBoundParameters.ContainsKey("DesktopSuite")') &&
    releaseCandidateText.includes(
      '-DesktopSuite requires -IncludeDesktopSmoke and cannot be combined with -SkipPackagedSmoke.',
    ) &&
    releaseCandidateSmokeArgs.includes('"-DesktopSuite"') &&
    releaseCandidateSmokeArgs.includes('$DesktopSuite') &&
    releaseCandidateText.includes('desktop_suite = $DesktopSuite'),
  'release candidates validate, forward, and record the selected packaged desktop suite while defaulting ordinary rehearsals to smoke',
)
record(
  'release.packaged-smoke-schema2-app-attestation',
  packagedSmokeText.includes('[int]$profileIsolationEvidence.schemaVersion -ne 2') &&
    packagedSmokeText.includes(
      '-not [bool]$profileIsolationEvidence.effectiveIsolationProven',
    ) &&
    packagedSmokeText.includes(
      '[string]$appAttestation.kind -ne "vectorgrid-qa-profile-attestation"',
    ) &&
    packagedSmokeText.includes(
      'Test-PathEquals -Left ([string]$appAttestation.effectiveIdentifier) -Right $expectedIsolatedAppData',
    ) &&
    packagedSmokeText.includes('effective_isolation_proven = if') &&
    packagedSmokeText.includes('app_attestation = if') &&
    packagedSmokeText.includes(
      'did not prove a schema-2 isolated-profile lifecycle with an exact matching application attestation',
    ),
  'release-proof packaged smoke requires schema-2 effective app-data isolation and exports the exact application attestation',
)
record(
  'release.public-gate-requires-exact-package-suite',
  publicReleaseText.includes('"-DesktopSuite", "release-proof"') &&
    publicReleaseText.includes('[string]$candidateSummary.desktop_suite -ne "release-proof"') &&
    publicReleaseText.includes('[string]$smoke.desktop_suite -ne "release-proof"') &&
    publicReleaseText.includes('-not [bool]$smoke.exact_package_release_proof') &&
    publicReleaseText.includes('[string]$smoke.release_variant -ne $Variant') &&
    publicReleaseText.includes(
      'Public release requires the exact-package release-proof desktop suite for the current release variant.',
    ) &&
    publicReleaseText.includes('$profileIsolation = $smoke.profile_isolation') &&
    publicReleaseText.includes('$null -eq $profileIsolation') &&
    publicReleaseText.includes('[int]$profileIsolation.schema_version -ne 2') &&
    publicReleaseText.includes('[string]$profileIsolation.status -ne "passed"') &&
    publicReleaseText.includes('-not [bool]$profileIsolation.real_profile_unchanged') &&
    publicReleaseText.includes('-not [bool]$profileIsolation.effective_isolation_proven') &&
    publicReleaseText.includes('[string]$profileIsolation.teardown -ne "completed"') &&
    publicReleaseText.includes('$null -eq $appAttestation') &&
    publicReleaseText.includes(
      '[string]$appAttestation.kind -ne "vectorgrid-qa-profile-attestation"',
    ) &&
    publicReleaseText.includes(
      'Test-PathEquals -Left ([string]$appAttestation.app_data_dir) -Right $expectedIsolatedAppData',
    ) &&
    publicReleaseText.includes(
      'exact_package_release_proof = if ($null -ne $smoke)',
    ) &&
    publicReleaseText.includes(
      'profile_isolation_status = if ($null -ne $smoke',
    ) &&
    publicReleaseText.includes('profile_isolation_schema_version = if') &&
    publicReleaseText.includes('effective_profile_isolation_proven = if') &&
    publicReleaseText.includes('profile_app_attestation = if') &&
    packagedSmokeText.includes('$requiresIsolatedProfile') &&
    packagedSmokeText.includes('VECTORGRID_DESKTOP_SMOKE_ISOLATED_PROFILE_ROOT') &&
    packagedSmokeText.includes(
      'did not prove a schema-2 isolated-profile lifecycle with an exact matching application attestation',
    ),
  'public release explicitly requires the exact-package suite plus schema-2 effective-isolation and exact application-attestation evidence',
)
record(
  'release.release-proof-journey-evidence-authority',
  packagedSmokeText.includes('release_proof_evidence_contract.mjs') &&
    packagedSmokeText.includes('"--mode", "write"') &&
    packagedSmokeText.includes(
      '@($releaseProofEvidence.journeys).Count -ne [int]$releaseProofEvidence.required_journey_count',
    ) &&
    publicReleaseText.includes('"--mode", "verify"') &&
    publicReleaseText.includes('$script:ReleaseProofEvidenceVerification') &&
    publicReleaseText.includes(
      '[int]$script:ReleaseProofEvidenceVerification.manifest.required_journey_count -ne [int]$releaseProofEvidence.required_journey_count',
    ) &&
    publicReleaseText.includes(
      '[string]$script:ReleaseProofEvidenceVerification.manifest_sha256 -ne [string]$releaseProofEvidence.manifest_sha256',
    ),
  'packaged smoke binds every allowlisted journey to one run/variant/binary manifest and the public gate independently revalidates it',
)
record(
  'release.packaged-smoke.static-mode-not-runtime-proof',
  packagedSmokeText.includes('"bundle-static-inspection"') &&
    packagedSmokeText.includes('runtime_executed = -not [bool]$StaticInspectionOnly'),
  'static package inspection remains distinct from runtime packaged-smoke evidence',
)
record(
  'release.packaged-smoke.allowlists-and-hashes-payload',
  packagedSmokeText.includes('Extracted NSIS payload contains unexpected files') &&
    packagedSmokeText.includes('catboost_bundle_resource_sha256') &&
    packagedSmokeText.includes('catboost_bundle_resource_expected_sha256') &&
    packagedSmokeText.includes('catboost_license_sha256') &&
    packagedSmokeText.includes('catboost_license_expected_sha256') &&
    packagedSmokeText.includes('lightweight_charts_license_sha256') &&
    packagedSmokeText.includes('lightweight_charts_license_expected_sha256') &&
    packagedSmokeText.includes('lightweight_charts_notice_sha256') &&
    packagedSmokeText.includes('lightweight_charts_notice_expected_sha256') &&
    packagedSmokeText.includes('synthetic_sample_csv_sha256') &&
    packagedSmokeText.includes('bundle_verified_resources'),
  'bundle inspection rejects unexpected payloads and verifies required resource hashes',
)
record(
  'release.public-gate.requires-extracted-inner-signature',
  publicReleaseText.includes('[string]$smoke.binary_origin -ne "extracted-bundle"') &&
    publicReleaseText.includes('[string]$smoke.binary_code_signature.status -ne "Valid"'),
  'public release requires runtime smoke and a valid signature on the installer-embedded application',
)
record(
  'release.public-gate.dirty-runs-are-rehearsal-only',
  publicReleaseText.includes('-AllowDirtyWorktree is rehearsal-only and requires -AllowUnsignedBeta') &&
    publicReleaseText.includes('"rehearsal-passed"') &&
    publicReleaseText.includes('public_release_eligible'),
  'dirty worktrees cannot produce publishable public-release success evidence',
)
record(
  'release.manifest-is-staging-not-publication-authority',
  releaseManifestText.includes('publishable = $false') &&
    releaseManifestText.includes('installer_signature_eligible') &&
    releaseManifestText.includes('public-release-summary.json is final publication authority') &&
    releaseCandidateText.includes('installer_signature_eligible'),
  'staging manifests cannot independently claim final publication eligibility',
)
record(
  'release.output-directories-reject-stale-evidence',
  releaseCandidateText.includes('function Assert-FreshOutputDirectory') &&
    releaseCandidateText.includes('Release output directory is not empty.') &&
    publicReleaseText.includes('function Assert-FreshOutputDirectory') &&
    publicReleaseText.includes('Public release output directory is not empty.'),
  'candidate and public release gates require a new or empty ordinary output directory',
)
record(
  'release.staged-artifact-directory-is-exact',
  releaseManifestText.includes('function Assert-EmptyArtifactStagingDirectory') &&
    releaseManifestText.includes('function Assert-ExactStagedArtifactSet') &&
    releaseManifestText.includes(
      'Release artifact staging directory does not exactly match the generated manifest artifact set.',
    ) &&
    publicReleaseText.includes(
      'Staged release artifacts directory contains files not declared exactly once by manifest.staged_artifacts.',
    ) &&
    publicReleaseText.includes(
      'Staged release artifact is not an immediate child of manifest.staged_artifacts_dir',
    ),
  'release staging rejects stale or nested artifacts and the public gate rechecks the exact manifest-declared file set',
)
record(
  'release.candidate-skip-workflow-is-rehearsal-only',
  releaseCandidateText.includes('[switch]$SkipWorkflowGate') &&
    releaseCandidateText.includes('skip_workflow_gate = [bool]$SkipWorkflowGate') &&
    releaseCandidateText.includes('workflow_authority_complete = -not [bool]$SkipWorkflowGate') &&
    releaseCandidateText.includes('if ($SkipWorkflowGate -and -not $UnsignedBeta)') &&
    releaseCandidateText.includes('if ($SkipWorkflowGate -and ($IncludePerf -or $IncludeGpuValidationPerf))') &&
    releaseCandidateText.includes('-SkipWorkflowGate is rehearsal-only and requires -UnsignedBeta.') &&
    releaseCandidateText.includes('Workflow gate was explicitly deferred for this unsigned rehearsal.') &&
    publicReleaseText.includes('Public release requires complete workflow authority; a candidate created with -SkipWorkflowGate is rehearsal-only.'),
  'skipping the workflow/performance authority is explicit, recorded, and restricted to unsigned rehearsals',
)
record(
  'release.output-parity-authority-is-eligible-for-clean-clone-landing',
  [
    'scripts/run_output_parity_check.mjs',
    'scripts/check_output_parity.mjs',
    'scripts/check_output_semantic_parity.mjs',
    'scripts/headless_output_parity_pack.json',
    'scripts/output_parity_pairs.json',
    'scripts/output_parity_semantic_pairs.json',
    'scripts/baselines/samples/report.json',
    'scripts/baselines/samples/report.xlsx',
  ].every((filePath) => fs.existsSync(path.join(repoRoot, filePath))) &&
    gitignoreText.includes('!/scripts/run_output_parity_check.mjs') &&
    gitignoreText.includes('!/scripts/baselines/samples/report.json') &&
    gitignoreText.includes('!/scripts/baselines/samples/report.xlsx'),
  'public-release output-parity scripts, configs, and minimal baselines are present and explicitly unignored so they can be landed',
)
const outputParityArgs = Array.isArray(outputParityPack.args)
  ? outputParityPack.args.map((value) => String(value))
  : []
record(
  'release.output-parity-uses-tracked-sample',
  outputParityArgs.filter(
    (value) => value === 'src-tauri/resources/sample-data/vectorgrid_synthetic_ohlcv.csv',
  ).length === 2 &&
    !outputParityArgs.some((value) => /(?:^|[\\/])data[\\/]AAPL\.csv$/i.test(value)),
  'clean-clone output parity uses the tracked deterministic sample for primary and cross inputs',
)
record(
  'release.public-gate.pins-and-timestamps-signer',
  publicReleaseText.includes('ExpectedSignerThumbprint') &&
    publicReleaseText.includes('VECTORGRID_EXPECTED_SIGNER_THUMBPRINT') &&
    publicReleaseText.includes('timestamp_thumbprint') &&
    publicReleaseText.includes('signatures must match the expected publisher'),
  'final public release pins and timestamps both inner and outer Authenticode signatures',
)
record(
  'release.public-gate.final-rehash',
  publicReleaseText.includes('Staged artifact hash changed after manifest generation') &&
    publicReleaseText.includes('SHA256SUMS.txt') &&
    publicReleaseText.includes('final_artifact_hashes') &&
    publicReleaseText.includes('Final Authenticode recheck failed for the embedded application') &&
    publicReleaseText.includes('Embedded application hash changed after packaged smoke'),
  'final public gate re-hashes staged and embedded artifacts and rechecks their signatures',
)
record(
  'release.public-gpu-variants-require-performance-authority',
  publicReleaseText.includes(
    '@("gpu-ma", "gpu-kernel") -contains $Variant -and -not $IncludePerf',
  ) &&
    publicReleaseText.includes(
      'Public GPU releases require -IncludePerf for the variant-specific optimization performance authority.',
    ) &&
    publicReleaseText.includes(
      '$Variant -eq "gpu-kernel" -and -not $IncludeGpuValidationPerf',
    ) &&
    publicReleaseText.includes(
      'Public gpu-kernel releases require -IncludeGpuValidationPerf for fixed-candidate GPU validation authority.',
    ),
  'public GPU variants cannot claim eligibility without their documented optimization and validation performance lanes',
)
record(
  'release.provenance.repo-context-and-cuda-tools',
  provenanceText.includes('git -C $repoRoot') &&
    provenanceText.includes('Push-Location $repoRoot') &&
    provenanceText.includes('selected_nvcc') &&
    provenanceText.includes('path_nvcc'),
  'release provenance is repo-context anchored and distinguishes selected versus PATH CUDA tools',
)
record(
  'ci.required-cpu-workflow-prepared',
  cpuCiWorkflowText.includes('pull_request:') &&
    cpuCiWorkflowText.includes('push:') &&
    cpuCiWorkflowText.includes('merge_group:') &&
    cpuCiWorkflowText.includes('workflow_dispatch:') &&
    cpuCiWorkflowText.includes('name: Required CPU quality') &&
    cpuCiWorkflowText.includes('runs-on: windows-2022') &&
    cpuCiWorkflowText.includes('scripts/run_cpu_ci_gate.ps1'),
  'prepared CPU workflow exposes the stable required check on PR, push, merge-queue, and manual triggers',
)
const cpuCiActionRefs = Array.from(
  cpuCiWorkflowText.matchAll(/uses:\s+([^\s#]+)/g),
  (match) => match[1],
)
record(
  'ci.workflow-actions-use-immutable-revisions',
  cpuCiActionRefs.length > 0 &&
    cpuCiActionRefs.every((reference) => /@[0-9a-f]{40}$/i.test(reference)) &&
    dependabotText.includes('package-ecosystem: github-actions') &&
    dependabotText.includes('interval: weekly'),
  'every external GitHub Action is pinned to a full immutable commit SHA and has a weekly update lane',
  { action_refs: cpuCiActionRefs },
)
record(
  'ci.required-cpu-toolchain-and-browser-contract',
  cpuCiWorkflowText.includes('node-version: "22.12.0"') &&
    cpuCiWorkflowText.includes('web/package.json') &&
    cpuCiWorkflowText.includes('npm.cmd --prefix web ci') &&
    cpuCiWorkflowText.includes('playwright.cmd install chromium') &&
    cpuCiWorkflowText.includes('CARGO_BUILD_JOBS: "1"') &&
    cpuCiWorkflowText.includes('CARGO_INCREMENTAL: "0"') &&
    cpuCiWorkflowText.includes('RUST_TEST_THREADS: "1"'),
  'required CPU workflow pins a supported Node selection, derives npm from the repo contract, installs locked dependencies/Chromium, and serializes Rust work',
)
record(
  'ci.required-cpu-gate-scope',
  cpuCiGateText.includes('cargo fetch --locked') &&
    cpuCiGateText.includes('cargo metadata --locked --offline --format-version 1') &&
    cpuCiGateText.indexOf('cargo fetch --locked') <
      cpuCiGateText.indexOf('cargo metadata --locked --offline --format-version 1') &&
    cpuCiGateText.indexOf('cargo metadata --locked --offline --format-version 1') <
      cpuCiGateText.indexOf('run_repo_quality_gate.ps1') &&
    cpuCiGateText.includes('run_repo_quality_gate.ps1') &&
    cpuCiGateText.includes('Strict Clippy: ta_strategies') &&
    cpuCiGateText.includes('cargo clippy --locked -p ta_strategies') &&
    cpuCiGateText.includes('Strict Clippy: ta_optimizer') &&
    cpuCiGateText.includes('cargo clippy --locked -p ta_optimizer') &&
    !cpuCiGateText.includes('-StrictClippy') &&
    cpuCiGateText.includes('-IncludeWorkspaceTests') &&
    cpuCiGateText.includes('--maxWorkers=1') &&
    cpuCiGateText.includes('--fileParallelism=false') &&
    cpuCiGateText.includes('-IncludeFullWorkflow') &&
    cpuCiGateText.includes('-IncludeWcag22Smoke') &&
    cpuCiGateText.includes('run_output_parity_check.mjs') &&
    !cpuCiGateText.includes('run_gpu_perf_gate.ps1') &&
    !cpuCiGateText.includes('run_cpu_perf_gate.ps1') &&
    !cpuCiGateText.includes('test:desktop:smoke') &&
    !cpuCiGateText.includes('-IncludeDesktopSmoke'),
  'required CPU gate is comprehensive, serialized, headless, strictly lints the warning-free crates, and excludes performance/GPU/native-desktop execution',
)
record(
  'ci.required-check-documented',
  ciDocsText.includes('Required CPU quality') &&
    ciDocsText.includes('branch protection') &&
    ciDocsText.includes('Separate lanes') &&
    ciDocsText.includes('clean-VM') &&
    gitignoreText.includes('!/docs/CI.md'),
  'required check name and separate GPU/native/clean-machine lanes are documented',
)
record(
  'ci.required-evidence-is-redacted',
  cpuCiGateText.includes('build_support_evidence_bundle.ps1') &&
    cpuCiGateText.includes('output\\qa\\required-cpu-quality') &&
    cpuCiGateText.includes('-EvidenceRoot $currentEvidenceRoot') &&
    cpuCiGateText.includes('-AutomaticUpload') &&
    cpuCiWorkflowText.includes(
      'output/support-evidence/ci/**/vectorgrid-support-evidence.zip',
    ) &&
    cpuCiWorkflowText.includes(
      'output/support-evidence/ci/**/vectorgrid-support-evidence.zip.sha256',
    ) &&
    !cpuCiWorkflowText.includes('output/qa/**') &&
    !cpuCiWorkflowText.includes('output/playwright/**') &&
    ciDocsText.includes('Raw QA directories') &&
    ciDocsText.includes('are not uploaded'),
  'required CPU CI uploads only the fixed-allowlist redacted support archive and checksum',
)

const docsRequiredPatterns = [`!/${DOCS_CONTRACT_DIR}/`, `!/${DOCS_CONTRACT_DIR}/**`]
const missingDocsPatterns = docsRequiredPatterns.filter((pattern) => !gitignoreText.includes(pattern))
record(
  'docs.feature-expansion.unignored-patterns',
  missingDocsPatterns.length === 0,
  'feature expansion docs folder has required .gitignore unignore patterns',
  { required: docsRequiredPatterns, missing: missingDocsPatterns },
)

const manifestContractDocsText = readText(DOCS_CONTRACT_FILE)
record(
  'docs.manifest-contract.embed-resource-resolution',
  manifestContractDocsText.includes('vendor/embed-resource-3.0.6') &&
    manifestContractDocsText.includes('.cargo_vcs_info.json') &&
    manifestContractDocsText.includes('Cargo.toml.orig') &&
    manifestContractDocsText.includes('tauri-winres 0.3.5') &&
    manifestContractDocsText.includes(
      'embed-resource 3.0.6` resolution',
    ),
  'manifest/dependency contract documents the vendored resource crate provenance and exact resolved edge',
  { path: DOCS_CONTRACT_FILE },
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
  embed_resource: {
    expected_version: EXPECTED_EMBED_RESOURCE_VERSION,
    expected_patch: EXPECTED_EMBED_RESOURCE_PATCH,
    expected_upstream_git_sha: EXPECTED_EMBED_RESOURCE_UPSTREAM_GIT_SHA,
    vendor_package: {
      name: embedResourceVendorPackage.name ?? null,
      version: embedResourceVendorPackage.version ?? null,
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
