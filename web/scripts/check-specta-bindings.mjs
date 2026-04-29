import { spawnSync } from 'node:child_process'
import path from 'node:path'
import process from 'node:process'

const webRoot = process.cwd()
const repoRoot = path.resolve(webRoot, '..')

const cargoBin = process.platform === 'win32' ? 'cargo.exe' : 'cargo'
const cargoEnv = { ...process.env }

if (process.platform === 'win32') {
  cargoEnv.CARGO_BUILD_JOBS = '1'
}

const result = spawnSync(
  cargoBin,
  [
    'test',
    '-p',
    'ta_desktop_demo_app',
    '--test',
    'specta_bindings_export',
    'specta_bindings_are_current',
  ],
  {
    cwd: repoRoot,
    env: cargoEnv,
    stdio: 'inherit',
  },
)

if (result.error) {
  throw result.error
}

if (result.status !== 0) {
  console.error('ERROR: src/tauri/generatedBindings.ts is stale. Run: npm run gen:bindings')
  process.exit(result.status ?? 1)
}
