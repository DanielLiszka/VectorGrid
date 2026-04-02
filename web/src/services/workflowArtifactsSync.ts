import { invokeCommand } from '../tauri/invoke'
import {
  applyWorkflowArtifactsPayload,
  readLatestWorkflowRunIdentitySnapshot,
  useWorkflowArtifactsStore,
} from '../stores/workflowArtifactsStore'
import { resolveRuntimePinia } from '../stores/runtimePinia'
import { refreshAnalysisFromWorkflowRun } from './analysisService'

type WorkflowArtifactsSource = 'legacy-runtime' | 'vue'

function normalizeText(value: unknown): string {
  return String(value ?? '').trim()
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  return value as Record<string, unknown>
}

function syncSourceIfStoreBound(source: WorkflowArtifactsSource): void {
  const pinia = resolveRuntimePinia()
  if (!pinia) return
  useWorkflowArtifactsStore(pinia).source = source
}

export async function syncWorkflowArtifactsFromLegacyRuntime(
  source: WorkflowArtifactsSource = 'vue',
): Promise<boolean> {
  try {
    const rowsRaw = (await invokeCommand('list_run_artifacts')) as unknown
    const rows = Array.isArray(rowsRaw) ? rowsRaw : []
    if (rows.length === 0) return false

    const latest = asRecord(rows[0])
    const latestPath = normalizeText(latest?.path)
    const latestTs = normalizeText(latest?.ts)
    if (!latestPath) return false

    const currentIdentity = readLatestWorkflowRunIdentitySnapshot()
    const canSkipLoad = !!(
      currentIdentity &&
      currentIdentity.artifactPath === latestPath &&
      currentIdentity.ts === latestTs &&
      currentIdentity.hasValidation
    )
    if (canSkipLoad) {
      syncSourceIfStoreBound(source)
      return false
    }

    const payloadRaw = (await invokeCommand('load_run_artifact_hydration', {
      path: latestPath,
    })) as unknown
    const payload = asRecord(payloadRaw)
    if (!payload) return false

    const applied = applyWorkflowArtifactsPayload(payload, source)
    const meaningfullyApplied = applied.runChanged || applied.validationChanged
    if (meaningfullyApplied) refreshAnalysisFromWorkflowRun()
    return meaningfullyApplied
  } catch {
    return false
  }
}
