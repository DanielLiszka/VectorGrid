// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it } from 'vitest'
import ValidationWorkspacePanel from './ValidationWorkspacePanel.vue'
import { useAnalysisStore } from '../stores/analysisStore'
import { useDataStore } from '../stores/dataStore'
import { bindRuntimePinia } from '../stores/runtimePinia'
import { useValidationStore } from '../stores/validationStore'
import { useWorkflowArtifactsStore } from '../stores/workflowArtifactsStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { VALIDATION_RESULTS_TABLE_STATE_KEY } from '../utils/core/storageKeys'

function installValidationPanelMountShell(): void {
  document.body.innerHTML = `
    <section data-vue-panel="validate"></section>
    <div id="validationVueShellMount"></div>
  `
}

async function flushPanel(): Promise<void> {
  await nextTick()
  await nextTick()
}

describe('ValidationWorkspacePanel', () => {
  beforeEach(() => {
    installValidationPanelMountShell()
    localStorage.clear()
  })

  it('keeps the primary validation action inline in the panel', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    workflow.selectResultsTab('validate', { persist: false })

    const wrapper = mount(ValidationWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    expect(document.body.textContent || '').toContain('Run validation')
    expect(document.querySelector('#vueValAdmissionPolicy')).not.toBeNull()
    const runButtons = Array.from(document.querySelectorAll('button')).filter(
      (button) => button.textContent?.trim() === 'Run validation',
    )
    expect(runButtons).toHaveLength(1)

    wrapper.unmount()
  })

  it('shows a legacy-vs-current admission preview when policy differs from legacy', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const dataStore = useDataStore(pinia)
    const validationStore = useValidationStore(pinia)
    const artifactsStore = useWorkflowArtifactsStore(pinia)

    workflow.selectResultsTab('validate', { persist: false })
    dataStore.csvPath = 'C:/data/aapl.csv'
    dataStore.dataTimeframe = '1m'
    validationStore.candidates = '2'
    validationStore.admissionPolicy = 'scalar_union'
    artifactsStore.applyVueRuntimePayload({
      currentRun: {
        ts: '2026-04-04T10:00:00Z',
        csv_path: 'C:/data/aapl.csv',
        dataset_meta: {
          csv_path: 'C:/data/aapl.csv',
          timeframe: '1m',
          label: 'AAPL',
        },
        req: {
          strategy_kind: 'DoubleMa',
          data_id: 'data-validation-preview',
          objective: 'Sharpe',
          ma_source: 'close',
          fast_ma_types: ['sma'],
          slow_ma_types: ['ema'],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: 'sma',
                slow_ma_type: 'ema',
                fast_len: 10,
                slow_len: 40,
              },
              metrics: {
                pnl: 0.15,
                sharpe: 1.8,
                calmar: 0.7,
                omega: 1.02,
                max_dd: 0.32,
              },
            },
            {
              params: {
                fast_ma_type: 'sma',
                slow_ma_type: 'ema',
                fast_len: 12,
                slow_len: 48,
              },
              metrics: {
                pnl: 0.13,
                sharpe: 1.4,
                calmar: 0.85,
                omega: 1.08,
                max_dd: 0.24,
              },
            },
            {
              params: {
                fast_ma_type: 'sma',
                slow_ma_type: 'ema',
                fast_len: 8,
                slow_len: 26,
              },
              metrics: {
                pnl: 0.1,
                sharpe: 0.9,
                calmar: 1.6,
                omega: 1.35,
                max_dd: 0.06,
              },
            },
          ],
        },
      },
      currentValidation: null,
    })

    const wrapper = mount(ValidationWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const bodyText = document.body.textContent || ''
    expect(bodyText).toContain('Admission Compare')
    expect(bodyText).toContain(
      'scalar_union vs legacy • overlap=1 • current-only=1 • legacy-only=1',
    )
    expect(bodyText).toContain('Admission Delta')
    expect(bodyText).toContain('heads=Sharpe/MaxDrawdown/Omega/Calmar')

    wrapper.unmount()
  })

  it('persists visible validation result columns and restores defaults', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const dataStore = useDataStore(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const validationStore = useValidationStore(pinia)
    workflow.selectResultsTab('validate', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.topRows = 1
    validationStore.tableRowCount = 1
    validationStore.tableVisible = true
    validationStore.validationColumns = [
      { label: 'Route', alignLeft: true },
      { label: 'Warning', alignLeft: true },
      { label: 'Score', alignLeft: false },
    ]
    validationStore.validationRows = [
      {
        cells: [
          { text: 'GpuKernel', className: 'left', title: 'GpuKernel' },
          { text: 'Stable', className: 'left', title: 'Stable' },
          { text: '0.92', className: '', title: '0.92' },
        ],
      },
    ]

    const wrapper = mount(ValidationWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const scoreToggle = document.querySelector<HTMLInputElement>('#validationColumnToggle_col_2')
    expect(scoreToggle).not.toBeNull()
    scoreToggle!.checked = false
    scoreToggle!.dispatchEvent(new Event('change', { bubbles: true }))
    await flushPanel()

    const headerText = Array.from(document.querySelectorAll('th')).map((cell) =>
      String(cell.textContent || '').trim(),
    )
    expect(headerText).toEqual(['Route', 'Warning'])
    expect(localStorage.getItem(VALIDATION_RESULTS_TABLE_STATE_KEY)).toContain('"visibleColumnKeys":["col_0","col_1"]')

    wrapper.unmount()

    mount(ValidationWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const remountHeaderText = Array.from(document.querySelectorAll('th')).map((cell) =>
      String(cell.textContent || '').trim(),
    )
    expect(remountHeaderText).toEqual(['Route', 'Warning'])

    const restoreButton = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) =>
      String(button.textContent || '').includes('Restore defaults'),
    )
    restoreButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await flushPanel()

    const restoredHeaderText = Array.from(document.querySelectorAll('th')).map((cell) =>
      String(cell.textContent || '').trim(),
    )
    expect(restoredHeaderText).toEqual(['Route', 'Warning', 'Score'])
    expect(localStorage.getItem(VALIDATION_RESULTS_TABLE_STATE_KEY)).toBeNull()
  })
})
