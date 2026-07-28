import assert from 'node:assert/strict'
import crypto from 'node:crypto'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import test from 'node:test'
import {
  loadReleaseProofJourneyConfig,
  verifyReleaseProofEvidenceManifest,
  writeReleaseProofEvidenceManifest,
} from './release_proof_evidence_contract.mjs'

function sha256(filePath) {
  return crypto.createHash('sha256').update(fs.readFileSync(filePath)).digest('hex')
}

function fixture() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'vectorgrid-release-proof-contract-'))
  const evidenceRoot = path.join(root, 'evidence')
  fs.mkdirSync(evidenceRoot)
  const binaryPath = path.join(root, 'ta_desktop_demo_app.exe')
  fs.writeFileSync(binaryPath, Buffer.from('exact-packaged-binary-fixture'))
  const config = loadReleaseProofJourneyConfig()
  const runId = 'release-proof-contract-fixture'
  const variant = 'cpu'
  const binaryIdentity = {
    path: binaryPath,
    sizeBytes: fs.statSync(binaryPath).size,
    sha256: sha256(binaryPath),
  }
  for (const journey of config.journeys) {
    fs.writeFileSync(
      path.join(evidenceRoot, journey.evidence_file),
      `${JSON.stringify({
        schemaVersion: 2,
        contract: 'vectorgrid-release-proof-journey-v2',
        journeyId: journey.id,
        status: 'passed',
        generatedAt: '2026-07-28T12:00:00.000Z',
        runId,
        releaseVariant: variant,
        binaryIdentity,
        evidence: {
          contract: journey.evidence_contract,
          fixture: true,
        },
      }, null, 2)}\n`,
      'utf8',
    )
  }
  const options = {
    evidenceRoot,
    runId,
    variant,
    binaryPath,
    binarySizeBytes: binaryIdentity.sizeBytes,
    binarySha256: binaryIdentity.sha256,
    manifestPath: path.join(evidenceRoot, 'release-proof-evidence-manifest.json'),
  }
  return {
    root,
    evidenceRoot,
    config,
    options,
  }
}

function withFixture(callback) {
  const value = fixture()
  try {
    return callback(value)
  } finally {
    fs.rmSync(value.root, { recursive: true, force: true })
  }
}

test('write and verify bind every allowlisted journey to one run and packaged binary', () => {
  withFixture(({ config, options }) => {
    const written = writeReleaseProofEvidenceManifest(options)
    assert.equal(written.manifest.status, 'passed')
    assert.equal(written.manifest.required_journey_count, config.journeys.length)
    assert.deepEqual(
      written.manifest.journeys.map((journey) => journey.id),
      config.journeys.map((journey) => journey.id),
    )
    assert.match(written.manifest_sha256, /^[a-f0-9]{64}$/)

    const verified = verifyReleaseProofEvidenceManifest(options)
    assert.equal(verified.manifest_sha256, written.manifest_sha256)
  })
})

test('missing, failed, stale-run, foreign-binary, and empty journey evidence fails closed', () => {
  for (const mutation of [
    'missing',
    'failed',
    'stale-run',
    'foreign-binary',
    'missing-evidence',
    'foreign-evidence-contract',
    'contract-only-evidence',
  ]) {
    withFixture(({ config, evidenceRoot, options }) => {
      const journeyPath = path.join(evidenceRoot, config.journeys[0].evidence_file)
      if (mutation === 'missing') {
        fs.rmSync(journeyPath)
      } else {
        const value = JSON.parse(fs.readFileSync(journeyPath, 'utf8'))
        if (mutation === 'failed') value.status = 'failed'
        if (mutation === 'stale-run') value.runId = 'foreign-run'
        if (mutation === 'foreign-binary') value.binaryIdentity.sha256 = '0'.repeat(64)
        if (mutation === 'missing-evidence') delete value.evidence
        if (mutation === 'foreign-evidence-contract') {
          value.evidence.contract = 'foreign-evidence-contract-v1'
        }
        if (mutation === 'contract-only-evidence') {
          value.evidence = { contract: value.evidence.contract }
        }
        fs.writeFileSync(journeyPath, `${JSON.stringify(value, null, 2)}\n`)
      }
      assert.throws(
        () => writeReleaseProofEvidenceManifest(options),
        /missing|stale|failed|foreign|payload/i,
        mutation,
      )
    })
  }
})

test('non-allowlisted proof entries and duplicate manifest creation are rejected', () => {
  withFixture(({ evidenceRoot, options }) => {
    fs.writeFileSync(path.join(evidenceRoot, 'invented-proof.json'), '{}\n')
    assert.throws(
      () => writeReleaseProofEvidenceManifest(options),
      /non-allowlisted proof files/,
    )
    fs.rmSync(path.join(evidenceRoot, 'invented-proof.json'))
    writeReleaseProofEvidenceManifest(options)
    assert.throws(
      () => writeReleaseProofEvidenceManifest(options),
      /Refusing to overwrite/,
    )
  })

  withFixture(({ evidenceRoot, options }) => {
    fs.mkdirSync(path.join(evidenceRoot, 'invented-proof.json'))
    assert.throws(
      () => writeReleaseProofEvidenceManifest(options),
      /non-allowlisted proof files/,
    )
  })

  withFixture(({ evidenceRoot, options }) => {
    assert.throws(
      () =>
        writeReleaseProofEvidenceManifest({
          ...options,
          manifestPath: path.join(evidenceRoot, 'alternate-manifest.json'),
        }),
      /canonical filename/,
    )
  })
})

test('public revalidation detects proof or manifest mutation after packaged smoke', () => {
  withFixture(({ config, evidenceRoot, options }) => {
    writeReleaseProofEvidenceManifest(options)
    const journeyPath = path.join(evidenceRoot, config.journeys[1].evidence_file)
    const value = JSON.parse(fs.readFileSync(journeyPath, 'utf8'))
    value.evidence.mutated = true
    fs.writeFileSync(journeyPath, `${JSON.stringify(value, null, 2)}\n`)
    assert.throws(
      () => verifyReleaseProofEvidenceManifest(options),
      /no longer matches/,
    )
  })

  withFixture(({ options }) => {
    writeReleaseProofEvidenceManifest(options)
    const manifest = JSON.parse(fs.readFileSync(options.manifestPath, 'utf8'))
    manifest.required_journey_count = 0
    fs.writeFileSync(options.manifestPath, `${JSON.stringify(manifest, null, 2)}\n`)
    assert.throws(
      () => verifyReleaseProofEvidenceManifest(options),
      /no longer matches/,
    )
  })
})
