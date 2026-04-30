import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const webRoot = path.resolve(scriptDir, '..')
const distDir = path.resolve(webRoot, 'dist')

const failures = []

function walk(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      walk(fullPath)
    } else {
      const relative = path.relative(webRoot, fullPath).replaceAll(path.sep, '/')
      if (relative.endsWith('.map')) {
        failures.push(`release dist must not contain sourcemaps: ${relative}`)
      }
      if (relative === 'dist/bundle-stats.html') {
        failures.push(`release dist must not contain analyzer output: ${relative}`)
      }
    }
  }
}

if (!fs.existsSync(distDir)) {
  failures.push('release dist directory was not created')
} else {
  walk(distDir)
}

if (failures.length > 0) {
  console.error(failures.join('\n'))
  process.exit(1)
}

console.log('Release dist check passed.')
