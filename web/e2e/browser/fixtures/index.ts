export type {
  FixtureFamilyId,
  FixtureFamilyModule,
  MockTauriScenario,
} from './types'
export { fixtureFamilyIds } from './types'

export { baseCommandResults, savedOptionsArtifactFixturePack } from './base'
export { emptyAppScenario, emptyFixtureFamily, helpTooltipReadyScenario, staleShellStateScenario } from './empty'
export { dataLoadFailureScenario, dataLoadSuccessScenario, datasetFixtureFamily } from './dataset'
export {
  gpuStatusFailureScenario,
  settingsArtifactCatalogFailureScenario,
  settingsFixtureFamily,
  settingsLocalDataInventoryFailureScenario,
  settingsStorageDiagnosticsScenario,
} from './settings'
export {
  validationDatasetMismatchScenario,
  validationFixtureFamily,
  validationRejectedScenario,
  validationReviewScenario,
  validationSuccessScenario,
} from './validation'
export {
  optionsFixtureFamily,
  optionsValidationInputFailureScenario,
  optionsValidationPreviewFailureScenario,
  optionsValidationReadyScenario,
} from './options'
export {
  liveExecutionFixtureFamily,
  liveReadyScenario,
  liveSafetyReadyScenario,
  liveStreamStopReadyScenario,
} from './liveExecution'
export {
  mlSignalBundleStatusFailureScenario,
  mlSignalDatasetExportFailureScenario,
  mlSignalFixtureFamily,
  mlSignalPolicySweepFailureScenario,
  mlSignalScoreCacheFailureScenario,
  mlSignalUnavailableScenario,
  mlSignalValidationFailureScenario,
} from './mlSignal'
export {
  artifactLibraryFixtureFamily,
  savedRunOpenScenario,
  savedRunRepairScenario,
  savedRunRepairWriteScenario,
} from './artifactLibrary'
export {
  candidateDetailReadyScenario,
  compositeRouteSummaryReadyScenario,
  heatmapLayoutScenario,
  optimizationFailureScenario,
  optimizationFixtureFamily,
  optimizationSuccessScenario,
  taskCenterProgressReadyScenario,
} from './optimization'
export {
  aggregateFixtureFamily,
  aggregateMetaReadyScenario,
  aggregateMetaSelectionResponse,
} from './aggregate'
export {
  campaignBatchReadyScenario,
  campaignFixtureFamily,
  campaignRunResponse,
} from './campaign'

import { artifactLibraryFixtureFamily } from './artifactLibrary'
import { aggregateFixtureFamily } from './aggregate'
import { campaignFixtureFamily } from './campaign'
import { datasetFixtureFamily } from './dataset'
import { emptyFixtureFamily } from './empty'
import { liveExecutionFixtureFamily } from './liveExecution'
import { mlSignalFixtureFamily } from './mlSignal'
import { optimizationFixtureFamily } from './optimization'
import { optionsFixtureFamily } from './options'
import { settingsFixtureFamily } from './settings'
import type { FixtureFamilyModule, MockTauriScenario } from './types'
import { validationFixtureFamily } from './validation'

export const fixtureFamilies: FixtureFamilyModule[] = [
  emptyFixtureFamily,
  datasetFixtureFamily,
  optimizationFixtureFamily,
  validationFixtureFamily,
  artifactLibraryFixtureFamily,
  aggregateFixtureFamily,
  optionsFixtureFamily,
  mlSignalFixtureFamily,
  campaignFixtureFamily,
  liveExecutionFixtureFamily,
  settingsFixtureFamily,
]

export const fixtureScenarios: MockTauriScenario[] = fixtureFamilies.flatMap((family) => family.scenarios)
