import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it } from 'vitest'
import FinalReviewWorkspacePanel from './FinalReviewWorkspacePanel.vue'
import { useAnalysisStore } from '../stores/analysisStore'
import { useDataStore } from '../stores/dataStore'
import { useValidationStore } from '../stores/validationStore'
import { useWorkflowArtifactsStore } from '../stores/workflowArtifactsStore'

function installFinalReviewPanelMountShell(): void {
  document.body.innerHTML = `
    <section data-vue-panel="final"></section>
    <div id="finalReviewVueShellMount"></div>
  `
}

async function flushPanel(): Promise<void> {
  await nextTick()
  await nextTick()
}

describe('FinalReviewWorkspacePanel', () => {
  beforeEach(() => {
    installFinalReviewPanelMountShell()
    localStorage.clear()
  })

  it('uses singular and plural agreement for evidence awaiting review', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const validationStore = useValidationStore(pinia)
    const workflowArtifactsStore = useWorkflowArtifactsStore(pinia)

    mount(FinalReviewWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    expect(document.querySelector('.finalReviewVerdict')?.textContent).toContain(
      '4 evidence items still need review.',
    )

    dataStore.dataId = 'dataset_1'
    analysisStore.bestTitle = 'Rank #1 · Winner'
    validationStore.tableRowCount = 1
    await flushPanel()

    expect(document.querySelector('.finalReviewVerdict')?.textContent).toContain(
      '1 evidence item still needs review.',
    )

    const artifactPath = 'C:/runs/vectorgrid_run_2026-07-24.json'
    workflowArtifactsStore.applyLegacyRuntimePayload({
      currentRun: {
        ts: '2026-07-24T12:00:00Z',
        artifact_path: artifactPath,
        req: {},
        res: {},
      },
    })
    await flushPanel()

    const artifactEvidence = document.querySelector(
      '.finalReviewEvidence .compactEvidenceText',
    )
    expect(artifactEvidence?.textContent).toBe('vectorgrid_run_2026-07-24.json')
    expect(artifactEvidence?.getAttribute('title')).toBe(artifactPath)
    expect(artifactEvidence?.getAttribute('aria-label')).toBe(
      `Run artifact path: ${artifactPath}`,
    )
  })
})
