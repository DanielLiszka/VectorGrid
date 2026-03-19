import type {
  DynamicIndicatorCatalogEntry,
  DynamicIndicatorOutputDescriptor,
  DynamicIndicatorParamDescriptor,
  DynamicIndicatorParamValue,
} from '../services/dynamicIndicatorCatalogService'

export type DynamicBuilderConditionOp = 'gt' | 'ge' | 'lt' | 'le' | 'cross_up' | 'cross_down'
export type DynamicBuilderGroupMode = 'all' | 'any'
export type DynamicBuilderOperandKind = 'indicator' | 'close' | 'ma_source' | 'constant'

export interface DynamicBuilderOperand {
  kind: DynamicBuilderOperandKind
  value: string
}

export interface DynamicBuilderIndicatorSlot {
  id: string
  indicatorId: string
  outputId: string
  params: Record<string, DynamicIndicatorParamValue>
}

export interface DynamicBuilderCondition {
  id: string
  op: DynamicBuilderConditionOp
  lhs: DynamicBuilderOperand
  rhs: DynamicBuilderOperand
}

export interface DynamicBuilderGroup {
  id: string
  mode: DynamicBuilderGroupMode
  conditionIds: string[]
}

export interface DynamicBuilderSignalBindings {
  longEntryGroup: string
  longExitGroup: string
  shortEntryGroup: string
  shortExitGroup: string
}

export interface DynamicDoubleMaBuilderState {
  longOnly: boolean
  allowFlip: boolean
  stopLossPct: string
  takeProfitPct: string
  indicators: DynamicBuilderIndicatorSlot[]
  conditions: DynamicBuilderCondition[]
  groups: DynamicBuilderGroup[]
  signals: DynamicBuilderSignalBindings
}

const DEFAULT_LONG_CONDITIONS: DynamicBuilderCondition[] = [
  {
    id: 'long_entry_cond',
    op: 'cross_up',
    lhs: { kind: 'indicator', value: 'fast_ma' },
    rhs: { kind: 'indicator', value: 'slow_ma' },
  },
  {
    id: 'long_exit_cond',
    op: 'cross_down',
    lhs: { kind: 'indicator', value: 'fast_ma' },
    rhs: { kind: 'indicator', value: 'slow_ma' },
  },
]

const DEFAULT_SHORT_CONDITIONS: DynamicBuilderCondition[] = [
  {
    id: 'short_entry_cond',
    op: 'cross_down',
    lhs: { kind: 'indicator', value: 'fast_ma' },
    rhs: { kind: 'indicator', value: 'slow_ma' },
  },
  {
    id: 'short_exit_cond',
    op: 'cross_up',
    lhs: { kind: 'indicator', value: 'fast_ma' },
    rhs: { kind: 'indicator', value: 'slow_ma' },
  },
]

const DEFAULT_LONG_GROUPS: DynamicBuilderGroup[] = [
  { id: 'long_entry_group', mode: 'all', conditionIds: ['long_entry_cond'] },
  { id: 'long_exit_group', mode: 'all', conditionIds: ['long_exit_cond'] },
]

const DEFAULT_SHORT_GROUPS: DynamicBuilderGroup[] = [
  { id: 'short_entry_group', mode: 'all', conditionIds: ['short_entry_cond'] },
  { id: 'short_exit_group', mode: 'all', conditionIds: ['short_exit_cond'] },
]

interface BuiltStrategyIr {
  schema_version: number
  profile: 'double_ma'
  ma_source: string
  fast_ma: {
    ma_type: string
    params: Record<string, number>
  }
  slow_ma: {
    ma_type: string
    params: Record<string, number>
  }
  program: {
    indicators: Array<Record<string, unknown>>
    conditions: Array<Record<string, unknown>>
    groups: Array<Record<string, unknown>>
    signals: Record<string, string | null>
  }
  execution: {
    strategy: Record<string, unknown>
  }
}

const BUILDER_SCHEMA_VERSION = 2

function normalizeText(value: unknown): string {
  return String(value ?? '').trim()
}

function parseFinite(value: unknown): number | null {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : null
}

function encodeF64BitsDecimalString(value: number): string {
  const buffer = new ArrayBuffer(8)
  const view = new DataView(buffer)
  view.setFloat64(0, value, false)
  return view.getBigUint64(0, false).toString(10)
}

export function isFloatingOutput(output: DynamicIndicatorOutputDescriptor | null | undefined): boolean {
  const valueType = normalizeText(output?.valueType).toLowerCase()
  return valueType === 'f64' || valueType === 'f32'
}

function canDefaultParam(param: DynamicIndicatorParamDescriptor): boolean {
  if (param.defaultValue) return true
  if (param.kind === 'enum_string') return param.enumValues.length > 0
  if (param.kind === 'bool') return true
  if (param.kind === 'int' || param.kind === 'float') {
    return param.min != null || param.max != null
  }
  return false
}

