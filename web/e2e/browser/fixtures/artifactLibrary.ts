import type { FixtureFamilyModule, MockTauriScenario } from './types'
import { optionsValidationReadyScenario, savedOptionsArtifactFixturePack } from './scenarios'

export { savedOptionsArtifactFixturePack }

export const savedRunRepairScenario: MockTauriScenario = {
  ...optionsValidationReadyScenario,
  name: 'saved-run-repair-ready',
  family: 'artifactLibrary',
  description: 'A saved DoubleMa run can run a dry-run artifact repair check.',
  expectedInvokes: [
    'list_run_artifacts',
    'load_run_artifact_detail',
    'repair_run_artifact',
  ],
}

export const savedRunRepairWriteScenario: MockTauriScenario = {
  ...optionsValidationReadyScenario,
  name: 'saved-run-repair-write-ready',
  family: 'artifactLibrary',
  description: 'A saved DoubleMa run can write a repaired artifact copy.',
  commandResults: {
    ...optionsValidationReadyScenario.commandResults,
    list_run_artifacts: savedOptionsArtifactFixturePack.repairedRunSummaries,
    repair_run_artifact: savedOptionsArtifactFixturePack.repairWrite,
  },
  expectedInvokes: [
    'list_run_artifacts',
    'load_run_artifact_detail',
    'repair_run_artifact',
  ],
}

export const savedRunOpenScenario: MockTauriScenario = {
  ...optionsValidationReadyScenario,
  name: 'saved-run-open-ready',
  family: 'artifactLibrary',
  description:
    'A saved DoubleMa run can be loaded, applied, and queued for rerun from the Library panel.',
  commandResults: {
    ...optionsValidationReadyScenario.commandResults,
    load_run_artifact_hydration: {
      currentRun: {
        ts: savedOptionsArtifactFixturePack.parentRunDetail.ts,
        artifact_path: 'C:/runs/vectorgrid_options_parent.json',
        csv_path: 'C:/fixtures/aapl_1m.csv',
        dataset_meta:
          savedOptionsArtifactFixturePack.parentRunDetail.datasetMeta,
        req: savedOptionsArtifactFixturePack.parentRunDetail.runReq,
        res: savedOptionsArtifactFixturePack.parentRunDetail.runRes,
        tags: ['qa', 'options'],
      },
      currentValidation: null,
    },
    load_price_data: {
      id: 'fixture_aapl_1m',
      meta: {
        csv_path: 'C:/fixtures/aapl_1m.csv',
        timeframe: '1m',
        len: 5000,
      },
      cache_hit: true,
      cache_path: 'C:/VectorGridData/cache/aapl_1m.vta',
      load_source: 'binary_cache',
    },
    run_double_ma_optimization:
      savedOptionsArtifactFixturePack.parentRunDetail.runRes,
    save_run_artifact: {
      path: 'C:/runs/vectorgrid_options_parent_rerun.json',
      bytes: 65536,
    },
  },
  expectedInvokes: [
    'load_run_artifact_hydration',
    'load_run_artifact_detail',
    'load_price_data',
    'run_double_ma_optimization',
    'save_run_artifact',
  ],
}

export const artifactLibraryFixtureFamily: FixtureFamilyModule = {
  family: 'artifactLibrary',
  description: 'Saved run library, artifact hydration, repair, and reopen browser QA scenarios.',
  scenarios: [
    savedRunRepairScenario,
    savedRunRepairWriteScenario,
    savedRunOpenScenario,
  ],
}
