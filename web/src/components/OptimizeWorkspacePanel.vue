<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useOptimizeStore, type OptimizeOption } from '../stores/optimizeStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { useQueuedInputWriter } from '../composables/useQueuedInputWriter'
import {
  listMovingAverageCatalog,
  type MaParamDescriptor,
  type MaParamKind,
  type MovingAverageCatalogEntry,
} from '../services/movingAverageCatalogService'
import {
  listDynamicIndicatorCatalog,
  type DynamicIndicatorCatalogEntry,
} from '../services/dynamicIndicatorCatalogService'
import { listGpuSpeedupMap, type GpuSpeedupEntry } from '../services/gpuSpeedupsService'

type MaSide = 'fast' | 'slow'
interface MaParamUiState {
  value: string
  sweepEnabled: boolean
  sweepStart: string
  sweepEnd: string
  sweepStep: string
}
type MaParamUiStateByKey = Record<string, MaParamUiState>
type MaParamUiStateByMa = Record<string, MaParamUiStateByKey>
interface RenderedMaParam {
  schema: MaParamDescriptor
  state: MaParamUiState
}

const optimizeStore = useOptimizeStore()
const workflowStore = useWorkflowStore()
const {
  strategyKind,
  strategyHint,
  strategyOptions,
  presetProfile,
  presetOptions,
  backend,
  backendOptions,
  deviceId,
  deviceOptions,
  objective,
  mode,
  fastStart,
  fastEnd,
  fastStep,
  slowStart,
  slowEnd,
  slowStep,
  fastMaType,
  fastMaTypeOptions,
  slowMaType,
  slowMaTypeOptions,
  pairsPreview,
  gpuStatus,
  gpuParityMeta,
  progressMeta,
  statusLine,
  runDisabled,
  validateDisabled,
  canApplyPreset,
  canPresetKernel,
  canRefreshGpuStatus,
  canParityCheck,
  updatedAtUnixMs,
} = storeToRefs(optimizeStore)
const { controlsTab } = storeToRefs(workflowStore)

const mountReady = ref(false)
const panelRoot = ref<HTMLElement | null>(null)
let panelDisposed = false

const strategyDraft = ref('')
const presetDraft = ref('')
const backendDraft = ref('')
const deviceDraft = ref('0')
const objectiveDraft = ref('Sharpe')
const modeDraft = ref('Grid')
const fastStartDraft = ref('')
const fastEndDraft = ref('')
const fastStepDraft = ref('')
const slowStartDraft = ref('')
const slowEndDraft = ref('')
const slowStepDraft = ref('')
const fastTypeDraft = ref('')
const slowTypeDraft = ref('')
const maParamCatalogLoaded = ref(false)
const maParamSchemasByMaId = shallowRef<Record<string, MaParamDescriptor[]>>({})
const maCatalogEntries = shallowRef<MovingAverageCatalogEntry[]>([])
const dynamicCatalogEntries = shallowRef<DynamicIndicatorCatalogEntry[]>([])
const maSpeedupById = shallowRef<Record<string, GpuSpeedupEntry>>({})
const maSpeedupLoaded = ref(false)
const maParamUiStateBySide = ref<{ fast: MaParamUiStateByMa; slow: MaParamUiStateByMa }>({ fast: {}, slow: {} })
const maParamTouchedBySide = ref<{ fast: Record<string, boolean>; slow: Record<string, boolean> }>({ fast: {}, slow: {} })

const visible = computed(() => String(controlsTab.value || '').trim() === 'opt')
const selectedFastMaType = computed(() => String(fastTypeDraft.value || fastMaType.value || '').trim())
const selectedSlowMaType = computed(() => String(slowTypeDraft.value || slowMaType.value || '').trim())
const fastMaParamSchema = computed(() => maParamSchemasByMaId.value[selectedFastMaType.value] || [])
const slowMaParamSchema = computed(() => maParamSchemasByMaId.value[selectedSlowMaType.value] || [])
const fastRenderedMaParams = computed<RenderedMaParam[]>(() =>
  fastMaParamSchema.value.map((schema) => ({
    schema,
    state: resolveMaParamUiState('fast', selectedFastMaType.value, schema),
  })),
)
const slowRenderedMaParams = computed<RenderedMaParam[]>(() =>
  slowMaParamSchema.value.map((schema) => ({
    schema,
    state: resolveMaParamUiState('slow', selectedSlowMaType.value, schema),
  })),
)

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
})


function resolvedOptions(
  options: OptimizeOption[],
  fallbackValue: string,
  fallbackLabel: string,
  fallbackOptions: OptimizeOption[] = [],
): OptimizeOption[] {
  if (options.length > 0) return options
  if (fallbackOptions.length > 0) return fallbackOptions
  const value = String(fallbackValue || '').trim()
  if (!value) return []
  return [{ value, label: String(fallbackLabel || value).trim() || value, disabled: false }]
}

type BackendKind = 'Auto' | 'CpuOnly' | 'GpuKernel'

function normalizeBackendKind(value: string): BackendKind {
  const next = String(value || '').trim()
  if (next === 'CpuOnly') return 'CpuOnly'
  if (next === 'GpuOnly') return 'GpuKernel'
  if (next === 'GpuHybridKernel') return 'GpuKernel'
  if (next === 'GpuKernel') return 'GpuKernel'
  return 'Auto'
}

