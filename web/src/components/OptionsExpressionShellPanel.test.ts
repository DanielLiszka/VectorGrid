import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { describe, expect, it, vi } from 'vitest'
import { useWorkflowArtifactsStore } from '../stores/workflowArtifactsStore'
import { createTestPinia } from '../test/pinia'
import OptionsExpressionShellPanel from './OptionsExpressionShellPanel.vue'

vi.mock('./OptionsExpressionPanel.vue', async () => {
  const { defineComponent } = await import('vue')
  return {
    default: defineComponent({
      name: 'OptionsExpressionPanelStub',
      props: {
        parentArtifactPath: { type: String, required: true },
        parentStrategyKind: { type: String, required: true },
        parentDatasetLabel: { type: String, required: true },
        parentCompatible: { type: Boolean, required: false },
      },
      template:
        '<div data-test="options-panel-props" :data-path="parentArtifactPath" :data-strategy="parentStrategyKind" :data-dataset="parentDatasetLabel" :data-compatible="String(parentCompatible)" />',
    }),
  }
})

async function flushUi(): Promise<void> {
  await nextTick()
  await Promise.resolve()
  await nextTick()
}

describe('OptionsExpressionShellPanel', () => {
  it('attaches the current saved run as the Research-hosted options parent', async () => {
    document.body.innerHTML = '<div id="optionsVueShellMount"></div>'
    const pinia = createTestPinia()
    const artifactsStore = useWorkflowArtifactsStore(pinia)
    artifactsStore.currentRun = {
      ts: '2026-05-17T18:00:00Z',
      artifactPath: 'C:/runs/vectorgrid_doublema_latest.json',
      artifactKind: 'optimization_run',
      reportXlsxPath: '',
      csvPath: 'C:/data/SPY.csv',
      datasetMeta: { label: 'SPY 1d', symbol: 'SPY' },
      routeProvenance: null,
      lineage: null,
      mlSignal: null,
      optimizationMetricSurface: null,
      stageCompletions: [],
      tags: [],
      req: { strategy_kind: 'DoubleMa' },
      res: { top: [] },
    }

    const wrapper = mount(OptionsExpressionShellPanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushUi()

    const props = document.querySelector('[data-test="options-panel-props"]')
    expect(props?.getAttribute('data-path')).toBe(
      'C:/runs/vectorgrid_doublema_latest.json',
    )
    expect(props?.getAttribute('data-strategy')).toBe('DoubleMa')
    expect(props?.getAttribute('data-dataset')).toBe('SPY 1d')
    expect(props?.getAttribute('data-compatible')).toBe('true')

    wrapper.unmount()
  })

  it('keeps the panel in empty-state mode when no run artifact is available', async () => {
    document.body.innerHTML = '<div id="optionsVueShellMount"></div>'
    const pinia = createTestPinia()

    const wrapper = mount(OptionsExpressionShellPanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushUi()

    const props = document.querySelector('[data-test="options-panel-props"]')
    expect(props?.getAttribute('data-path')).toBe('')
    expect(props?.getAttribute('data-strategy')).toBe('')
    expect(props?.getAttribute('data-compatible')).toBe('false')

    wrapper.unmount()
  })
})
