import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import test from 'node:test'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.resolve(scriptDir, '..')

const VERSION = '5.1.0'
const LICENSE_PATH =
  'src-tauri/resources/licenses/TradingView-Lightweight-Charts-LICENSE.txt'
const NOTICE_PATH =
  'src-tauri/resources/licenses/TradingView-Lightweight-Charts-NOTICE.txt'
const LICENSE_BUNDLE_TARGET =
  'licenses/TradingView-Lightweight-Charts-LICENSE.txt'
const NOTICE_BUNDLE_TARGET =
  'licenses/TradingView-Lightweight-Charts-NOTICE.txt'
const EXPECTED_LICENSE_SHA256 =
  '70c9d5382506dd184465425c08a99ad9bd6d9ac1313c252968ba0b585e5ef823'
const EXPECTED_NOTICE_SHA256 =
  'eccf1c464e60ff87453aa11e80d15649d6e0c3315d7f11f660f5e1c91378a375'
const EXPECTED_NOTICE =
  'TradingView Lightweight Charts™ Copyright (\u0441) 2025 TradingView, Inc. https://www.tradingview.com/\n'

function read(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), 'utf8')
}

function readJson(relativePath) {
  return JSON.parse(read(relativePath))
}

function sha256(relativePath) {
  return createHash('sha256')
    .update(fs.readFileSync(path.join(repoRoot, relativePath)))
    .digest('hex')
}

test('pins Lightweight Charts 5.1.0 and its Apache-2.0 package identity', () => {
  const packageJson = readJson('web/package.json')
  const packageLock = readJson('web/package-lock.json')
  const lockedPackage = packageLock.packages?.['node_modules/lightweight-charts']

  assert.equal(packageJson.dependencies?.['lightweight-charts'], VERSION)
  assert.equal(packageLock.packages?.['']?.dependencies?.['lightweight-charts'], VERSION)
  assert.equal(lockedPackage?.version, VERSION)
  assert.equal(lockedPackage?.license, 'Apache-2.0')
  assert.equal(
    lockedPackage?.resolved,
    `https://registry.npmjs.org/lightweight-charts/-/lightweight-charts-${VERSION}.tgz`,
  )
})

test('tracks the exact reviewed upstream notice and package license', () => {
  assert.equal(read(NOTICE_PATH), EXPECTED_NOTICE)
  assert.equal(sha256(LICENSE_PATH), EXPECTED_LICENSE_SHA256)
  assert.equal(sha256(NOTICE_PATH), EXPECTED_NOTICE_SHA256)

  const license = read(LICENSE_PATH)
  assert.match(license, /Apache License\s+Version 2\.0, January 2004/)
  assert.match(license, /Copyright 2023 TradingView, Inc\./)
})

test('maps both legal artifacts in canonical and mirrored Windows bundles', () => {
  const canonicalResources =
    readJson('src-tauri/tauri.windows.conf.json').bundle?.resources ?? {}
  const mirrorResources = readJson('tauri.windows.conf.json').bundle?.resources ?? {}

  assert.equal(
    canonicalResources[
      'resources/licenses/TradingView-Lightweight-Charts-LICENSE.txt'
    ],
    LICENSE_BUNDLE_TARGET,
  )
  assert.equal(
    canonicalResources[
      'resources/licenses/TradingView-Lightweight-Charts-NOTICE.txt'
    ],
    NOTICE_BUNDLE_TARGET,
  )
  assert.equal(
    mirrorResources[
      'src-tauri/resources/licenses/TradingView-Lightweight-Charts-LICENSE.txt'
    ],
    LICENSE_BUNDLE_TARGET,
  )
  assert.equal(
    mirrorResources[
      'src-tauri/resources/licenses/TradingView-Lightweight-Charts-NOTICE.txt'
    ],
    NOTICE_BUNDLE_TARGET,
  )
})

test('release manifest and packaged smoke retain and hash both artifacts', () => {
  const releaseManifest = read('scripts/generate_release_manifest.ps1')
  const packagedSmoke = read('scripts/run_packaged_smoke.ps1')

  for (const expected of [
    'TradingView-Lightweight-Charts-LICENSE.txt',
    'TradingView-Lightweight-Charts-NOTICE.txt',
    'lightweight_charts_license_sha256',
    'lightweight_charts_license_expected_sha256',
    'lightweight_charts_notice_sha256',
    'lightweight_charts_notice_expected_sha256',
  ]) {
    assert.match(releaseManifest, new RegExp(expected))
    assert.match(packagedSmoke, new RegExp(expected))
  }

  for (const expectedSha256 of [EXPECTED_LICENSE_SHA256, EXPECTED_NOTICE_SHA256]) {
    assert.match(releaseManifest, new RegExp(expectedSha256))
  }

  for (const bundleTarget of [LICENSE_BUNDLE_TARGET, NOTICE_BUNDLE_TARGET]) {
    assert.match(packagedSmoke, new RegExp(bundleTarget.replaceAll('.', '\\.')))
  }
})