function defaultParamValue(param: DynamicIndicatorParamDescriptor): DynamicIndicatorParamValue | null {
  if (param.defaultValue) return param.defaultValue
  if (param.kind === 'enum_string') {
    const first = param.enumValues[0]
    return first ? { kind: 'enum_string', value: first } : null
  }
  if (param.kind === 'bool') return { kind: 'bool', value: false }
  if (param.kind === 'int') {
    const base = param.min ?? param.max ?? 1
    return { kind: 'int', value: Math.round(base) }
  }
  if (param.kind === 'float') {
    const base = param.min ?? param.max ?? 1
    return { kind: 'float', value: base }
  }
  return null
}

export function builderEligibleDynamicIndicators(
  catalog: DynamicIndicatorCatalogEntry[],
): DynamicIndicatorCatalogEntry[] {
  return catalog.filter((entry) => {
    if (!entry.dynamicStrategyEligible) return false
    if (!entry.outputs.some((output) => isFloatingOutput(output))) return false
    return entry.params.every((param) => !param.required || canDefaultParam(param))
  })
}

export function preferredFloatingOutputId(entry: DynamicIndicatorCatalogEntry | null | undefined): string {
  if (!entry) return ''
  const exact = entry.outputs.find((output) => normalizeText(output.id) === 'value' && isFloatingOutput(output))
  if (exact) return exact.id
  return entry.outputs.find((output) => isFloatingOutput(output))?.id || ''
}

export function defaultIndicatorParams(
  entry: DynamicIndicatorCatalogEntry | null | undefined,
): Record<string, DynamicIndicatorParamValue> {
  if (!entry) return {}
  const out: Record<string, DynamicIndicatorParamValue> = {}
  for (const param of entry.params) {
    const next = defaultParamValue(param)
    if (!next) continue
    out[param.key] = next
  }
  return out
}

export function createDefaultIndicatorSlot(
  entry: DynamicIndicatorCatalogEntry | null | undefined,
  id = 'indicator_1',
): DynamicBuilderIndicatorSlot {
  return {
    id,
    indicatorId: normalizeText(entry?.id),
    outputId: preferredFloatingOutputId(entry),
    params: defaultIndicatorParams(entry),
  }
}

export function createDefaultDynamicDoubleMaBuilderState(
  catalog: DynamicIndicatorCatalogEntry[],
): DynamicDoubleMaBuilderState {
  const eligible = builderEligibleDynamicIndicators(catalog)
  const firstIndicator = eligible[0] ?? null
  return {
    longOnly: true,
    allowFlip: false,
    stopLossPct: '',
    takeProfitPct: '',
    indicators: firstIndicator ? [createDefaultIndicatorSlot(firstIndicator)] : [],
    conditions: DEFAULT_LONG_CONDITIONS.map((condition) => structuredClone(condition)),
    groups: DEFAULT_LONG_GROUPS.map((group) => structuredClone(group)),
    signals: {
      longEntryGroup: 'long_entry_group',
      longExitGroup: 'long_exit_group',
      shortEntryGroup: '',
      shortExitGroup: '',
    },
  }
}

export function builderUsesConstantOperands(builder: DynamicDoubleMaBuilderState): boolean {
  return builder.conditions.some(
    (condition) => condition.lhs.kind === 'constant' || condition.rhs.kind === 'constant',
  )
}

export function setDynamicDoubleMaBuilderLongOnly(
  builder: DynamicDoubleMaBuilderState,
  longOnly: boolean,
): void {
  builder.longOnly = longOnly
  if (longOnly) {
    builder.allowFlip = false
    builder.signals.shortEntryGroup = ''
    builder.signals.shortExitGroup = ''
    const removedConditionIds = new Set(DEFAULT_SHORT_CONDITIONS.map((condition) => condition.id))
    const removedGroupIds = new Set(DEFAULT_SHORT_GROUPS.map((group) => group.id))
    builder.conditions = builder.conditions.filter((condition) => !removedConditionIds.has(condition.id))
    builder.groups = builder.groups
      .filter((group) => !removedGroupIds.has(group.id))
      .map((group) => ({
        ...group,
        conditionIds: group.conditionIds.filter((conditionId) => !removedConditionIds.has(conditionId)),
      }))
    return
  }

  for (const condition of DEFAULT_SHORT_CONDITIONS) {
    if (builder.conditions.some((entry) => entry.id === condition.id)) continue
    builder.conditions.push(structuredClone(condition))
  }
  for (const group of DEFAULT_SHORT_GROUPS) {
    if (builder.groups.some((entry) => entry.id === group.id)) continue
    builder.groups.push(structuredClone(group))
  }
  if (!normalizeText(builder.signals.shortEntryGroup)) builder.signals.shortEntryGroup = 'short_entry_group'
  if (!normalizeText(builder.signals.shortExitGroup)) builder.signals.shortExitGroup = 'short_exit_group'
}