function backendLabel(backendKind: BackendKind): string {
  if (backendKind === 'CpuOnly') return 'CPU'
  if (backendKind === 'GpuKernel') return 'GPU unified'
  return 'Auto'
}

function backendOptionDisplayLabel(option: OptimizeOption): string {
  const value = String(option.value || '').trim()
  if (value === 'Auto') return 'Auto (recommended)'
  if (value === 'CpuOnly') return 'CPU only (indicators + backtest on CPU)'
  if (value === 'GpuOnly' || value === 'GpuHybridKernel' || value === 'GpuKernel') {
    return 'GPU unified (kernel + staged fallback)'
  }
  return String(option.label || option.value || '').trim() || value
}

const backendExecutionHint = computed(() => {
  const backendKind = selectedBackend.value
  if (backendKind === 'CpuOnly') {
    return 'All indicator calculations and backtest metrics are executed on CPU (fp64 path).'
  }
  if (backendKind === 'GpuKernel') {
    return 'Unified route: kernel-capable indicators stay on GPU; non-kernel indicators are staged, and fallback paths are handled automatically.'
  }
  return 'Auto selects the best route for strategy constraints, MA capability, and gpu_speedups.csv policy.'
})

function supportsBackend(entry: MovingAverageCatalogEntry, backendKind: BackendKind): boolean {
  if (!entry.periodBased || !entry.singleOutput) return false
  if (backendKind === 'CpuOnly') return entry.supportsCpu
  if (backendKind === 'GpuKernel') {
    // Unified GPU mode can route via kernel, CUDA-sweep fallback, or CPU staging.
    return entry.supportsCudaKernel || entry.supportsCuda || entry.supportsCpu
  }
  return entry.supportsCpu || entry.supportsCuda || entry.supportsCudaKernel
}

function maOptionLabel(
  entry: MovingAverageCatalogEntry,
  backendKind: BackendKind,
  speedup: GpuSpeedupEntry | null,
): string {
  const base = String(entry.label || entry.id).trim() || entry.id
  if (!entry.periodBased || !entry.singleOutput) {
    return `${base} (not period/single-output)`
  }
  if (backendKind !== 'GpuKernel') return base

  const supportsGpuPath = entry.supportsCuda
  const backendSupported = supportsBackend(entry, backendKind)

  let perfText = ''
  if (supportsGpuPath) {
    if (speedup) {
      perfText = `${speedup.speedup.toFixed(2)}x GPU`
    } else if (maSpeedupLoaded.value) {
      perfText = 'warning: GPU slower vs CPU'
    } else {
      perfText = 'GPU speed: loading...'
    }
  }

  const supportText = backendSupported ? '' : `not supported on ${backendLabel(backendKind)}`
  if (perfText && supportText) {
    return `${base} (${perfText}; ${supportText})`
  }
  if (perfText) return `${base} (${perfText})`
  if (supportText) {
    return `${base} (${supportText})`
  }
  return base
}

function buildBackendAwareMaOptions(
  catalog: MovingAverageCatalogEntry[],
  backendKind: BackendKind,
  speedupById: Record<string, GpuSpeedupEntry>,
): OptimizeOption[] {
  const entries = [...catalog]
  entries.sort((a, b) => {
    const left = String(a.label || a.id).toLowerCase()
    const right = String(b.label || b.id).toLowerCase()
    return left.localeCompare(right)
  })
  return entries.map((entry) => {
    const id = String(entry.id || '').trim()
    const speedup = id ? (speedupById[id] || null) : null
    return {
      value: id,
      label: maOptionLabel(entry, backendKind, speedup),
      disabled: !supportsBackend(entry, backendKind),
    }
  })
}

function selectedMaHint(
  maType: string,
  backendKind: BackendKind,
  catalog: MovingAverageCatalogEntry[],
  speedupById: Record<string, GpuSpeedupEntry>,
): string {
  if (backendKind !== 'GpuKernel') return ''
  const id = String(maType || '').trim()
  if (!id) return ''
  const entry = catalog.find((row) => row.id === id)
  if (!entry) return ''
  const notes: string[] = []
  if (!supportsBackend(entry, backendKind)) {
    notes.push(`Selected MA is not supported on ${backendLabel(backendKind)}.`)
  }
  if (entry.supportsCpu && !entry.supportsCudaKernel) {
    notes.push('This MA will use CPU staging in unified GPU mode.')
  }
  if (entry.supportsCuda) {
    const speedup = speedupById[id]
    if (speedup) {
      notes.push(`GPU speedup on this system: ${speedup.speedup.toFixed(2)}x vs CPU.`)
    } else if (maSpeedupLoaded.value) {
      notes.push('Warning: this MA is slower on GPU vs CPU on this system.')
    }
  }
  return notes.join(' ')
}

