import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { describe, expect, it } from 'vitest'
import ValidationContextDock from './ValidationContextDock.vue'
import { bindRuntimePinia } from '../stores/runtimePinia'
import { useValidationStore } from '../stores/validationStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { useWorkflowArtifactsStore } from '../stores/workflowArtifactsStore'
import { useDataStore } from '../stores/dataStore'

const baseDataMeta = {
  len: 0,
  hasOpen: true,
  hasHigh: true,
  hasLow: true,
  hasClose: true,
  hasVolume: true,
  minTs: 0,
  maxTs: 0,
  timeframe: '15m',
  sourceSizeBytes: null,
  sourceMtimeUtcMs: null,
} as const

describe('ValidationContextDock', () => {
  it('renders validation summary state and opens the validation workspace action', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    bindRuntimePinia(pinia)

    const dataStore = useDataStore(pinia)
    dataStore.csvPath = 'C:/data/AAPL.csv'
    dataStore.dataTimeframe = '1h'
    dataStore.datasetLabel = 'AAPL'

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-04-09T12:00:00Z',
        csv_path: 'C:/data/AAPL.csv',
        dataset_meta: {
          csv_path: 'C:/data/AAPL.csv',
          timeframe: '1h',
          label: 'AAPL',
          symbol: 'AAPL',
        },
        req: { strategy_kind: 'DoubleMa' },
        res: {
          top: [{ params: { fast_ma_type: 'sma', slow_ma_type: 'ema', fast_len: 10, slow_len: 40 } }],
        },
      },
      currentValidation: null,
    })

    const validationStore = useValidationStore(pinia)
    validationStore.applyLegacySnapshot({
      validationMeta: '32 candidates',
      validationOutputPreview: 'report.xlsx',
      crossMeta: 'selected 2/6',
      tableRowCount: 12,
      minPsr: '0.80',
      minDsr: '0.60',
      cscvSlices: '16',
      statusLine: 'Ready.',
    })

    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflowStep('optimize', { persist: false })

    const wrapper = mount(ValidationContextDock, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    expect(wrapper.text()).toContain('Validation Context')
    expect(wrapper.text()).toContain('32 candidates')
    expect(wrapper.text()).toContain('Compatible: dataset + timeframe + strategy aligned')
    expect(wrapper.text()).toContain('PSR≥0.80')
    expect(wrapper.text()).toContain('DSR≥0.60')
    expect(wrapper.text()).toContain('CSCV=16')

    await wrapper.get('button').trigger('click')
    expect(workflowStore.workflowStep).toBe('validate')
  })

  it('surfaces explicit compatible validation context state when workflow data matches the saved run', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    bindRuntimePinia(pinia)

    useValidationStore(pinia).applyLegacySnapshot({
      validationMeta: '32 candidates',
      validationOutputPreview: 'report.xlsx',
      crossMeta: 'selected 0/0',
      tableRowCount: 12,
      statusLine: 'Ready.',
    })

    const dataStore = useDataStore(pinia)
    dataStore.csvPath = 'C:/data/AAPL.csv'
    dataStore.dataTimeframe = '15m'
    dataStore.baseDataMeta = { ...baseDataMeta }

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-04-09T09:00:00Z',
        csv_path: 'C:/data/AAPL.csv',
        dataset_meta: {
          csv_path: 'C:/data/AAPL.csv',
          timeframe: '15m',
          label: 'AAPL',
          symbol: 'AAPL',
        },
        req: {
          strategy_kind: 'DoubleMa',
          data_id: 'data-aapl',
          ma_source: 'close',
          fast_ma_types: ['sma'],
          slow_ma_types: ['ema'],
        },
        res: {
          top: [{ params: { fast_ma_type: 'sma', slow_ma_type: 'ema', fast_len: 10, slow_len: 40 } }],
        },
      },
      currentValidation: null,
    })

    const wrapper = mount(ValidationContextDock, {
      global: { plugins: [pinia] },
    })
    await nextTick()

    const compatibility = wrapper.get('[data-validation-compat]')
    expect(compatibility.text()).toContain('Compatible: dataset + timeframe + strategy aligned')
    expect(compatibility.attributes('title')).toContain('matches the artifact context')
  })

  it('surfaces explicit incompatible validation context state when workflow data mismatches the saved run', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    bindRuntimePinia(pinia)

    useValidationStore(pinia).applyLegacySnapshot({
      validationMeta: '32 candidates',
      validationOutputPreview: 'report.xlsx',
      crossMeta: 'selected 0/0',
      tableRowCount: 12,
      statusLine: 'Ready.',
    })

    const dataStore = useDataStore(pinia)
    dataStore.csvPath = 'C:/data/MSFT.csv'
    dataStore.dataTimeframe = '15m'
    dataStore.baseDataMeta = { ...baseDataMeta }

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-04-09T09:10:00Z',
        csv_path: 'C:/data/AAPL.csv',
        dataset_meta: {
          csv_path: 'C:/data/AAPL.csv',
          timeframe: '15m',
          label: 'AAPL',
          symbol: 'AAPL',
        },
        req: {
          strategy_kind: 'DoubleMa',
          data_id: 'data-aapl',
          ma_source: 'close',
          fast_ma_types: ['sma'],
          slow_ma_types: ['ema'],
        },
        res: {
          top: [{ params: { fast_ma_type: 'sma', slow_ma_type: 'ema', fast_len: 10, slow_len: 40 } }],
        },
      },
      currentValidation: null,
    })

    const wrapper = mount(ValidationContextDock, {
      global: { plugins: [pinia] },
    })
    await nextTick()

    const compatibility = wrapper.get('[data-validation-compat]')
    expect(compatibility.text()).toBe('Incompatible: dataset mismatch')
    expect(compatibility.attributes('title')).toContain('expects dataset AAPL')
  })

  it('surfaces partial compatibility when artifact metadata is incomplete', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    bindRuntimePinia(pinia)

    useValidationStore(pinia).applyLegacySnapshot({
      validationMeta: '32 candidates',
      validationOutputPreview: 'report.xlsx',
      crossMeta: 'selected 0/0',
      tableRowCount: 12,
      statusLine: 'Ready.',
    })

    const dataStore = useDataStore(pinia)
    dataStore.csvPath = 'C:/data/AAPL.csv'
    dataStore.dataTimeframe = '15m'
    dataStore.baseDataMeta = { ...baseDataMeta }

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-04-09T09:20:00Z',
        csv_path: '',
        dataset_meta: null,
        req: {},
        res: {
          top: [{ params: { fast_ma_type: 'sma', slow_ma_type: 'ema', fast_len: 10, slow_len: 40 } }],
        },
      },
      currentValidation: null,
    })

    const wrapper = mount(ValidationContextDock, {
      global: { plugins: [pinia] },
    })
    await nextTick()

    const compatibility = wrapper.get('[data-validation-compat]')
    expect(compatibility.text()).toBe('Partial: missing dataset + timeframe + strategy')
    expect(compatibility.attributes('title')).toContain('cannot be fully confirmed')
  })
})
