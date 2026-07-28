import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import test from 'node:test'
import { fileURLToPath } from 'node:url'
import { loadReleaseProofJourneyConfig } from './release_proof_evidence_contract.mjs'

const scriptsDir = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.resolve(scriptsDir, '..')

function readRepoFile(relativePath) {
  return fs.readFileSync(path.resolve(repoRoot, relativePath), 'utf8')
}

const expectedJourneyIds = [
  'clean-start',
  'titlebar-native-controls',
  'window-geometry',
  'window-launch-bounds',
  'native-data-flow',
  'local-paper-cold-restart',
  'options-lifecycle',
  'ml-native-catalog',
  'campaign-lifecycle',
  'cross-ticker',
  'route-matrix',
  'export',
  'recovery',
]

test('release-proof allowlist names every required native journey and emitting spec', () => {
  const config = loadReleaseProofJourneyConfig()
  assert.deepEqual(
    config.journeys.map(({ id }) => id),
    expectedJourneyIds,
  )
  assert.equal(
    new Set(config.journeys.map(({ spec }) => spec)).size,
    expectedJourneyIds.length,
  )

  for (const journey of config.journeys) {
    const specPath = path.resolve(repoRoot, journey.spec)
    const stat = fs.lstatSync(specPath)
    assert.equal(stat.isFile(), true, `${journey.id} spec is not a file`)
    assert.equal(stat.isSymbolicLink(), false, `${journey.id} spec is a symlink`)
    const source = fs.readFileSync(specPath, 'utf8')
    assert.match(
      source,
      new RegExp(
        `writeReleaseProofEvidence\\(\\s*['"]${journey.evidence_file.replaceAll(
          '.',
          '\\.',
        )}['"]`,
      ),
      `${journey.id} does not emit its allowlisted proof filename`,
    )
    assert.match(
      source,
      new RegExp(
        `contract:\\s*['"]${journey.evidence_contract.replaceAll('.', '\\.')}['"]`,
      ),
      `${journey.id} does not emit its allowlisted payload contract`,
    )
  }
})

test('shared desktop specs remain usable outside exact-package release proof', () => {
  const sharedJourneyIds = new Set([
    'clean-start',
    'titlebar-native-controls',
    'window-geometry',
    'window-launch-bounds',
    'native-data-flow',
    'route-matrix',
  ])
  const config = loadReleaseProofJourneyConfig()
  for (const journey of config.journeys.filter(({ id }) => sharedJourneyIds.has(id))) {
    const source = readRepoFile(journey.spec)
    assert.match(
      source,
      /isReleaseProofRun\(\)/,
      `${journey.id} must guard exact-package-only proof emission`,
    )
  }
})

test('WDIO, packaged smoke, and public gate share and revalidate one authority chain', () => {
  const wdio = readRepoFile('web/wdio.desktop.conf.ts')
  const evidenceHelper = readRepoFile(
    'web/e2e/desktop/support/releaseProofEvidence.ts',
  )
  const packagedSmoke = readRepoFile('scripts/run_packaged_smoke.ps1')
  const publicGate = readRepoFile('scripts/run_public_release_gate.ps1')

  assert.match(wdio, /scripts', 'release_proof_journeys\.json'/)
  assert.match(
    wdio,
    /releaseProofJourneyManifest = loadReleaseProofJourneyManifest\(\)/,
  )
  assert.match(evidenceHelper, /vectorgrid-release-proof-journey-v2/)
  assert.match(evidenceHelper, /schemaVersion:\s*2/)
  assert.match(evidenceHelper, /flag:\s*'wx'/)

  assert.match(packagedSmoke, /VECTORGRID_DESKTOP_SMOKE_VARIANT/)
  assert.match(packagedSmoke, /--mode", "write"/)
  assert.match(packagedSmoke, /release_proof_evidence_contract\.mjs/)
  assert.match(packagedSmoke, /source artifact directory already exists/i)
  assert.match(
    packagedSmoke,
    /journeys\)\.Count -ne \[int\]\$releaseProofEvidence\.required_journey_count/,
  )

  assert.match(publicGate, /--mode", "verify"/)
  assert.match(publicGate, /manifest_sha256/)
  assert.match(publicGate, /ReleaseProofEvidenceVerification/)
  assert.match(
    publicGate,
    /required_journey_count -ne \[int\]\$releaseProofEvidence\.required_journey_count/,
  )
})
