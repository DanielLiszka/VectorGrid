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

function normalizeTags(value: unknown): string[] {
  if (!Array.isArray(value)) return []
  const tags: string[] = []
  for (const entry of value) {
    const tag = normalizeText(entry)
    if (!tag) continue
    tags.push(tag)
  }
  return tags
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

    const artifactRaw = (await invokeCommand('load_run_artifact', { path: latestPath })) as unknown
    const artifact = asRecord(artifactRaw)
    if (!artifact) return false

    const validation = asRecord(artifact.validation)
    const payload: Record<string, unknown> = {
      currentRun: {
        ts: normalizeText(artifact.ts ?? latest?.ts),
        artifact_path: latestPath,
        report_xlsx_path: normalizeText(artifact.report_xlsx_path ?? latest?.report_xlsx_path),
        csv_path: normalizeText(artifact.csv_path ?? latest?.csv_path),
        dataset_meta: asRecord(artifact.dataset_meta),
        tags: normalizeTags(artifact.tags ?? latest?.tags),
        req: artifact.req ?? null,
        res: artifact.res ?? null,
      },
      currentValidation: validation
        ? {
          ts: normalizeText(artifact.ts ?? latest?.ts),
          req: validation.req ?? null,
          res: validation.res ?? null,
        }
        : null,
    }

    const applied = applyWorkflowArtifactsPayload(payload, source)
    const meaningfullyApplied = applied.runChanged || applied.validationChanged
    if (meaningfullyApplied) refreshAnalysisFromWorkflowRun()
    return meaningfullyApplied
  } catch {
    return false
  }
}
