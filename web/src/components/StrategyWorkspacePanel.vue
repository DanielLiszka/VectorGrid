<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useStrategyStore, type StrategyOption } from '../stores/strategyStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useWorkflowStore } from '../stores/workflowStore'

const strategyStore = useStrategyStore()
const optimizeStore = useOptimizeStore()
const workflowStore = useWorkflowStore()
const {
  costPreset,
  costPresetOptions,
  feeBps,
  spreadBps,
  slippageBps,
  commission,
  epsRel,
  costHint,
  startingEquity,
  stopLossPct,
  takeProfitPct,
  cooldownBars,
  signalPersistenceBars,
  trailingStopPct,
  dmConstraintMinTrades,
  dmConstraintMaxDd,
  exitMode,
  exitModeOptions,
  kaufmanStopPeriod,
  kaufmanStopMult,
  kaufmanStopMaType,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(strategyStore)
const { strategyKind } = storeToRefs(optimizeStore)
const { controlsTab } = storeToRefs(workflowStore)

const mountReady = ref(false)
const panelRoot = ref<HTMLElement | null>(null)
const adoptedNodes: { node: HTMLElement; parent: Node; next: ChildNode | null }[] = []
const compatCleanupFns: Array<() => void> = []

const LEGACY_STRATEGY_TEXT_IDS = [
  'compositeSignalMode',
  'halfTrendAmplitude',
  'halfTrendAtrPeriod',
  'halfTrendChannelDev',
  'maczFastLength',
  'maczSlowLength',
  'maczSignalLength',
  'maczLengthZ',
  'maczLengthStdev',
  'maczGamma',
  'maczA',
  'maczB',
  'ottPeriod',
  'ottPercent',
  'ottMaType',
  'trendLrPeriod',
  'trendLrMinDeg',
  'trendBbwPeriod',
  'trendBbwMult',
  'trendBbwMin',
  'trendRocPeriod',
  'trendAdxPeriod',
  'trendAdxMin',
  'regimeAdxPeriod',
  'regimeAdxEnter',
  'regimeAdxExit',
  'regimeRevEntryRel',
  'regimeRevExitRel',
  'breakoutBbPeriod',
  'breakoutBbMult',
  'breakoutConfirmBars',
  'breakoutMacdFast',
  'breakoutMacdSlow',
  'breakoutMacdSignal',
  'breakoutCmoPeriod',
  'breakoutCmoMinAbs',
  'breakoutAdoscShort',
  'breakoutAdoscLong',
  'breakoutAdxPeriod',
  'breakoutAdxMin',
] as const

const LEGACY_STRATEGY_CHECKBOX_IDS = [
  'longOnly',
  'allowFlip',
  'tradeOnNext',
  'trendUseRoc',
  'trendUseAdx',
  'breakoutUseAdx',
  'maczUseVolume',
  'maczUseLag',
] as const

const costPresetDraft = ref('custom')
const feeDraft = ref('0')
const spreadDraft = ref('0')
const slippageDraft = ref('0')
const epsDraft = ref('0')
const equityDraft = ref('10000')
const slDraft = ref('')
const tpDraft = ref('')
const cooldownDraft = ref('0')
const persistenceDraft = ref('1')
const trailingDraft = ref('')
const minTradesDraft = ref('')
const maxDdDraft = ref('')
const exitModeDraft = ref('cross')
const kaufmanPeriodDraft = ref('22')
const kaufmanMultDraft = ref('2.0')
const kaufmanTypeDraft = ref('sma')

const visible = computed(() => String(controlsTab.value || '').trim() === 'strategy')
const strategyKindNormalized = computed(() => String(strategyKind.value || 'DoubleMa').trim() || 'DoubleMa')
const isDoubleLikeKind = computed(() => {
  const kind = strategyKindNormalized.value
  return kind === 'DoubleMa' || kind === 'CompositeTrend' || kind === 'RegimeHybrid' || kind === 'BreakoutMomentum'
})
const showDmConstraints = computed(() => isDoubleLikeKind.value)
const showKaufmanFields = computed(() => {
  const mode = String(exitMode.value || 'cross').trim() || 'cross'
  return isDoubleLikeKind.value && mode === 'kaufman_stop'
})

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
})

