import artifactBundleColorUrl from '../assets/generated-svg/artifact-bundle-color.svg'
import artifactExportSnapshotColorUrl from '../assets/generated-svg/artifact-export-snapshot-color.svg'
import artifactReopenReportColorUrl from '../assets/generated-svg/artifact-reopen-report-color.svg'
import artifactReportWorkbookColorUrl from '../assets/generated-svg/artifact-report-workbook-color.svg'
import artifactRevealPathColorUrl from '../assets/generated-svg/artifact-reveal-path-color.svg'
import compareCandidateFocusColorUrl from '../assets/generated-svg/compare-candidate-focus-color.svg'
import compareCandidatesColorUrl from '../assets/generated-svg/compare-candidates-color.svg'
import comparePinCandidateColorUrl from '../assets/generated-svg/compare-pin-candidate-color.svg'
import compareRemoveFromSetColorUrl from '../assets/generated-svg/compare-remove-from-set-color.svg'
import compareReplacePinnedColorUrl from '../assets/generated-svg/compare-replace-pinned-color.svg'
import emptyArtifactLibraryColorUrl from '../assets/generated-svg/empty-artifact-library-color.svg'
import emptyCompareWorkspaceColorUrl from '../assets/generated-svg/empty-compare-workspace-color.svg'
import emptyResultsWorkspaceColorUrl from '../assets/generated-svg/empty-results-workspace-color.svg'
import modeBuildColorUrl from '../assets/generated-svg/mode-build-color.svg'
import modeOperateColorUrl from '../assets/generated-svg/mode-operate-color.svg'
import modeReviewColorUrl from '../assets/generated-svg/mode-review-color.svg'
import optionsCurrentResultColorUrl from '../assets/generated-svg/options-current-result-color.svg'
import optionsDatasetImportColorUrl from '../assets/generated-svg/options-dataset-import-color.svg'
import optionsEventPathReplayColorUrl from '../assets/generated-svg/options-event-path-replay-color.svg'
import optionsFeasibilityPreviewColorUrl from '../assets/generated-svg/options-feasibility-preview-color.svg'
import optionsFollowupWorkflowColorUrl from '../assets/generated-svg/options-followup-workflow-color.svg'
import optionsProviderDownloadColorUrl from '../assets/generated-svg/options-provider-download-color.svg'
import optionsTupleDetailColorUrl from '../assets/generated-svg/options-tuple-detail-color.svg'
import emptyOptionsWorkspaceColorUrl from '../assets/generated-svg/empty-options-workspace-color.svg'
import provenanceDatasetChangedColorUrl from '../assets/generated-svg/provenance-dataset-changed-color.svg'
import provenanceLowSampleColorUrl from '../assets/generated-svg/provenance-low-sample-color.svg'
import provenanceOosStableColorUrl from '../assets/generated-svg/provenance-oos-stable-color.svg'
import provenanceRegimeConcentratedColorUrl from '../assets/generated-svg/provenance-regime-concentrated-color.svg'
import provenanceReviewAttentionColorUrl from '../assets/generated-svg/provenance-review-attention-color.svg'
import provenanceSlippageSensitiveColorUrl from '../assets/generated-svg/provenance-slippage-sensitive-color.svg'
import provenanceValidationStaleColorUrl from '../assets/generated-svg/provenance-validation-stale-color.svg'
import runtimeReadyColorUrl from '../assets/generated-svg/runtime-ready-color.svg'
import taskCancelledColorUrl from '../assets/generated-svg/task-cancelled-color.svg'
import taskCancellingColorUrl from '../assets/generated-svg/task-cancelling-color.svg'
import taskCompletedColorUrl from '../assets/generated-svg/task-completed-color.svg'
import taskFailedColorUrl from '../assets/generated-svg/task-failed-color.svg'
import taskQueuedColorUrl from '../assets/generated-svg/task-queued-color.svg'
import taskRunningColorUrl from '../assets/generated-svg/task-running-color.svg'

export interface AssetIconSpec {
  kind: 'asset'
  label: string
  src: string
}

function asset(label: string, src: string): AssetIconSpec {
  return { kind: 'asset', label, src }
}

export const comparePinCandidateIcon = asset('Pin candidate', comparePinCandidateColorUrl)
export const compareCandidatesIcon = asset('Compare candidates', compareCandidatesColorUrl)
export const compareReplacePinnedIcon = asset('Replace pinned candidate', compareReplacePinnedColorUrl)
export const compareRemoveFromSetIcon = asset('Remove candidate from compare set', compareRemoveFromSetColorUrl)
export const compareCandidateFocusIcon = asset('Current candidate focus', compareCandidateFocusColorUrl)

