import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const webRoot = path.resolve(scriptDir, '..')
const distDir = path.resolve(webRoot, 'dist')

if (path.relative(webRoot, distDir).startsWith('..')) {
  throw new Error(`Refusing to clean dist outside web root: ${distDir}`)
}

fs.rmSync(distDir, { recursive: true, force: true })
console.log(`Removed ${path.relative(webRoot, distDir)}`)