function resolvedOptions(options: StrategyOption[], fallbackValue: string, fallbackLabel: string): StrategyOption[] {
  if (options.length > 0) return options
  const value = String(fallbackValue || '').trim()
  if (!value) return []
  return [{ value, label: String(fallbackLabel || value).trim() || value, disabled: false }]
}

const costPresetOptionItems = computed(() => resolvedOptions(costPresetOptions.value, costPreset.value, costPreset.value || 'custom'))
const exitModeOptionItems = computed(() => resolvedOptions(exitModeOptions.value, exitMode.value, exitMode.value || 'cross'))

watch(costPreset, (v) => { const next = String(v || 'custom'); if (costPresetDraft.value !== next) costPresetDraft.value = next }, { immediate: true })
watch(feeBps, (v) => { const next = String(v || '0'); if (feeDraft.value !== next) feeDraft.value = next }, { immediate: true })
watch(spreadBps, (v) => { const next = String(v || '0'); if (spreadDraft.value !== next) spreadDraft.value = next }, { immediate: true })
watch(slippageBps, (v) => { const next = String(v || '0'); if (slippageDraft.value !== next) slippageDraft.value = next }, { immediate: true })
watch(epsRel, (v) => { const next = String(v || '0'); if (epsDraft.value !== next) epsDraft.value = next }, { immediate: true })
watch(startingEquity, (v) => { const next = String(v || '10000'); if (equityDraft.value !== next) equityDraft.value = next }, { immediate: true })
watch(stopLossPct, (v) => { const next = String(v || ''); if (slDraft.value !== next) slDraft.value = next }, { immediate: true })
watch(takeProfitPct, (v) => { const next = String(v || ''); if (tpDraft.value !== next) tpDraft.value = next }, { immediate: true })
watch(cooldownBars, (v) => { const next = String(v || '0'); if (cooldownDraft.value !== next) cooldownDraft.value = next }, { immediate: true })
watch(signalPersistenceBars, (v) => { const next = String(v || '1'); if (persistenceDraft.value !== next) persistenceDraft.value = next }, { immediate: true })
watch(trailingStopPct, (v) => { const next = String(v || ''); if (trailingDraft.value !== next) trailingDraft.value = next }, { immediate: true })
watch(dmConstraintMinTrades, (v) => { const next = String(v || ''); if (minTradesDraft.value !== next) minTradesDraft.value = next }, { immediate: true })
watch(dmConstraintMaxDd, (v) => { const next = String(v || ''); if (maxDdDraft.value !== next) maxDdDraft.value = next }, { immediate: true })
watch(exitMode, (v) => {
  const next = String(v || 'cross')
  if (exitModeDraft.value !== next) exitModeDraft.value = next
  syncLegacyStrategyVisibility()
}, { immediate: true })
watch(kaufmanStopPeriod, (v) => { const next = String(v || '22'); if (kaufmanPeriodDraft.value !== next) kaufmanPeriodDraft.value = next }, { immediate: true })
watch(kaufmanStopMult, (v) => { const next = String(v || '2.0'); if (kaufmanMultDraft.value !== next) kaufmanMultDraft.value = next }, { immediate: true })
watch(kaufmanStopMaType, (v) => { const next = String(v || 'sma'); if (kaufmanTypeDraft.value !== next) kaufmanTypeDraft.value = next }, { immediate: true })
watch(strategyKindNormalized, () => {
  syncLegacyStrategyVisibility()
}, { flush: 'post' })
watch(updatedAtUnixMs, () => {
  syncLegacyStrategyInputsFromStore()
  syncLegacyStrategyVisibility()
}, { flush: 'post' })

function setInputValue(id: string, nextValue: string): void {
  strategyStore.setInputField(id, nextValue)
}

function setSelectValue(id: string, nextValue: string): void {
  strategyStore.setSelectField(id, nextValue)
}

