import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import test from 'node:test'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.resolve(scriptDir, '..')
const packagedSmokePath = path.resolve(scriptDir, 'run_packaged_smoke.ps1')
const publicReleasePath = path.resolve(scriptDir, 'run_public_release_gate.ps1')

function read(relativePath) {
  return fs.readFileSync(path.resolve(repoRoot, relativePath), 'utf8')
}

test('packaged smoke requires schema-2 application-attested profile isolation', () => {
  const packagedSmoke = read('scripts/run_packaged_smoke.ps1')

  assert.match(
    packagedSmoke,
    /\[int\]\$profileIsolationEvidence\.schemaVersion\s+-ne\s+2/,
  )
  assert.match(
    packagedSmoke,
    /-not\s+\[bool\]\$profileIsolationEvidence\.effectiveIsolationProven/,
  )
  assert.match(
    packagedSmoke,
    /\[string\]\$appAttestation\.kind\s+-ne\s+"vectorgrid-qa-profile-attestation"/,
  )
  assert.match(
    packagedSmoke,
    /\[string\]\$appAttestation\.runId\s+-ne\s+\$resolvedRunId/,
  )
  assert.match(
    packagedSmoke,
    /Test-PathEquals\s+-Left\s+\(\[string\]\$appAttestation\.effectiveIdentifier\)\s+-Right\s+\$expectedIsolatedAppData/,
  )
  assert.match(
    packagedSmoke,
    /Test-PathEquals\s+-Left\s+\(\[string\]\$attestedPaths\.webviewDataDir\)\s+-Right\s+\(Join-Path\s+\$expectedIsolatedAppData\s+"webview2"\)/,
  )
  assert.match(
    packagedSmoke,
    /effective_isolation_proven\s*=\s*if\s*\(\$null\s+-ne\s+\$profileIsolationEvidence\.PSObject\.Properties\["effectiveIsolationProven"\]\)/,
  )
  assert.match(packagedSmoke, /app_attestation\s*=\s*if/)
  assert.doesNotMatch(
    packagedSmoke,
    /\[int\]\$profileIsolationEvidence\.schemaVersion\s+-ne\s+1/,
  )
})

test('isolated packaged smoke never reads or copies the real app log', () => {
  const packagedSmoke = read('scripts/run_packaged_smoke.ps1')

  assert.match(
    packagedSmoke,
    /\$appLogPath\s*=\s*if\s*\(\$requiresIsolatedProfile\)\s*\{\s*[\s\S]*?\$null\s*\}\s*else\s*\{\s*Resolve-AppLogPath\s*\}/,
  )
  assert.match(
    packagedSmoke,
    /if\s*\(\s*-not\s+\$requiresIsolatedProfile\s+-and\s+-not\s+\$StaticInspectionOnly[\s\S]*?Copy-Item\s+-LiteralPath\s+\$appLogPath/,
  )
})

test('application and WDIO harness share the marker and attestation handshake', () => {
  const rustProfile = read('src-tauri/src/qa_profile.rs')
  const harness = read('web/e2e/desktop/support/profileIsolation.ts')
  const wdioConfig = read('web/wdio.desktop.conf.ts')
  const desktopHarness = `${harness}\n${wdioConfig}`

  for (const value of [
    'VECTORGRID_DESKTOP_SMOKE_APP_DATA_OVERRIDE',
    'VECTORGRID_DESKTOP_SMOKE_ISOLATED_PROFILE_ROOT',
    'VECTORGRID_DESKTOP_SMOKE_APP_DATA_DIR',
    'VECTORGRID_DESKTOP_SMOKE_RUN_ID',
    '.vectorgrid-desktop-smoke-profile.json',
    '.vectorgrid-qa-profile-attestation.json',
    'vectorgrid-qa-profile-attestation',
  ]) {
    assert.ok(rustProfile.includes(value), `Rust profile contract is missing ${value}`)
    assert.ok(desktopHarness.includes(value), `WDIO profile contract is missing ${value}`)
  }
  assert.match(rustProfile, /PROFILE_SCHEMA_VERSION:\s*u32\s*=\s*2/)
  assert.match(harness, /const PROFILE_SCHEMA_VERSION = 2/)
  assert.match(harness, /assertOwnedTreeHasNoReparsePoints\(root\)/)
  assert.match(harness, /export function activateDesktopProfileEnvironment/)
  const buildIndex = wdioConfig.indexOf('buildDesktopSmokeApp()')
  const activationIndex = wdioConfig.indexOf(
    'activateDesktopProfileEnvironment(desktopProfileIsolation)',
  )
  const driverIndex = wdioConfig.indexOf('await startTauriDriver()', activationIndex)
  assert.ok(buildIndex >= 0 && buildIndex < activationIndex)
  assert.ok(activationIndex < driverIndex)
})

test('public release gate consumes the promoted schema-2 attestation contract', () => {
  const publicGate = read('scripts/run_public_release_gate.ps1')

  assert.match(publicGate, /\[int\]\$profileIsolation\.schema_version\s+-ne\s+2/)
  assert.match(
    publicGate,
    /-not\s+\[bool\]\$profileIsolation\.effective_isolation_proven/,
  )
  assert.match(
    publicGate,
    /\[string\]\$appAttestation\.kind\s+-ne\s+"vectorgrid-qa-profile-attestation"/,
  )
  assert.match(
    publicGate,
    /Test-PathEquals\s+-Left\s+\(\[string\]\$appAttestation\.effective_identifier\)\s+-Right\s+\$expectedIsolatedAppData/,
  )
  assert.match(publicGate, /effective_profile_isolation_proven\s*=/)
  assert.match(publicGate, /profile_app_attestation\s*=/)
})

test('release profile-isolation PowerShell remains parser-clean', () => {
  const source = String.raw`
$tokens = $null
$errors = $null
[System.Management.Automation.Language.Parser]::ParseFile(
  $env:VECTORGRID_SCRIPT_UNDER_TEST,
  [ref]$tokens,
  [ref]$errors
) | Out-Null
if ($errors.Count -gt 0) {
  $errors | ForEach-Object { [Console]::Error.WriteLine($_.Message) }
  exit 1
}
`
  const encoded = Buffer.from(source, 'utf16le').toString('base64')
  for (const scriptPath of [packagedSmokePath, publicReleasePath]) {
    const result = spawnSync(
      'powershell.exe',
      ['-NoProfile', '-NonInteractive', '-EncodedCommand', encoded],
      {
        cwd: repoRoot,
        encoding: 'utf8',
        env: {
          ...process.env,
          VECTORGRID_SCRIPT_UNDER_TEST: scriptPath,
        },
        windowsHide: true,
      },
    )
    assert.equal(
      result.status,
      0,
      `${path.basename(scriptPath)} parser errors:\n${String(result.stderr || result.stdout).trim()}`,
    )
  }
})
