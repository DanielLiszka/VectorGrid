import type { FixtureFamilyModule, MockTauriScenario } from './types'
import { emptyAppScenario, gpuStatusFailureScenario } from './scenarios'

const settingsArtifactCatalog = {
  runs: [{ path: 'C:/runs/vectorgrid_run_settings.json' }],
  meta: [{ path: 'C:/runs/vectorgrid_meta_settings.json' }],
  stage5: [],
  policy_evals: [{ path: 'C:/runs/vectorgrid_policy_eval_settings.json' }],
  options_expression: [],
}

const settingsMarketDataInfo = {
  default_root_dir: 'C:/Users/qa/AppData/Local/VectorGrid/data',
  configured_root_dir: 'D:/VectorGridData',
  effective_root_dir: 'D:/VectorGridData',
  polygon_cache_dir: 'D:/VectorGridData/polygon',
}

const settingsPolygonCacheInfo = {
  cache_dir: 'D:/VectorGridData/polygon',
  has_api_key: true,
  credential_status: {
    has_api_key: true,
    state: 'available',
    storage_backend: 'keyring',
    degraded: true,
    message: 'Keyring unlocked through fallback storage.',
  },
  datasets: [
    { symbol: 'AAPL', timeframe: '1m' },
    { symbol: 'MSFT', timeframe: '5m' },
  ],
}

export const settingsStorageDiagnosticsScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: 'settings-storage-diagnostics',
  family: 'settings',
  description:
    'Settings diagnostics report artifact counts, market-data paths, and degraded credential storage.',
  commandResults: {
    ...emptyAppScenario.commandResults,
    get_market_data_dir_info: settingsMarketDataInfo,
    polygon_get_cache_info: settingsPolygonCacheInfo,
    list_artifact_library: settingsArtifactCatalog,
    rebuild_artifact_catalog: settingsArtifactCatalog,
  },
  expectedInvokes: [
    'get_market_data_dir_info',
    'polygon_get_cache_info',
    'list_artifact_library',
    'rebuild_artifact_catalog',
  ],
}

export const settingsArtifactCatalogFailureScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: 'settings-artifact-catalog-failure',
  family: 'settings',
  description:
    'Settings diagnostics surface artifact catalog refresh failures without breaking the shell.',
  commandResults: {
    ...emptyAppScenario.commandResults,
    list_artifact_library: {
      __mockFailure: 'Artifact catalog fixture failure.',
    },
  },
  expectedInvokes: ['list_artifact_library'],
}

export const settingsLocalDataInventoryFailureScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: 'settings-local-data-inventory-failure',
  family: 'settings',
  description:
    'Settings diagnostics surface local data inventory failures without breaking the shell.',
  commandResults: {
    ...emptyAppScenario.commandResults,
    polygon_get_cache_info: {
      __mockFailure: 'Polygon cache inventory fixture failure.',
    },
  },
  expectedInvokes: ['polygon_get_cache_info'],
}

export {
  gpuStatusFailureScenario,
}

export const settingsFixtureFamily: FixtureFamilyModule = {
  family: 'settings',
  description: 'Settings, diagnostics, GPU status, storage, and desktop-runtime browser QA scenarios.',
  scenarios: [
    gpuStatusFailureScenario,
    settingsStorageDiagnosticsScenario,
    settingsArtifactCatalogFailureScenario,
    settingsLocalDataInventoryFailureScenario,
  ],
}