function queryLegacyById<T extends Element>(id: string): T | null {
  const selector = `[id="${id}"]`
  const panel = panelRoot.value ?? (document.querySelector('[data-vue-panel="strategy"]') as HTMLElement | null)
  if (panel) {
    const scoped = panel.querySelector(selector) as T | null
    if (scoped) return scoped
  }
  const byQuery = typeof document.querySelector === 'function'
    ? document.querySelector(selector)
    : null
  if (byQuery) return byQuery as T
  const getById = (document as unknown as { getElementById?: (nextId: string) => Element | null }).getElementById
  if (typeof getById !== 'function') return null
  return (getById.call(document, id) as T | null) ?? null
}

function setElementDisplay(id: string, visibleValue: boolean): void {
  const el = queryLegacyById<HTMLElement>(id)
  if (!el) return
  el.style.display = visibleValue ? '' : 'none'
}

function setElementDisabled(id: string, disabled: boolean): void {
  const el = queryLegacyById<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>(id)
  if (!el) return
  el.disabled = disabled
}

function readChecked(id: string): boolean {
  const el = queryLegacyById<HTMLInputElement>(id)
  return !!el?.checked
}

function clearCompatListeners(): void {
  while (compatCleanupFns.length > 0) {
    const cleanup = compatCleanupFns.pop()
    if (!cleanup) continue
    cleanup()
  }
}

function syncLegacyStrategyVisibility(): void {
  const kind = strategyKindNormalized.value
  const isCompTrend = kind === 'CompositeTrend'
  const isRegimeHybrid = kind === 'RegimeHybrid'
  const isBreakoutMomentum = kind === 'BreakoutMomentum'
  const isCompCpu = kind === 'CompositeCpu'
  const isComp = isCompCpu || kind === 'CompositeMo'
  const useKaufmanExit = showKaufmanFields.value

  setElementDisplay('trendQualityWrap', isCompTrend || isRegimeHybrid)
  setElementDisplay('regimeHybridParamsWrap', isRegimeHybrid)
  setElementDisplay('breakoutMomentumWrap', isBreakoutMomentum)
  setElementDisplay('compositeSignalModeWrap', isComp)
  setElementDisplay('compositeParamsWrap', isCompCpu)

  setElementDisabled('trendAdxPeriod', !(isCompTrend && readChecked('trendUseAdx')))
  setElementDisabled('trendAdxMin', !(isCompTrend && readChecked('trendUseAdx')))
  setElementDisabled('breakoutAdxPeriod', !(isBreakoutMomentum && readChecked('breakoutUseAdx')))
  setElementDisabled('breakoutAdxMin', !(isBreakoutMomentum && readChecked('breakoutUseAdx')))
  setElementDisabled('allowFlip', useKaufmanExit)

  const allowFlip = queryLegacyById<HTMLInputElement>('allowFlip')
  if (allowFlip) {
    allowFlip.title = useKaufmanExit
      ? 'Allow flip is ignored when Exit mode = Kaufman-stop.'
      : 'Allow switching long↔short on opposite signals (when Long-only is off).'
  }
}

function seedLegacyStrategyInputsToStore(): void {
  const storeMap = strategyStore as unknown as Record<string, unknown>
  for (const id of LEGACY_STRATEGY_TEXT_IDS) {
    const el = queryLegacyById<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>(id)
    if (!el) continue
    const domValue = String(el.value || '').trim()
    const storeValue = String(storeMap[id] ?? '').trim()
    if (!storeValue && domValue) strategyStore.setInputField(id, domValue)
  }
  for (const id of LEGACY_STRATEGY_CHECKBOX_IDS) {
    const el = queryLegacyById<HTMLInputElement>(id)
    if (!el) continue
    const storeValue = !!storeMap[id]
    if (storeValue !== !!el.checked) strategyStore.setCheckboxField(id, !!el.checked)
  }
}