const selectedBackend = computed<BackendKind>(() => normalizeBackendKind(backendDraft.value || backend.value || 'Auto'))
const backendAwareMaOptions = computed(() =>
  buildBackendAwareMaOptions(maCatalogEntries.value, selectedBackend.value, maSpeedupById.value),
)
const fastMaBackendHint = computed(() =>
  selectedMaHint(selectedFastMaType.value, selectedBackend.value, maCatalogEntries.value, maSpeedupById.value),
)
const slowMaBackendHint = computed(() =>
  selectedMaHint(selectedSlowMaType.value, selectedBackend.value, maCatalogEntries.value, maSpeedupById.value),
)
const dynamicCatalogMeta = computed(() => {
  const total = dynamicCatalogEntries.value.length
  if (total <= 0) return 'Dynamic indicator catalog: unavailable.'
  const gpuPreferred = dynamicCatalogEntries.value.filter((entry) => entry.prefersGpuBySpeedup).length
  return `Dynamic indicator catalog: ${total} eligible indicators (${gpuPreferred} GPU-preferred by gpu_speedups.csv).`
})

const strategyOptionItems = computed(() => resolvedOptions(strategyOptions.value, strategyKind.value, strategyKind.value || 'Strategy'))
const presetOptionItems = computed(() => resolvedOptions(presetOptions.value, presetProfile.value, presetProfile.value || 'Preset'))
const backendOptionItems = computed(() =>
  resolvedOptions(backendOptions.value, backend.value, backend.value || 'Backend')
    .map((option) => ({
      ...option,
      label: backendOptionDisplayLabel(option),
    })),
)
const deviceOptionItems = computed(() => resolvedOptions(deviceOptions.value, deviceId.value || '0', deviceId.value || 'Auto'))
const fastTypeOptionItems = computed(() =>
  resolvedOptions(backendAwareMaOptions.value, fastMaType.value, fastMaType.value || 'MA type', fastMaTypeOptions.value),
)
const slowTypeOptionItems = computed(() =>
  resolvedOptions(backendAwareMaOptions.value, slowMaType.value, slowMaType.value || 'MA type', slowMaTypeOptions.value),
)

watch(
  () => [
    String(strategyKind.value || ''),
    String(presetProfile.value || ''),
    String(backend.value || ''),
    String(deviceId.value || '0'),
    String(objective.value || 'Sharpe'),
    String(mode.value || 'Grid'),
    String(fastStart.value || ''),
    String(fastEnd.value || ''),
    String(fastStep.value || ''),
    String(slowStart.value || ''),
    String(slowEnd.value || ''),
    String(slowStep.value || ''),
    String(fastMaType.value || ''),
    String(slowMaType.value || ''),
  ] as const,
  ([
    nextStrategy,
    nextPreset,
    nextBackend,
    nextDeviceId,
    nextObjective,
    nextMode,
    nextFastStart,
    nextFastEnd,
    nextFastStep,
    nextSlowStart,
    nextSlowEnd,
    nextSlowStep,
    nextFastMaType,
    nextSlowMaType,
  ]) => {
    if (strategyDraft.value !== nextStrategy) strategyDraft.value = nextStrategy
    if (presetDraft.value !== nextPreset) presetDraft.value = nextPreset
    if (backendDraft.value !== nextBackend) backendDraft.value = nextBackend
    if (deviceDraft.value !== nextDeviceId) deviceDraft.value = nextDeviceId
    if (objectiveDraft.value !== nextObjective) objectiveDraft.value = nextObjective
    if (modeDraft.value !== nextMode) modeDraft.value = nextMode
    if (fastStartDraft.value !== nextFastStart) fastStartDraft.value = nextFastStart
    if (fastEndDraft.value !== nextFastEnd) fastEndDraft.value = nextFastEnd
    if (fastStepDraft.value !== nextFastStep) fastStepDraft.value = nextFastStep
    if (slowStartDraft.value !== nextSlowStart) slowStartDraft.value = nextSlowStart
    if (slowEndDraft.value !== nextSlowEnd) slowEndDraft.value = nextSlowEnd
    if (slowStepDraft.value !== nextSlowStep) slowStepDraft.value = nextSlowStep
    if (fastTypeDraft.value !== nextFastMaType) fastTypeDraft.value = nextFastMaType
    if (slowTypeDraft.value !== nextSlowMaType) slowTypeDraft.value = nextSlowMaType
  },
  { immediate: true },
)

function setInputValue(id: string, nextValue: string): void {
  optimizeStore.setInputField(id, nextValue)
}

const { queueInputValue, flushQueuedInputValuesNow } = useQueuedInputWriter(setInputValue)

function setSelectValue(id: string, nextValue: string): void {
  optimizeStore.setSelectField(id, nextValue)
}

function runOptimizationAction(): void {
  flushQueuedInputValuesNow()
  optimizeStore.runOptimization()
}

function runValidationAction(): void {
  flushQueuedInputValuesNow()
  optimizeStore.runValidation()
}

function normalizeFinite(value: unknown): number | null {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : null
}

function serializeNumber(value: number): string {
  return Number.isFinite(value) ? String(value) : ''
}

function defaultSweepStep(schema: MaParamDescriptor): number {
  if (schema.step != null && Number.isFinite(schema.step) && schema.step > 0) return schema.step
  return schema.kind === 'int' ? 1 : 0.1
}

function resolveInputStep(step: number | null, kind: MaParamKind): string {
  if (step != null && Number.isFinite(step) && step > 0) return String(step)
  return kind === 'int' ? '1' : 'any'
}

