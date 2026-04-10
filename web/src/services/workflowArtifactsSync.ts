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
  const out: string[] = []
  for (const item of value) {
    const next = normalizeText(item)
    if (!next) continue
    out.push(next)
  }
  return out
}

function sameStringArray(left: string[], right: string[]): boolean {
  if (left.length !== right.length) return false
  for (let index = 0; index < left.length; index += 1) {
    if (left[index] !== right[index]) return false
  }
  return true
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

function latestSummaryMatchesCurrentStore(latest: Record<string, unknown>): boolean {
  const pinia = resolveRuntimePinia()
  if (!pinia) return true
  const currentRun = useWorkflowArtifactsStore(pinia).currentRun
  if (!currentRun) return true
  return (
    normalizeText(latest.report_xlsx_path ?? latest.reportXlsxPath) ===
      normalizeText(currentRun.reportXlsxPath) &&
    normalizeText(latest.csv_path ?? latest.csvPath) ===
      normalizeText(currentRun.csvPath) &&
    sameStringArray(normalizeTags(latest.tags), normalizeTags(currentRun.tags))
  )
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
    if (!latest || !latestPath) return false

    const currentIdentity = readLatestWorkflowRunIdentitySnapshot()
    const canSkipLoad = !!(
      currentIdentity &&
      currentIdentity.artifactPath === latestPath &&
      currentIdentity.ts === latestTs &&
      currentIdentity.hasValidation &&
      latestSummaryMatchesCurrentStore(latest)
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