function syncLegacyStrategyInputsFromStore(): void {
  const storeMap = strategyStore as unknown as Record<string, unknown>
  for (const id of LEGACY_STRATEGY_TEXT_IDS) {
    const el = queryLegacyById<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>(id)
    if (!el) continue
    const nextValue = String(storeMap[id] ?? '')
    if (String(el.value || '') === nextValue) continue
    el.value = nextValue
  }
  for (const id of LEGACY_STRATEGY_CHECKBOX_IDS) {
    const el = queryLegacyById<HTMLInputElement>(id)
    if (!el) continue
    const nextChecked = !!storeMap[id]
    if (!!el.checked === nextChecked) continue
    el.checked = nextChecked
  }
}

function bindLegacyStrategyInputs(): void {
  clearCompatListeners()
  const onTextChange = (id: string, target: EventTarget | null) => {
    const el = target as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement | null
    if (!el) return
    strategyStore.setInputField(id, String(el.value || ''))
    syncLegacyStrategyVisibility()
  }
  const onCheckboxChange = (id: string, target: EventTarget | null) => {
    const el = target as HTMLInputElement | null
    if (!el) return
    strategyStore.setCheckboxField(id, !!el.checked)
    syncLegacyStrategyVisibility()
  }

  for (const id of LEGACY_STRATEGY_TEXT_IDS) {
    const el = queryLegacyById<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>(id)
    if (!el) continue
    const handler = (event: Event) => onTextChange(id, event.target)
    el.addEventListener('input', handler)
    el.addEventListener('change', handler)
    compatCleanupFns.push(() => {
      el.removeEventListener('input', handler)
      el.removeEventListener('change', handler)
    })
  }

  for (const id of LEGACY_STRATEGY_CHECKBOX_IDS) {
    const el = queryLegacyById<HTMLInputElement>(id)
    if (!el) continue
    const handler = (event: Event) => onCheckboxChange(id, event.target)
    el.addEventListener('input', handler)
    el.addEventListener('change', handler)
    compatCleanupFns.push(() => {
      el.removeEventListener('input', handler)
      el.removeEventListener('change', handler)
    })
  }
}

function resolveMount(): void {
  const panel = document.querySelector('[data-vue-panel="strategy"]') as HTMLElement | null
  const mount = document.querySelector('#strategyVueShellMount') as HTMLElement | null
  if (!panel || !mount) return
  panelRoot.value = panel
  panel.dataset.vueStrategy = '1'
  mountReady.value = true
}

function adoptLegacyStrategyBlocks(): void {
  const panel = panelRoot.value ?? (document.querySelector('[data-vue-panel="strategy"]') as HTMLElement | null)
  const host = document.querySelector('#strategyCompatHostVue') as HTMLElement | null
  if (!panel || !host) return
  const nodes = Array.from(panel.querySelectorAll('.strategyLegacyPassthrough')) as HTMLElement[]
  for (const node of nodes) {
    if (node.parentElement === host) continue
    const parent = node.parentNode
    if (!parent) continue
    adoptedNodes.push({ node, parent, next: node.nextSibling })
    host.appendChild(node)
  }
}

function restoreLegacyStrategyBlocks(): void {
  while (adoptedNodes.length > 0) {
    const entry = adoptedNodes.pop()
    if (!entry) continue
    if (entry.next && entry.next.parentNode === entry.parent) {
      entry.parent.insertBefore(entry.node, entry.next)
    } else {
      entry.parent.appendChild(entry.node)
    }
  }
}

onMounted(async () => {
  await nextTick()
  resolveMount()
  window.setTimeout(() => {
    adoptLegacyStrategyBlocks()
    seedLegacyStrategyInputsToStore()
    syncLegacyStrategyInputsFromStore()
    bindLegacyStrategyInputs()
    syncLegacyStrategyVisibility()
  }, 0)
})

onBeforeUnmount(() => {
  clearCompatListeners()
  if (panelRoot.value) {
    delete panelRoot.value.dataset.vueStrategy
    panelRoot.value = null
  }
  restoreLegacyStrategyBlocks()
})
</script>