function createMaParamUiState(
  schema: MaParamDescriptor,
  seededValue: unknown,
  seededSweep: unknown,
): MaParamUiState {
  const defaultValue = normalizeFinite(seededValue) ?? schema.defaultValue
  const sweepRow = Array.isArray(seededSweep) ? seededSweep : null
  const sweepStart = sweepRow ? normalizeFinite(sweepRow[0]) : null
  const sweepEnd = sweepRow ? normalizeFinite(sweepRow[1]) : null
  const sweepStep = sweepRow ? normalizeFinite(sweepRow[2]) : null
  const sweepEnabled =
    sweepStart != null &&
    sweepEnd != null &&
    sweepStep != null &&
    sweepStep > 0

  const resolvedSweepStep = sweepEnabled ? (sweepStep as number) : defaultSweepStep(schema)

  return {
    value: serializeNumber(defaultValue),
    sweepEnabled,
    sweepStart: serializeNumber(sweepEnabled ? (sweepStart as number) : defaultValue),
    sweepEnd: serializeNumber(sweepEnabled ? (sweepEnd as number) : defaultValue),
    sweepStep: serializeNumber(resolvedSweepStep),
  }
}

function resolveSideTypeState(side: MaSide, maId: string): MaParamUiStateByKey {
  if (!maId) return {}
  const sideState = maParamUiStateBySide.value[side]
  const row = sideState[maId]
  return row && typeof row === 'object' ? row : {}
}

function resolveMaParamUiState(side: MaSide, maId: string, schema: MaParamDescriptor): MaParamUiState {
  const state = resolveSideTypeState(side, maId)[schema.key]
  return state || createMaParamUiState(schema, schema.defaultValue, null)
}

function ensureSideTypeSchemaState(
  side: MaSide,
  maId: string,
  schema: MaParamDescriptor[],
  seedParams: unknown,
  seedSweeps: unknown,
  overwrite = false,
): void {
  if (!maId || schema.length === 0) return
  const sideState = maParamUiStateBySide.value[side]
  const paramsRow = seedParams && typeof seedParams === 'object' ? seedParams as Record<string, unknown> : {}
  const sweepsRow = seedSweeps && typeof seedSweeps === 'object' ? seedSweeps as Record<string, unknown> : {}
  let typeState = sideState[maId]
  if (!typeState || typeof typeState !== 'object') {
    typeState = {}
    sideState[maId] = typeState
  }

  const allowed = new Set<string>()
  for (const item of schema) {
    allowed.add(item.key)
    if (!overwrite && typeState[item.key]) continue
    typeState[item.key] = createMaParamUiState(item, paramsRow[item.key], sweepsRow[item.key])
  }

  for (const key of Object.keys(typeState)) {
    if (!allowed.has(key)) delete typeState[key]
  }
}

function buildSideMaParamsAndSweeps(side: MaSide): { params: Record<string, number> | null; sweeps: Record<string, [number, number, number]> | null } {
  const maId = side === 'fast' ? selectedFastMaType.value : selectedSlowMaType.value
  const schema = side === 'fast' ? fastMaParamSchema.value : slowMaParamSchema.value
  if (!maId || schema.length === 0) return { params: null, sweeps: null }

  const stateByKey = resolveSideTypeState(side, maId)
  const params: Record<string, number> = {}
  const sweeps: Record<string, [number, number, number]> = {}
  for (const item of schema) {
    const state = stateByKey[item.key] || createMaParamUiState(item, item.defaultValue, null)
    const rawValue = normalizeFinite(state.value)
    params[item.key] = rawValue ?? item.defaultValue
    if (!state.sweepEnabled) continue
    const start = normalizeFinite(state.sweepStart)
    const end = normalizeFinite(state.sweepEnd)
    const step = normalizeFinite(state.sweepStep)
    if (start == null || end == null || step == null || step <= 0) continue
    sweeps[item.key] = [start, end, step]
  }

  return {
    params: Object.keys(params).length > 0 ? params : null,
    sweeps: Object.keys(sweeps).length > 0 ? sweeps : null,
  }
}

function syncMaParamStateFromVue(): void {
  optimizeStore.syncMaParamState({
    fast: buildSideMaParamsAndSweeps('fast'),
    slow: buildSideMaParamsAndSweeps('slow'),
  })
}

function markSelectedTypeTouched(side: MaSide): void {
  const maId = side === 'fast' ? selectedFastMaType.value : selectedSlowMaType.value
  if (!maId) return
  maParamTouchedBySide.value[side][maId] = true
}

function updateMaParamValue(side: MaSide, key: string, raw: string): void {
  const maId = side === 'fast' ? selectedFastMaType.value : selectedSlowMaType.value
  const schema = side === 'fast' ? fastMaParamSchema.value : slowMaParamSchema.value
  const item = schema.find((row) => row.key === key)
  if (!maId || !item) return
  ensureSideTypeSchemaState(
    side,
    maId,
    schema,
    side === 'fast' ? optimizeStore.fastMaParams : optimizeStore.slowMaParams,
    side === 'fast' ? optimizeStore.fastMaParamSweeps : optimizeStore.slowMaParamSweeps,
    false,
  )
  const stateByKey = resolveSideTypeState(side, maId)
  const state = stateByKey[key]
  if (!state) return
  state.value = raw
  markSelectedTypeTouched(side)
  syncMaParamStateFromVue()
}

