// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import RunFooterPanel from './RunFooterPanel.vue'
import { useCommandActionStore } from '../stores/commandActionStore'
import { useDataStore } from '../stores/dataStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useValidationStore } from '../stores/validationStore'
import { useWorkflowArtifactsStore } from '../stores/workflowArtifactsStore'
import { useWorkflowStore } from '../stores/workflowStore'

function installFooterMountShell(): void {
  document.body.innerHTML = `
    <footer id="runFooterShell">
      <div id="runFooterVueShellMount"></div>
    </footer>
  `
}

async function flushPanel(): Promise<void> {
  await nextTick()
  await nextTick()
}

describe('RunFooterPanel', () => {
  beforeEach(() => {
    installFooterMountShell()
    localStorage.clear()
  })

  it('uses Run optimization as the primary action on the optimize workflow step', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    const actions = useCommandActionStore(pinia)
    workflow.selectWorkflowStep('optimize', { persist: false })
    optimize.runDisabled = false
    const runSpy = vi.spyOn(actions, 'runOptimization').mockImplementation(() => {})

    const wrapper = mount(RunFooterPanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const primary = Array.from(document.querySelectorAll('button')).find((button) =>
      button.textContent?.trim() === 'Run optimization',
    ) as HTMLButtonElement | undefined
    expect(primary).toBeDefined()
    primary?.click()
    expect(runSpy).toHaveBeenCalledTimes(1)

    wrapper.unmount()
  })

  it('switches the primary action to validation on the validation workflow step', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const dataStore = useDataStore(pinia)
    const validation = useValidationStore(pinia)
    const artifacts = useWorkflowArtifactsStore(pinia)
    const actions = useCommandActionStore(pinia)
    workflow.selectWorkflowStep('validate', { persist: false })
    dataStore.dataId = 'AAPL_1D'
    dataStore.datasetLabel = 'AAPL'
    artifacts.applyVueRuntimePayload({
      currentRun: {
        ts: '2026-03-09T15:00:00Z',
        req: {},
        res: { top: [] },
      },
    })
    validation.validateDisabled = false
    validation.validationHint = 'Validation ready.'
    const validateSpy = vi.spyOn(actions, 'validateOptimization').mockImplementation(() => {})

    const wrapper = mount(RunFooterPanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const primary = Array.from(document.querySelectorAll('button')).find((button) =>
      button.textContent?.trim() === 'Run validation',
    ) as HTMLButtonElement | undefined
    expect(primary).toBeDefined()
    expect(primary?.disabled).toBe(false)
    primary?.click()
    expect(validateSpy).toHaveBeenCalledTimes(1)
    expect(document.body.textContent || '').toContain('Validation readiness')

    wrapper.unmount()
  })

  it('disables validation when the validation step has no dataset or optimization results yet', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    workflow.selectWorkflowStep('validate', { persist: false })

    const wrapper = mount(RunFooterPanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const primary = Array.from(document.querySelectorAll('button')).find((button) =>
      button.textContent?.trim() === 'Run validation',
    ) as HTMLButtonElement | undefined
    expect(primary).toBeDefined()
    expect(primary?.disabled).toBe(true)

    wrapper.unmount()
  })
})