function buildOperand(operand: DynamicBuilderOperand): Record<string, unknown> {
  if (operand.kind === 'constant') {
    const numeric = parseFinite(operand.value)
    if (numeric == null) {
      throw new Error(`Constant operand '${operand.value}' is not a finite number.`)
    }
    return { kind: 'constant_bits', bits: encodeF64BitsDecimalString(numeric) }
  }
  if (operand.kind === 'close') return { kind: 'close' }
  if (operand.kind === 'ma_source') return { kind: 'ma_source' }
  const id = normalizeText(operand.value)
  if (!id) throw new Error('Indicator operand is missing an indicator id.')
  return { kind: 'indicator', id }
}

function buildIndicatorParamKvs(params: Record<string, DynamicIndicatorParamValue>): Array<Record<string, unknown>> {
  return Object.entries(params)
    .filter(([key]) => normalizeText(key).length > 0)
    .map(([key, value]) => ({
      key,
      value: {
        kind: value.kind,
        value: value.value,
      },
    }))
}

function parseOptionalPct(value: string): number | null {
  const trimmed = normalizeText(value)
  if (!trimmed) return null
  const numeric = parseFinite(trimmed)
  if (numeric == null) {
    throw new Error(`Risk field '${value}' is not a finite number.`)
  }
  return numeric
}

export function buildDynamicDoubleMaStrategyIrFromBuilder(input: {
  fastMaType: string
  slowMaType: string
  fastMaParams?: Record<string, number> | null
  slowMaParams?: Record<string, number> | null
  builder: DynamicDoubleMaBuilderState
}): BuiltStrategyIr {
  const fastMaType = normalizeText(input.fastMaType).toLowerCase()
  const slowMaType = normalizeText(input.slowMaType).toLowerCase()
  if (!fastMaType || !slowMaType) {
    throw new Error('Fast and slow MA types must be selected.')
  }
  const builder = input.builder
  const indicators: Array<Record<string, unknown>> = [
    { id: 'fast_ma', kind: 'fast_ma', params: [], output_id: null },
    { id: 'slow_ma', kind: 'slow_ma', params: [], output_id: null },
  ]
  for (const slot of builder.indicators) {
    const slotId = normalizeText(slot.id)
    const indicatorId = normalizeText(slot.indicatorId).toLowerCase()
    const outputId = normalizeText(slot.outputId).toLowerCase()
    if (!slotId || !indicatorId || !outputId) continue
    indicators.push({
      id: slotId,
      kind: null,
      indicator_id: indicatorId,
      params: buildIndicatorParamKvs(slot.params),
      output_id: outputId,
    })
  }

  const rawConditions = builder.conditions.map((condition) => ({
    id: normalizeText(condition.id),
    op: condition.op,
    lhs: buildOperand(condition.lhs),
    rhs: buildOperand(condition.rhs),
  }))

  const rawGroups = builder.groups.map((group) => ({
    id: normalizeText(group.id),
    mode: group.mode,
    condition_ids: group.conditionIds.map((id) => normalizeText(id)).filter((id) => id.length > 0),
  }))

  const signals = {
    long_entry_group: normalizeText(builder.signals.longEntryGroup) || null,
    long_exit_group: normalizeText(builder.signals.longExitGroup) || null,
    short_entry_group: builder.longOnly ? null : (normalizeText(builder.signals.shortEntryGroup) || null),
    short_exit_group: builder.longOnly ? null : (normalizeText(builder.signals.shortExitGroup) || null),
  }

  const referencedGroupIds = new Set(
    Object.values(signals).filter((value): value is string => typeof value === 'string' && value.length > 0),
  )
  const groups = rawGroups.filter((group) => referencedGroupIds.has(String(group.id)))
  const referencedConditionIds = new Set(
    groups.flatMap((group) =>
      Array.isArray(group.condition_ids)
        ? group.condition_ids.filter((conditionId): conditionId is string => typeof conditionId === 'string' && conditionId.length > 0)
        : [],
    ),
  )
  const conditions = rawConditions.filter((condition) => referencedConditionIds.has(String(condition.id)))

  return {
    schema_version: BUILDER_SCHEMA_VERSION,
    profile: 'double_ma',
    ma_source: 'close',
    fast_ma: {
      ma_type: fastMaType,
      params: input.fastMaParams || {},
    },
    slow_ma: {
      ma_type: slowMaType,
      params: input.slowMaParams || {},
    },
    program: {
      indicators,
      conditions,
      groups,
      signals,
    },
    execution: {
      strategy: {
        long_only: builder.longOnly,
        allow_flip: builder.longOnly ? false : builder.allowFlip,
        trade_on_next_bar: true,
        commission: 0.0,
        eps_rel: 0.0,
        stop_loss_pct: parseOptionalPct(builder.stopLossPct),
        take_profit_pct: parseOptionalPct(builder.takeProfitPct),
      },
    },
  }
}

export function formatDynamicDoubleMaBuilderIrJson(input: {
  fastMaType: string
  slowMaType: string
  fastMaParams?: Record<string, number> | null
  slowMaParams?: Record<string, number> | null
  builder: DynamicDoubleMaBuilderState
}): string {
  return JSON.stringify(buildDynamicDoubleMaStrategyIrFromBuilder(input), null, 2)
}