function updateMaParamSweepEnabled(side: MaSide, key: string, enabled: boolean): void {
  const maId = side === 'fast' ? selectedFastMaType.value : selectedSlowMaType.value
  const schema = side === 'fast' ? fastMaParamSchema.value : slowMaParamSchema.value
  const item = schema.find((row) => row.key === key)
  if (!maId || !item) return
  const stateByKey = resolveSideTypeState(side, maId)
  const state = stateByKey[key]
  if (!state) return
  state.sweepEnabled = enabled
  if (enabled) {
    if (!String(state.sweepStart || '').trim()) state.sweepStart = String(state.value || item.defaultValue)
    if (!String(state.sweepEnd || '').trim()) state.sweepEnd = String(state.value || item.defaultValue)
    if (!String(state.sweepStep || '').trim()) state.sweepStep = serializeNumber(defaultSweepStep(item))
  }
  markSelectedTypeTouched(side)
  syncMaParamStateFromVue()
}

function updateMaParamSweepField(side: MaSide, key: string, field: 'sweepStart' | 'sweepEnd' | 'sweepStep', raw: string): void {
  const maId = side === 'fast' ? selectedFastMaType.value : selectedSlowMaType.value
  if (!maId) return
  const stateByKey = resolveSideTypeState(side, maId)
  const state = stateByKey[key]
  if (!state) return
  state[field] = raw
  markSelectedTypeTouched(side)
  syncMaParamStateFromVue()
}

function maParamInputId(side: MaSide, key: string): string {
  const maId = side === 'fast' ? selectedFastMaType.value : selectedSlowMaType.value
  return `vue_${side}_${maId || 'ma'}_param_${key}`
}

async function loadMaParamCatalog(): Promise<void> {
  try {
    const catalog = await listMovingAverageCatalog()
    if (panelDisposed) return
    const nextMap: Record<string, MaParamDescriptor[]> = {}
    for (const item of catalog) {
      nextMap[item.id] = item.params
    }
    maParamSchemasByMaId.value = nextMap
    maCatalogEntries.value = catalog
  } catch {
    if (panelDisposed) return
    maParamSchemasByMaId.value = {}
    maCatalogEntries.value = []
  } finally {
    if (!panelDisposed) maParamCatalogLoaded.value = true
  }
}

async function loadMaSpeedups(): Promise<void> {
  try {
    const speedupMap = await listGpuSpeedupMap()
    if (panelDisposed) return
    maSpeedupById.value = speedupMap
  } catch {
    if (panelDisposed) return
    maSpeedupById.value = {}
  } finally {
    if (!panelDisposed) maSpeedupLoaded.value = true
  }
}

async function loadDynamicCatalogMeta(): Promise<void> {
  try {
    const catalog = await listDynamicIndicatorCatalog()
    if (panelDisposed) return
    dynamicCatalogEntries.value = catalog
  } catch {
    if (panelDisposed) return
    dynamicCatalogEntries.value = []
  }
}

function resolveMount(): void {
  const panel = document.querySelector('[data-vue-panel="opt"]') as HTMLElement | null
  const mount = document.querySelector('#optVueShellMount') as HTMLElement | null
  if (!panel || !mount) return
  panelRoot.value = panel
  panel.dataset.vueOpt = '1'
  mountReady.value = true
}

watch([maParamCatalogLoaded, selectedFastMaType, fastMaParamSchema], () => {
  if (!maParamCatalogLoaded.value) return
  ensureSideTypeSchemaState(
    'fast',
    selectedFastMaType.value,
    fastMaParamSchema.value,
    optimizeStore.fastMaParams,
    optimizeStore.fastMaParamSweeps,
  )
  syncMaParamStateFromVue()
}, { immediate: true })

watch([maParamCatalogLoaded, selectedSlowMaType, slowMaParamSchema], () => {
  if (!maParamCatalogLoaded.value) return
  ensureSideTypeSchemaState(
    'slow',
    selectedSlowMaType.value,
    slowMaParamSchema.value,
    optimizeStore.slowMaParams,
    optimizeStore.slowMaParamSweeps,
  )
  syncMaParamStateFromVue()
}, { immediate: true })

watch([() => optimizeStore.fastMaParams, () => optimizeStore.fastMaParamSweeps], () => {
  if (!maParamCatalogLoaded.value) return
  const maId = selectedFastMaType.value
  if (!maId || maParamTouchedBySide.value.fast[maId]) return
  ensureSideTypeSchemaState(
    'fast',
    maId,
    fastMaParamSchema.value,
    optimizeStore.fastMaParams,
    optimizeStore.fastMaParamSweeps,
    true,
  )
}, { deep: true, immediate: true })

watch([() => optimizeStore.slowMaParams, () => optimizeStore.slowMaParamSweeps], () => {
  if (!maParamCatalogLoaded.value) return
  const maId = selectedSlowMaType.value
  if (!maId || maParamTouchedBySide.value.slow[maId]) return
  ensureSideTypeSchemaState(
    'slow',
    maId,
    slowMaParamSchema.value,
    optimizeStore.slowMaParams,
    optimizeStore.slowMaParamSweeps,
    true,
  )
}, { deep: true, immediate: true })

onMounted(async () => {
  await nextTick()
  resolveMount()
  await Promise.all([loadMaParamCatalog(), loadMaSpeedups(), loadDynamicCatalogMeta()])
  syncMaParamStateFromVue()
})

