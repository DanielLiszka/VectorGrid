import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'

function walkFiles(rootDir, predicate, out = []) {
  const entries = fs.readdirSync(rootDir, { withFileTypes: true })
  for (const entry of entries) {
    const full = path.join(rootDir, entry.name)
    if (entry.isDirectory()) {
      if (entry.name === 'node_modules' || entry.name === 'dist' || entry.name === '.git') continue
      if (full.endsWith(path.join('src', 'legacy', 'vendor'))) continue
      walkFiles(full, predicate, out)
      continue
    }
    if (predicate(full)) out.push(full)
  }
  return out
}

function isIgnoredSourceFile(filePath) {
  return /\.test\.(ts|tsx|js|mjs)$/.test(filePath)
}

function lineForIndex(text, index) {
  let line = 1
  for (let i = 0; i < index; i += 1) {
    if (text.charCodeAt(i) === 10) line += 1
  }
  return line
}

function parseRegistryCommands(tsText) {
  const re = /'([a-z_][a-z0-9_]*)'/g
  const out = new Set()
  for (const m of tsText.matchAll(re)) {
    out.add(m[1])
  }
  return out
}

function gatherCommandLiterals(filePath, text) {
  const patterns = [
    /\binvoke\s*\(\s*["']([a-z_][a-z0-9_]*)["']/g,
    /\binvokeCommand\s*\(\s*["']([a-z_][a-z0-9_]*)["']/g,
    /\binvokeByName\s*\(\s*["']([a-z_][a-z0-9_]*)["']/g,
  ]
  const out = []
  for (const re of patterns) {
    for (const m of text.matchAll(re)) {
      out.push({
        command: m[1],
        line: lineForIndex(text, m.index ?? 0),
        filePath,
      })
    }
  }
  return out
}

function main() {
  const webRoot = process.cwd()
  const srcRoot = path.join(webRoot, 'src')
  const scriptsRoot = path.join(webRoot, 'scripts')
  const registryPath = path.join(srcRoot, 'tauri', 'commands.ts')

  const registryText = fs.readFileSync(registryPath, 'utf8')
  const knownCommands = parseRegistryCommands(registryText)

  const files = [
    ...walkFiles(srcRoot, (p) => /\.(ts|tsx|js|mjs|vue)$/.test(p)),
    ...walkFiles(scriptsRoot, (p) => /\.mjs$/.test(p)),
  ]

  const unknownRefs = []
  for (const filePath of files) {
    if (isIgnoredSourceFile(filePath)) continue
    const text = fs.readFileSync(filePath, 'utf8')
    const refs = gatherCommandLiterals(filePath, text)
    for (const ref of refs) {
      if (!knownCommands.has(ref.command)) {
        unknownRefs.push(ref)
      }
    }
  }

  if (unknownRefs.length > 0) {
    console.error('ERROR: Unknown Tauri command literals found:')
    for (const ref of unknownRefs) {
      const rel = path.relative(webRoot, ref.filePath).replace(/\\/g, '/')
      console.error(`  - ${rel}:${ref.line} -> ${ref.command}`)
    }
    process.exit(1)
  }

  console.log(`OK: All literal invoke command names are known (${knownCommands.size} registered commands).`)
}

main()
