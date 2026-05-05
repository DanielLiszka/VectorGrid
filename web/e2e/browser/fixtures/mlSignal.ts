import type { FixtureFamilyModule, MockTauriScenario } from './types'
import { emptyAppScenario, validationSuccessScenario } from './scenarios'

function sampleLoadedDataResult(): Record<string, unknown> {
  const hydration = validationSuccessScenario.commandResults
    .load_latest_run_artifact_hydration as Record<string, unknown>
  const currentRun = ((hydration.hydration as Record<string, unknown>)
    .currentRun ?? {}) as Record<string, unknown>
  return {
    id: 'fixture_aapl_1m',
    meta: currentRun.dataset_meta ?? {},
    cache_hit: true,
    cache_path: 'C:/VectorGridData/cache/aapl_1m.vta',
    load_source: 'binary_cache',
  }
}

const qaBundleSummary = {
  bundle_id: 'qa-bundle',
  manifest_path: 'C:/VectorGridData/ml/qa-bundle/manifest.json',
  state: 'available',
  model_family: 'catboost',
  issues: [],
}

const loadedDatasetCommandResults = {
  ...emptyAppScenario.commandResults,
  load_price_data: sampleLoadedDataResult(),
}

export const mlSignalUnavailableScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: 'ml-signal-unavailable',
  family: 'mlSignal',
  description:
    'ML Signal mode is available but no model bundles are installed.',
  commandResults: {
    ...emptyAppScenario.commandResults,
    list_ml_signal_model_bundles: [],
    ml_signal_model_status: {
      status: {
        state: 'missing',
        issues: ['No ML signal bundle selected in browser QA fixture.'],
      },
    },
  },
  expectedInvokes: ['list_ml_signal_model_bundles'],
}

export const mlSignalBundleStatusFailureScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: 'ml-signal-bundle-status-failure',
  family: 'mlSignal',
  description:
    'ML Signal mode surfaces model bundle status failures after a bundle is selected.',
  commandResults: {
    ...emptyAppScenario.commandResults,
    list_ml_signal_model_bundles: [qaBundleSummary],
    ml_signal_model_status: {
      __mockFailure: 'Bundle status fixture failure.',
    },
  },
  expectedInvokes: ['list_ml_signal_model_bundles', 'ml_signal_model_status'],
}

export const mlSignalDatasetExportFailureScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: 'ml-signal-dataset-export-failure',
  family: 'mlSignal',
  description:
    'ML Signal dataset export failure remains visible and recoverable.',
  commandResults: {
    ...loadedDatasetCommandResults,
    export_ml_signal_dataset: {
      __mockFailure: 'ML dataset export fixture failure.',
    },
  },
  expectedInvokes: ['load_price_data', 'export_ml_signal_dataset'],
}

export const mlSignalScoreCacheFailureScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: 'ml-signal-score-cache-failure',
  family: 'mlSignal',
  description:
    'ML Signal score-cache generation failure remains visible and recoverable.',
  commandResults: {
    ...loadedDatasetCommandResults,
    generate_ml_signal_score_cache: {
      __mockFailure: 'ML score cache fixture failure.',
    },
  },
  expectedInvokes: ['load_price_data', 'generate_ml_signal_score_cache'],
}

export const mlSignalValidationFailureScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: 'ml-signal-validation-failure',
  family: 'mlSignal',
  description:
    'ML Signal validation failure remains visible and recoverable.',
  commandResults: {
    ...loadedDatasetCommandResults,
    run_ml_signal_validation: {
      __mockFailure: 'ML validation fixture failure.',
    },
  },
  expectedInvokes: ['load_price_data', 'run_ml_signal_validation'],
}

export const mlSignalPolicySweepFailureScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: 'ml-signal-policy-sweep-failure',
  family: 'mlSignal',
  description:
    'ML Signal policy-sweep failure remains visible and recoverable.',
  commandResults: {
    ...loadedDatasetCommandResults,
    run_ml_signal_policy_sweep: {
      __mockFailure: 'ML policy sweep fixture failure.',
    },
  },
  expectedInvokes: ['load_price_data', 'run_ml_signal_policy_sweep'],
}

export const mlSignalFixtureFamily: FixtureFamilyModule = {
  family: 'mlSignal',
  description: 'ML Signal model availability, failure, and validation browser QA scenarios.',
  scenarios: [
    mlSignalUnavailableScenario,
    mlSignalBundleStatusFailureScenario,
    mlSignalDatasetExportFailureScenario,
    mlSignalScoreCacheFailureScenario,
    mlSignalValidationFailureScenario,
    mlSignalPolicySweepFailureScenario,
  ],
}