onBeforeUnmount(() => {
  flushQueuedInputValuesNow()
  panelDisposed = true
  if (panelRoot.value) {
    delete panelRoot.value.dataset.vueOpt
    panelRoot.value = null
  }
})
</script>

<template>
  <Teleport v-if="mountReady" to="#optVueShellMount">
    <section class="optPanelVueRoot optVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="title">Optimize</div>
        <div class="meta mono">{{ stamp }}</div>
      </header>

      <section class="pane">
        <div class="gridTwo">
          <div>
            <label for="vueStrategyKind">Strategy</label>
            <select
              id="vueStrategyKind"
              :value="strategyDraft"
              @change="strategyDraft = String(($event.target as HTMLSelectElement).value || ''); setSelectValue('strategyKind', strategyDraft)"
            >
              <option
                v-for="option in strategyOptionItems"
                :key="`strategy_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
            <div class="muted">{{ strategyHint }}</div>
          </div>

          <div>
            <label for="vuePresetProfile">Preset</label>
            <div class="inputWithAction">
              <select
                id="vuePresetProfile"
                :value="presetDraft"
                @change="presetDraft = String(($event.target as HTMLSelectElement).value || ''); setSelectValue('presetProfile', presetDraft)"
              >
                <option
                  v-for="option in presetOptionItems"
                  :key="`preset_${option.value}_${option.label}`"
                  :value="option.value"
                  :disabled="option.disabled"
                >
                  {{ option.label }}
                </option>
              </select>
              <button type="button" :disabled="!canApplyPreset" @click="optimizeStore.applyPreset()">Apply preset</button>
            </div>
          </div>
        </div>

        <div class="gridTwo">
          <div>
            <label for="vueObjective">Objective</label>
            <select
              id="vueObjective"
              :value="objectiveDraft"
              @change="objectiveDraft = String(($event.target as HTMLSelectElement).value || 'Sharpe'); setSelectValue('objective', objectiveDraft)"
            >
              <option value="Sharpe">Sharpe</option>
              <option value="Sortino">Sortino</option>
              <option value="Calmar">Calmar</option>
              <option value="Omega">Omega</option>
              <option value="Pnl">PnL</option>
              <option value="MaxDrawdown">Min drawdown</option>
            </select>
          </div>
          <div>
            <label for="vueMode">Mode</label>
            <select
              id="vueMode"
              :value="modeDraft"
              @change="modeDraft = String(($event.target as HTMLSelectElement).value || 'Grid'); setSelectValue('mode', modeDraft)"
            >
              <option value="Grid">Grid</option>
              <option value="CoarseToFine">Coarse-to-Fine</option>
              <option value="GpuVdps">GPU VDPS</option>
              <option value="Auto">Auto</option>
            </select>
          </div>
        </div>
      </section>

      <section class="pane">
        <div class="gridTwo">
          <div>
            <label for="vueBackend">Backend</label>
            <select
              id="vueBackend"
              :value="backendDraft"
              @change="backendDraft = String(($event.target as HTMLSelectElement).value || ''); setSelectValue('backend', backendDraft)"
            >
              <option
                v-for="option in backendOptionItems"
                :key="`backend_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
            <div class="muted mono">{{ backendExecutionHint }}</div>
          </div>
          <div>
            <label for="vueDevice">GPU device</label>
            <select
              id="vueDevice"
              :value="deviceDraft"
              @change="deviceDraft = String(($event.target as HTMLSelectElement).value || '0'); setSelectValue('deviceId', deviceDraft)"
            >
              <option
                v-for="option in deviceOptionItems"
                :key="`device_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
        </div>
        <div class="actions">
          <button type="button" :disabled="!canPresetKernel" @click="optimizeStore.presetKernel()">Preset GPU kernel</button>
          <button type="button" :disabled="!canRefreshGpuStatus" @click="optimizeStore.refreshGpuStatus()">Refresh GPU status</button>
          <button type="button" :disabled="!canParityCheck" @click="optimizeStore.runGpuParityCheck()">Parity check</button>
        </div>
        <div class="muted mono">{{ gpuStatus }}</div>
        <div class="muted mono">{{ gpuParityMeta }}</div>
        <div class="muted mono">{{ dynamicCatalogMeta }}</div>
      </section>

      <section class="pane">
        <div class="split">
          <div>
            <h3>Fast range</h3>
            <div class="rowThree">
              <div>
                <label for="vueFastStart">Start</label>
                <input
                  id="vueFastStart"
                  type="number"
                  min="1"
                  step="1"
                  :value="fastStartDraft"
                  @input="fastStartDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('fastStart', fastStartDraft)"
                />
              </div>
              <div>
                <label for="vueFastEnd">End</label>
                <input
                  id="vueFastEnd"
                  type="number"
                  min="1"
                  step="1"
                  :value="fastEndDraft"
                  @input="fastEndDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('fastEnd', fastEndDraft)"
                />
              </div>
              <div>
                <label for="vueFastStep">Step</label>
                <input
                  id="vueFastStep"
                  type="number"
                  min="0"
                  step="1"
                  :value="fastStepDraft"
                  @input="fastStepDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('fastStep', fastStepDraft)"
                />
              </div>
            </div>
            <label for="vueFastType">Fast MA type</label>
            <select
              id="vueFastType"
              :value="fastTypeDraft"
              @change="fastTypeDraft = String(($event.target as HTMLSelectElement).value || ''); setSelectValue('fastMaType', fastTypeDraft)"
            >
              <option
                v-for="option in fastTypeOptionItems"
                :key="`fasttype_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
            <div v-if="fastMaBackendHint" class="muted mono">{{ fastMaBackendHint }}</div>
          </div>

          <div>
            <h3>Slow range</h3>
            <div class="rowThree">
              <div>
                <label for="vueSlowStart">Start</label>
                <input
                  id="vueSlowStart"
                  type="number"
                  min="1"
                  step="1"
                  :value="slowStartDraft"
                  @input="slowStartDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('slowStart', slowStartDraft)"
                />
              </div>
              <div>
                <label for="vueSlowEnd">End</label>
                <input
                  id="vueSlowEnd"
                  type="number"
                  min="1"
                  step="1"
                  :value="slowEndDraft"
                  @input="slowEndDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('slowEnd', slowEndDraft)"
                />
              </div>
              <div>
                <label for="vueSlowStep">Step</label>
                <input
                  id="vueSlowStep"
                  type="number"
                  min="0"
                  step="1"
                  :value="slowStepDraft"
                  @input="slowStepDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('slowStep', slowStepDraft)"
                />
              </div>
            </div>
            <label for="vueSlowType">Slow MA type</label>
            <select
              id="vueSlowType"
              :value="slowTypeDraft"
              @change="slowTypeDraft = String(($event.target as HTMLSelectElement).value || ''); setSelectValue('slowMaType', slowTypeDraft)"
            >
              <option
                v-for="option in slowTypeOptionItems"
                :key="`slowtype_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
            <div v-if="slowMaBackendHint" class="muted mono">{{ slowMaBackendHint }}</div>
          </div>
        </div>
        <div class="muted mono">{{ pairsPreview }}</div>
      </section>

      <section class="pane">
        <h3>Run controls</h3>
        <div class="actions">
          <button type="button" class="primary" :disabled="runDisabled" @click="runOptimizationAction">Run optimization</button>
          <button type="button" :disabled="validateDisabled" @click="runValidationAction">Run validation</button>
        </div>
        <div class="muted mono">{{ progressMeta || '—' }}</div>
        <div class="muted mono">{{ statusLine }}</div>
      </section>

      <section class="pane">
        <h3>MA parameters</h3>
        <div class="split">
          <div class="maParamSide">
            <div class="muted maParamTitle">Fast MA parameters</div>
            <div v-if="fastRenderedMaParams.length === 0" class="muted mono">No extra params for the selected MA.</div>
            <div
              v-for="entry in fastRenderedMaParams"
              :key="`fast_ma_param_${entry.schema.key}`"
              class="maParamBlock"
            >
              <label :for="maParamInputId('fast', entry.schema.key)">{{ entry.schema.label }}</label>
              <input
                :id="maParamInputId('fast', entry.schema.key)"
                type="number"
                :min="entry.schema.min ?? undefined"
                :max="entry.schema.max ?? undefined"
                :step="resolveInputStep(entry.schema.step, entry.schema.kind)"
                :value="entry.state.value"
                @input="updateMaParamValue('fast', entry.schema.key, String(($event.target as HTMLInputElement).value || ''))"
              />
              <label class="maParamSweepToggle">
                <input
                  type="checkbox"
                  :checked="entry.state.sweepEnabled"
                  @change="updateMaParamSweepEnabled('fast', entry.schema.key, !!(($event.target as HTMLInputElement).checked))"
                />
                Optimize (sweep)
              </label>
              <div class="rowThree maParamSweepRow" v-show="entry.state.sweepEnabled">
                <div>
                  <label :for="`${maParamInputId('fast', entry.schema.key)}_sweep_start`">Start</label>
                  <input
                    :id="`${maParamInputId('fast', entry.schema.key)}_sweep_start`"
                    type="number"
                    :min="entry.schema.min ?? undefined"
                    :max="entry.schema.max ?? undefined"
                    :step="resolveInputStep(entry.schema.step, entry.schema.kind)"
                    :value="entry.state.sweepStart"
                    @input="updateMaParamSweepField('fast', entry.schema.key, 'sweepStart', String(($event.target as HTMLInputElement).value || ''))"
                  />
                </div>
                <div>
                  <label :for="`${maParamInputId('fast', entry.schema.key)}_sweep_end`">End</label>
                  <input
                    :id="`${maParamInputId('fast', entry.schema.key)}_sweep_end`"
                    type="number"
                    :min="entry.schema.min ?? undefined"
                    :max="entry.schema.max ?? undefined"
                    :step="resolveInputStep(entry.schema.step, entry.schema.kind)"
                    :value="entry.state.sweepEnd"
                    @input="updateMaParamSweepField('fast', entry.schema.key, 'sweepEnd', String(($event.target as HTMLInputElement).value || ''))"
                  />
                </div>
                <div>
                  <label :for="`${maParamInputId('fast', entry.schema.key)}_sweep_step`">Step</label>
                  <input
                    :id="`${maParamInputId('fast', entry.schema.key)}_sweep_step`"
                    type="number"
                    min="0"
                    :step="resolveInputStep(entry.schema.step, entry.schema.kind)"
                    :value="entry.state.sweepStep"
                    @input="updateMaParamSweepField('fast', entry.schema.key, 'sweepStep', String(($event.target as HTMLInputElement).value || ''))"
                  />
                </div>
              </div>
              <div v-if="entry.schema.notes" class="muted maParamNote">{{ entry.schema.notes }}</div>
            </div>
          </div>
          <div class="maParamSide">
            <div class="muted maParamTitle">Slow MA parameters</div>
            <div v-if="slowRenderedMaParams.length === 0" class="muted mono">No extra params for the selected MA.</div>
            <div
              v-for="entry in slowRenderedMaParams"
              :key="`slow_ma_param_${entry.schema.key}`"
              class="maParamBlock"
            >
              <label :for="maParamInputId('slow', entry.schema.key)">{{ entry.schema.label }}</label>
              <input
                :id="maParamInputId('slow', entry.schema.key)"
                type="number"
                :min="entry.schema.min ?? undefined"
                :max="entry.schema.max ?? undefined"
                :step="resolveInputStep(entry.schema.step, entry.schema.kind)"
                :value="entry.state.value"
                @input="updateMaParamValue('slow', entry.schema.key, String(($event.target as HTMLInputElement).value || ''))"
              />
              <label class="maParamSweepToggle">
                <input
                  type="checkbox"
                  :checked="entry.state.sweepEnabled"
                  @change="updateMaParamSweepEnabled('slow', entry.schema.key, !!(($event.target as HTMLInputElement).checked))"
                />
                Optimize (sweep)
              </label>
              <div class="rowThree maParamSweepRow" v-show="entry.state.sweepEnabled">
                <div>
                  <label :for="`${maParamInputId('slow', entry.schema.key)}_sweep_start`">Start</label>
                  <input
                    :id="`${maParamInputId('slow', entry.schema.key)}_sweep_start`"
                    type="number"
                    :min="entry.schema.min ?? undefined"
                    :max="entry.schema.max ?? undefined"
                    :step="resolveInputStep(entry.schema.step, entry.schema.kind)"
                    :value="entry.state.sweepStart"
                    @input="updateMaParamSweepField('slow', entry.schema.key, 'sweepStart', String(($event.target as HTMLInputElement).value || ''))"
                  />
                </div>
                <div>
                  <label :for="`${maParamInputId('slow', entry.schema.key)}_sweep_end`">End</label>
                  <input
                    :id="`${maParamInputId('slow', entry.schema.key)}_sweep_end`"
                    type="number"
                    :min="entry.schema.min ?? undefined"
                    :max="entry.schema.max ?? undefined"
                    :step="resolveInputStep(entry.schema.step, entry.schema.kind)"
                    :value="entry.state.sweepEnd"
                    @input="updateMaParamSweepField('slow', entry.schema.key, 'sweepEnd', String(($event.target as HTMLInputElement).value || ''))"
                  />
                </div>
                <div>
                  <label :for="`${maParamInputId('slow', entry.schema.key)}_sweep_step`">Step</label>
                  <input
                    :id="`${maParamInputId('slow', entry.schema.key)}_sweep_step`"
                    type="number"
                    min="0"
                    :step="resolveInputStep(entry.schema.step, entry.schema.kind)"
                    :value="entry.state.sweepStep"
                    @input="updateMaParamSweepField('slow', entry.schema.key, 'sweepStep', String(($event.target as HTMLInputElement).value || ''))"
                  />
                </div>
              </div>
              <div v-if="entry.schema.notes" class="muted maParamNote">{{ entry.schema.notes }}</div>
            </div>
          </div>
        </div>
      </section>
    </section>
  </Teleport>
