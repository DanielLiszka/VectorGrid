import type { FixtureFamilyModule } from './types'
import { dataLoadFailureScenario, emptyAppScenario, validationSuccessScenario } from './scenarios'

function asRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' ? (value as Record<string, unknown>) : {}
}

function sampleLoadedDataResult(): Record<string, unknown> {
  const latestHydration = asRecord(validationSuccessScenario.commandResults.load_latest_run_artifact_hydration)
  const hydration = asRecord(latestHydration.hydration)
  const currentRun = asRecord(hydration.currentRun)
  return {
    id: 'fixture_aapl_1m',
    meta: asRecord(currentRun.dataset_meta),
    cache_hit: true,
    cache_path: 'C:/VectorGridData/cache/aapl_1m.vta',
    load_source: 'binary_cache',
  }
}

export const dataLoadSuccessScenario = {
  ...emptyAppScenario,
  name: 'data-load-success',
  family: 'dataset' as const,
  description: 'Sample CSV loading succeeds and projects dataset readiness without running optimization.',
  commandResults: {
    ...emptyAppScenario.commandResults,
    load_price_data: sampleLoadedDataResult(),
  },
  expectedInvokes: ['load_price_data'],
}

export { dataLoadFailureScenario }

export const datasetFixtureFamily: FixtureFamilyModule = {
  family: 'dataset',
  description: 'Dataset loading, sample data, and data-provider browser QA scenarios.',
  scenarios: [dataLoadSuccessScenario, dataLoadFailureScenario],
}
