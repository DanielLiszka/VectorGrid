import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'

function readText(filePath) {
  return fs.readFileSync(filePath, 'utf8')
}

function uniqueSorted(items) {
  return Array.from(new Set(items)).sort()
}

function parseHandlerCommands(libRsText) {
  const m = libRsText.match(/generate_handler!\[(?<body>[\s\S]*?)\]/m)
  if (!m?.groups?.body) return []
  const body = m.groups.body
  const out = []
  const re = /\b([a-z_][a-z0-9_]*)\b/g
  for (const mm of body.matchAll(re)) {
    const name = mm[1]
    if (name === 'tauri' || name === 'generate_handler') continue
    out.push(name)
  }
  return uniqueSorted(out)
}

function parseRegistryCommands(tsText) {
  const re = /'([a-z_][a-z0-9_]*)'/g
  const out = []
  for (const m of tsText.matchAll(re)) {
    out.push(m[1])
  }
  return uniqueSorted(out)
}

function diff(left, right) {
  const r = new Set(right)
  return left.filter((x) => !r.has(x))
}

function main() {
  const webRoot = process.cwd()
  const repoRoot = path.resolve(webRoot, '..')
  const libRsPath = path.join(repoRoot, 'src-tauri', 'src', 'lib.rs')
  const registryPath = path.join(webRoot, 'src', 'tauri', 'commands.ts')
  const legacyAppPath = path.join(webRoot, 'src', 'legacy', 'app.js')

  const libRs = readText(libRsPath)
  const registryTs = readText(registryPath)
  const legacyApp = readText(legacyAppPath)

  const backend = parseHandlerCommands(libRs)
  const frontend = parseRegistryCommands(registryTs)

  const missingInFrontend = diff(backend, frontend)
  const extraInFrontend = diff(frontend, backend)

  let ok = true
  if (missingInFrontend.length > 0) {
    ok = false
    console.error('ERROR: Commands missing in frontend registry:')
    for (const c of missingInFrontend) console.error(`  - ${c}`)
  }

  if (extraInFrontend.length > 0) {
    ok = false
    console.error('ERROR: Extra commands in frontend registry not present in backend:')
    for (const c of extraInFrontend) console.error(`  - ${c}`)
  }

  const notReferencedInLegacy = backend.filter((cmd) => !new RegExp(`\\b${cmd}\\b`).test(legacyApp))
  if (notReferencedInLegacy.length > 0) {
    console.warn('WARN: Backend commands not referenced by name in legacy app:')
    for (const c of notReferencedInLegacy) console.warn(`  - ${c}`)
  }

  console.log(`Backend commands: ${backend.length}`)
  console.log(`Frontend registry commands: ${frontend.length}`)

  if (!ok) process.exit(1)
}

main()