</template>

<style scoped>
.optPanelVueRoot {
  display: grid;
  gap: 0.85rem;
}

.head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.75rem;
}

.title {
  font-size: 0.95rem;
  font-weight: 650;
}

.meta {
  font-size: 0.78rem;
  color: var(--muted);
}

.pane {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel-footer);
  padding: 0.75rem 0.8rem;
  min-width: 0;
}

.pane h3 {
  margin: 0 0 0.55rem;
  font-size: 0.86rem;
}

.gridTwo {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.7rem;
}

.gridTwo + .gridTwo {
  margin-top: 0.7rem;
}

.split {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.8rem;
}

.rowThree {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  gap: 0.55rem;
}

.inputWithAction {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.45rem;
}

.actions {
  margin-top: 0.6rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.actions button {
  padding: 0.42rem 0.6rem;
  font-size: 0.76rem;
}

.actions .primary {
  border-color: color-mix(in oklab, var(--accent) 70%, var(--border));
}

.maParamSide {
  min-width: 0;
}

.maParamTitle {
  margin-bottom: 0.35rem;
}

.maParamBlock {
  margin-top: 0.6rem;
}

.maParamSweepToggle {
  margin-top: 0.35rem;
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
}

.maParamSweepRow {
  margin-top: 0.35rem;
}

.maParamNote {
  margin-top: 0.25rem;
  font-size: 0.78rem;
}

.muted {
  color: var(--muted);
  font-size: 0.8rem;
  margin-top: 0.4rem;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 980px) {
  .gridTwo,
  .split,
  .rowThree,
  .inputWithAction {
    grid-template-columns: 1fr;
  }
}
</style>

<style>
[data-vue-panel='opt'][data-vue-opt='1'] > :not(#optVueShellMount) {
  display: none !important;
}
</style>