export const taskQueuedIcon = asset('Queued task', taskQueuedColorUrl)
export const taskRunningIcon = asset('Running task', taskRunningColorUrl)
export const taskCancellingIcon = asset('Cancelling task', taskCancellingColorUrl)
export const taskCompletedIcon = asset('Completed task', taskCompletedColorUrl)
export const taskFailedIcon = asset('Failed task', taskFailedColorUrl)
export const taskCancelledIcon = asset('Cancelled task', taskCancelledColorUrl)
export const runtimeReadyIcon = asset('Runtime ready', runtimeReadyColorUrl)

export const artifactBundleIcon = asset('Artifact bundle', artifactBundleColorUrl)
export const artifactReportWorkbookIcon = asset('Artifact report workbook', artifactReportWorkbookColorUrl)
export const artifactRevealPathIcon = asset('Reveal artifact path', artifactRevealPathColorUrl)
export const artifactExportSnapshotIcon = asset('Export snapshot', artifactExportSnapshotColorUrl)
export const artifactReopenReportIcon = asset('Reopen report', artifactReopenReportColorUrl)

export const emptyResultsWorkspaceIllustration = emptyResultsWorkspaceColorUrl
export const emptyCompareWorkspaceIllustration = emptyCompareWorkspaceColorUrl
export const emptyArtifactLibraryIllustration = emptyArtifactLibraryColorUrl

export const modeBuildIcon = asset('Build posture', modeBuildColorUrl)
export const modeReviewIcon = asset('Review posture', modeReviewColorUrl)
export const modeOperateIcon = asset('Operate posture', modeOperateColorUrl)

export const optionsFollowupWorkflowIcon = asset('Options follow-up workflow', optionsFollowupWorkflowColorUrl)
export const optionsDatasetImportIcon = asset('Options dataset import', optionsDatasetImportColorUrl)
export const optionsProviderDownloadIcon = asset('Options provider download', optionsProviderDownloadColorUrl)
export const optionsFeasibilityPreviewIcon = asset('Options feasibility preview', optionsFeasibilityPreviewColorUrl)
export const optionsCurrentResultIcon = asset('Options current result', optionsCurrentResultColorUrl)
export const optionsTupleDetailIcon = asset('Options tuple detail', optionsTupleDetailColorUrl)
export const optionsEventPathReplayIcon = asset('Options event path replay', optionsEventPathReplayColorUrl)

export const emptyOptionsWorkspaceIllustration = emptyOptionsWorkspaceColorUrl

export function shellPostureIcon(
  workspace: string | null | undefined,
  workflowStep: string | null | undefined,
): AssetIconSpec {
  const nextWorkspace = String(workspace || '').trim().toLowerCase()
  const nextStep = String(workflowStep || '').trim().toLowerCase()
  if (nextWorkspace === 'workflow') {
    if (nextStep === 'validate' || nextStep === 'analyze') return modeReviewIcon
    return modeBuildIcon
  }
  if (nextWorkspace === 'library' || nextWorkspace === 'aggregate') return modeReviewIcon
  return modeOperateIcon
}

export function shellPostureLabel(
  workspace: string | null | undefined,
  workflowStep: string | null | undefined,
): string {
  const icon = shellPostureIcon(workspace, workflowStep)
  if (icon === modeBuildIcon) return 'Build posture'
  if (icon === modeReviewIcon) return 'Review posture'
  return 'Operate posture'
}

export function provenanceIconForText(text: string | null | undefined): AssetIconSpec | null {
  const normalized = String(text || '').trim().toLowerCase()
  if (!normalized) return null

  if (normalized.includes('out-of-sample') || normalized.includes('oos stable')) {
    return asset('Out-of-sample stable', provenanceOosStableColorUrl)
  }
  if (
    normalized.includes('sample') ||
    normalized.includes('few trades') ||
    normalized.includes('low trades') ||
    normalized.includes('thin history')
  ) {
    return asset('Low sample', provenanceLowSampleColorUrl)
  }
  if (normalized.includes('slippage')) {
    return asset('Slippage sensitive', provenanceSlippageSensitiveColorUrl)
  }
  if (normalized.includes('regime')) {
    return asset('Regime concentrated', provenanceRegimeConcentratedColorUrl)
  }
  if (normalized.includes('validation') || normalized.includes('stale') || normalized.includes('outdated')) {
    return asset('Validation stale', provenanceValidationStaleColorUrl)
  }
  if (normalized.includes('dataset') || normalized.includes('data changed') || normalized.includes('data drift')) {
    return asset('Dataset changed', provenanceDatasetChangedColorUrl)
  }
  if (
    normalized.includes('review') ||
    normalized.includes('attention') ||
    normalized.includes('investigate') ||
    normalized.includes('manual check')
  ) {
    return asset('Review attention', provenanceReviewAttentionColorUrl)
  }

  return null
}
