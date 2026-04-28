import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

const srcRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const allowedRawTauriImports = new Set([
  path.resolve(srcRoot, 'tauri', 'events.ts'),
  path.resolve(srcRoot, 'tauri', 'generatedBindings.ts'),
  path.resolve(srcRoot, 'tauri', 'invoke.ts'),
])

function collectSourceFiles(rootDir: string): string[] {
  const entries = fs.readdirSync(rootDir, { withFileTypes: true })
  const files: string[] = []

  for (const entry of entries) {
    const nextPath = path.resolve(rootDir, entry.name)
    if (entry.isDirectory()) {
      files.push(...collectSourceFiles(nextPath))
      continue
    }
    if (!entry.isFile()) continue
    if (!/\.(ts|vue)$/.test(entry.name)) continue
    if (entry.name.endsWith('.test.ts') || entry.name.endsWith('.d.ts')) continue
    if (nextPath.includes(`${path.sep}test${path.sep}`)) continue
    files.push(nextPath)
  }

  return files
}

describe('tauri runtime wiring policy', () => {
  it('keeps raw Tauri core/event imports confined to the shared wrapper layer', () => {
    const offenders = collectSourceFiles(srcRoot)
      .filter((filePath) => {
        const body = fs.readFileSync(filePath, 'utf8')
        return /@tauri-apps\/api\/(?:core|event)/.test(body) && !allowedRawTauriImports.has(filePath)
      })
      .map((filePath) => path.relative(srcRoot, filePath))
      .sort()

    expect(offenders).toEqual([])
  })
})
