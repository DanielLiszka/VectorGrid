import { describe, expect, it } from 'vitest'
import { readdirSync, readFileSync, statSync } from 'node:fs'
import path from 'node:path'

function listVueFiles(dir: string): string[] {
  const files: string[] = []
  for (const entry of readdirSync(dir)) {
    const full = path.join(dir, entry)
    const stats = statSync(full)
    if (stats.isDirectory()) {
      files.push(...listVueFiles(full))
      continue
    }
    if (full.endsWith('.vue')) files.push(full)
  }
  return files
}

describe('component action ownership', () => {
  it('keeps invokeLegacyAction out of Vue components', () => {
    const componentsDir = path.resolve(__dirname, '../components')
    const files = listVueFiles(componentsDir)
    const offenders: string[] = []
    for (const file of files) {
      const source = readFileSync(file, 'utf8')
      if (source.includes('invokeLegacyAction(')) offenders.push(path.basename(file))
    }
    expect(offenders).toEqual([])
  })
})