<template>
  <Teleport v-if="mountReady" to="#strategyVueShellMount">
    <section class="strategyPanelVueRoot strategyVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="title">Strategy Controls</div>
        <div class="meta mono">{{ stamp }}</div>
      </header>

      <section class="pane">
        <h3>Execution cost model</h3>
        <label for="vueCostPreset">Cost preset</label>
        <select
          id="vueCostPreset"
          :value="costPresetDraft"
          @change="costPresetDraft = String(($event.target as HTMLSelectElement).value || 'custom'); setSelectValue('costPreset', costPresetDraft)"
        >
          <option
            v-for="option in costPresetOptionItems"
            :key="`cost_${option.value}_${option.label}`"
            :value="option.value"
            :disabled="option.disabled"
          >
            {{ option.label }}
          </option>
        </select>

        <div class="rowThree">
          <div>
            <label for="vueFeeBps">Fee (bps/side)</label>
            <input
              id="vueFeeBps"
              type="number"
              min="0"
              step="0.01"
              :value="feeDraft"
              @input="feeDraft = String(($event.target as HTMLInputElement).value || '0'); setInputValue('feeBps', feeDraft)"
            />
          </div>
          <div>
            <label for="vueSpreadBps">Spread (bps full)</label>
            <input
              id="vueSpreadBps"
              type="number"
              min="0"
              step="0.01"
              :value="spreadDraft"
              @input="spreadDraft = String(($event.target as HTMLInputElement).value || '0'); setInputValue('spreadBps', spreadDraft)"
            />
          </div>
          <div>
            <label for="vueSlippageBps">Slippage (bps/side)</label>
            <input
              id="vueSlippageBps"
              type="number"
              min="0"
              step="0.01"
              :value="slippageDraft"
              @input="slippageDraft = String(($event.target as HTMLInputElement).value || '0'); setInputValue('slippageBps', slippageDraft)"
            />
          </div>
        </div>
        <div class="rowThree">
          <div>
            <label for="vueCommission">Total cost (fraction/side)</label>
            <input id="vueCommission" type="text" :value="commission" readonly />
          </div>
          <div>
            <label for="vueEpsRel">Epsilon band (eps_rel)</label>
            <input
              id="vueEpsRel"
              type="number"
              min="0"
              step="0.0001"
              :value="epsDraft"
              @input="epsDraft = String(($event.target as HTMLInputElement).value || '0'); setInputValue('epsRel', epsDraft)"
            />
          </div>
          <div class="muted inlineMeta mono">{{ costHint }}</div>
        </div>
      </section>

      <section class="pane">
        <h3>Risk and sizing</h3>
        <div>
          <label for="vueStartingEquity">Starting equity</label>
          <input
            id="vueStartingEquity"
            type="number"
            min="0.01"
            step="1"
            :value="equityDraft"
            @input="equityDraft = String(($event.target as HTMLInputElement).value || '10000'); setInputValue('startingEquity', equityDraft)"
          />
        </div>

        <div class="rowThree">
          <div>
            <label for="vueStopLossPct">Stop-loss (optional)</label>
            <input
              id="vueStopLossPct"
              type="number"
              min="0"
              step="0.0001"
              :value="slDraft"
              @input="slDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('stopLossPct', slDraft)"
            />
          </div>
          <div>
            <label for="vueTakeProfitPct">Take-profit (optional)</label>
            <input
              id="vueTakeProfitPct"
              type="number"
              min="0"
              step="0.0001"
              :value="tpDraft"
              @input="tpDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('takeProfitPct', tpDraft)"
            />
          </div>
          <div>
            <label for="vueTrailingStopPct">Trailing stop (optional)</label>
            <input
              id="vueTrailingStopPct"
              type="number"
              min="0"
              step="0.0001"
              :value="trailingDraft"
              @input="trailingDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('trailingStopPct', trailingDraft)"
            />
          </div>
        </div>

        <div class="rowThree">
          <div>
            <label for="vueCooldownBars">Cooldown bars</label>
            <input
              id="vueCooldownBars"
              type="number"
              min="0"
              step="1"
              :value="cooldownDraft"
              @input="cooldownDraft = String(($event.target as HTMLInputElement).value || '0'); setInputValue('cooldownBars', cooldownDraft)"
            />
          </div>
          <div>
            <label for="vueSignalPersistenceBars">Signal persistence bars</label>
            <input
              id="vueSignalPersistenceBars"
              type="number"
              min="1"
              step="1"
              :value="persistenceDraft"
              @input="persistenceDraft = String(($event.target as HTMLInputElement).value || '1'); setInputValue('signalPersistenceBars', persistenceDraft)"
            />
          </div>
        </div>
      </section>

      <section class="pane">
        <h3>Constraints and exits</h3>
        <div class="rowTwo" v-if="showDmConstraints">
          <div>
            <label for="vueDmMinTrades">Min trades (optional)</label>
            <input
              id="vueDmMinTrades"
              type="number"
              min="0"
              step="1"
              :value="minTradesDraft"
              @input="minTradesDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('dmConstraintMinTrades', minTradesDraft)"
            />
          </div>
          <div>
            <label for="vueDmMaxDd">Max drawdown (optional)</label>
            <input
              id="vueDmMaxDd"
              type="number"
              min="0"
              step="0.0001"
              :value="maxDdDraft"
              @input="maxDdDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('dmConstraintMaxDd', maxDdDraft)"
            />
          </div>
        </div>

        <div class="rowTwo">
          <div>
            <label for="vueExitMode">Exit mode</label>
            <select
              id="vueExitMode"
              :value="exitModeDraft"
              @change="exitModeDraft = String(($event.target as HTMLSelectElement).value || 'cross'); setSelectValue('exitMode', exitModeDraft)"
            >
              <option
                v-for="option in exitModeOptionItems"
                :key="`exit_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
        </div>

        <div class="rowThree">
          <div v-if="showKaufmanFields">
            <label for="vueKaufmanPeriod">Kaufman period</label>
            <input
              id="vueKaufmanPeriod"
              type="number"
              min="1"
              step="1"
              :value="kaufmanPeriodDraft"
              @input="kaufmanPeriodDraft = String(($event.target as HTMLInputElement).value || '22'); setInputValue('kaufmanStopPeriod', kaufmanPeriodDraft)"
            />
          </div>
          <div v-if="showKaufmanFields">
            <label for="vueKaufmanMult">Kaufman multiplier</label>
            <input
              id="vueKaufmanMult"
              type="number"
              min="0.0001"
              step="0.01"
              :value="kaufmanMultDraft"
              @input="kaufmanMultDraft = String(($event.target as HTMLInputElement).value || '2.0'); setInputValue('kaufmanStopMult', kaufmanMultDraft)"
            />
          </div>
          <div v-if="showKaufmanFields">
            <label for="vueKaufmanType">Kaufman MA type</label>
            <input
              id="vueKaufmanType"
              type="text"
              :value="kaufmanTypeDraft"
              @input="kaufmanTypeDraft = String(($event.target as HTMLInputElement).value || 'sma'); setInputValue('kaufmanStopMaType', kaufmanTypeDraft)"
            />
          </div>
        </div>
      </section>

      <section class="pane">
        <h3>Strategy-specific filters</h3>
        <div id="strategyCompatHostVue"></div>
      </section>

      <div class="status mono">{{ statusLine }}</div>
    </section>
  </Teleport>
</template>

<style scoped>
.strategyPanelVueRoot {
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

.rowThree {
  margin-top: 0.6rem;
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.6rem;
}

.rowTwo {
  margin-top: 0.6rem;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.6rem;
}

.inlineMeta {
  align-self: end;
}

.status {
  font-size: 0.78rem;
  color: var(--muted);
}

.muted {
  color: var(--muted);
  font-size: 0.8rem;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 980px) {
  .rowThree,
  .rowTwo {
    grid-template-columns: 1fr;
  }
}
</style>

<style>
[data-vue-panel='strategy'][data-vue-strategy='1'] > :not(#strategyVueShellMount) {
  display: none !important;
}
</style>

