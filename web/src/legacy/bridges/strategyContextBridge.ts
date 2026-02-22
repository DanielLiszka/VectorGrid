import { useStrategyStore } from '../../stores/strategyStore'
import { createCoalescedSync } from './coalescedSync'

const STRATEGY_INPUT_IDS = [
  'costPreset',
  'feeBps',
  'spreadBps',
  'slippageBps',
  'commission',
  'epsRel',
  'startingEquity',
  'stopLossPct',
  'takeProfitPct',
  'cooldownBars',
  'signalPersistenceBars',
  'trailingStopPct',
  'dmConstraintMinTrades',
  'dmConstraintMaxDd',
  'exitMode',
  'kaufmanStopPeriod',
  'kaufmanStopMult',
  'kaufmanStopMaType',
] as const

const STRATEGY_TEXT_IDS = ['costHint', 'status'] as const
const STRATEGY_SNAPSHOT_EVENT = 'vta:strategy-snapshot'

function valueById(id: string): string {
  const el = document.getElementById(id) as HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement | null
  return String(el?.value || '').trim()
}

function textById(id: string): string {
  const el = document.getElementById(id)
  return String(el?.textContent || '').trim()
}

function activeControlsTab(): string {
  const rootTab = String(document.documentElement.dataset.controlsTab || '').trim()
  return rootTab || 'data'
}

function optionsById(id: string): { value: string; label: string; disabled: boolean }[] {
  const el = document.getElementById(id) as HTMLSelectElement | null
  if (!el) return []
  return Array.from(el.options)
    .map((option) => {
      const value = String(option.value || '').trim()
      const label = String(option.textContent || option.value || '').trim()
      return {
        value: value || label,
        label: label || value,
        disabled: !!option.disabled,
      }
    })
    .filter((entry) => entry.value.length > 0 || entry.label.length > 0)
}

function isVisibleById(id: string): boolean {
  const el = document.getElementById(id) as HTMLElement | null
  if (!el) return false
  return window.getComputedStyle(el).display !== 'none'
}

function observeTextNodes(ids: readonly string[], onChange: () => void): MutationObserver {
  const observer = new MutationObserver(() => onChange())
  for (const id of ids) {
    const node = document.getElementById(id)
    if (!node) continue
    observer.observe(node, {
      childList: true,
      characterData: true,
      subtree: true,
    })
  }
  return observer
}

export function attachStrategyContextBridge(): () => void {
  const store = useStrategyStore()

  const syncNow = () => {
    store.applyLegacySnapshot({
      activeControlsTab: activeControlsTab(),
      costPreset: valueById('costPreset') || 'custom',
      costPresetOptions: optionsById('costPreset'),
      feeBps: valueById('feeBps'),
      spreadBps: valueById('spreadBps'),
      slippageBps: valueById('slippageBps'),
      commission: valueById('commission'),
      epsRel: valueById('epsRel'),
      costHint: textById('costHint') || 'Total: —',
      startingEquity: valueById('startingEquity') || '10000',
      stopLossPct: valueById('stopLossPct'),
      takeProfitPct: valueById('takeProfitPct'),
      cooldownBars: valueById('cooldownBars') || '0',
      signalPersistenceBars: valueById('signalPersistenceBars') || '1',
      trailingStopPct: valueById('trailingStopPct'),
      dmConstraintMinTrades: valueById('dmConstraintMinTrades'),
      dmConstraintMaxDd: valueById('dmConstraintMaxDd'),
      dmConstraintsVisible: isVisibleById('dmConstraintsWrap'),
      exitMode: valueById('exitMode') || 'cross',
      exitModeOptions: optionsById('exitMode'),
      kaufmanStopPeriod: valueById('kaufmanStopPeriod') || '22',
      kaufmanStopMult: valueById('kaufmanStopMult') || '2.0',
      kaufmanStopMaType: valueById('kaufmanStopMaType') || 'sma',
      kaufmanStopPeriodVisible: isVisibleById('kaufmanStopPeriodWrap'),
      kaufmanStopMultVisible: isVisibleById('kaufmanStopMultWrap'),
      kaufmanStopMaTypeVisible: isVisibleById('kaufmanStopMaTypeWrap'),
      statusLine: textById('status') || 'Ready.',
    })
  }
  const scheduleSync = createCoalescedSync(syncNow)

  const onInputLike = () => scheduleSync()
  const attachedInputs: (HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement)[] = []
  for (const id of STRATEGY_INPUT_IDS) {
    const el = document.getElementById(id) as HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement | null
    if (!el) continue
    attachedInputs.push(el)
    el.addEventListener('input', onInputLike)
    el.addEventListener('change', onInputLike)
  }

  const textObserver = observeTextNodes(STRATEGY_TEXT_IDS, scheduleSync)
  const rootTabObserver = new MutationObserver(() => scheduleSync())
  rootTabObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-controls-tab'],
  })

  const visibilityObserver = new MutationObserver(() => scheduleSync())
  for (const id of ['dmConstraintsWrap', 'kaufmanStopPeriodWrap', 'kaufmanStopMultWrap', 'kaufmanStopMaTypeWrap'] as const) {
    const node = document.getElementById(id)
    if (!node) continue
    visibilityObserver.observe(node, {
      attributes: true,
      attributeFilter: ['style', 'class'],
    })
  }

  const onStrategySnapshot = (): void => scheduleSync()
  window.addEventListener(STRATEGY_SNAPSHOT_EVENT, onStrategySnapshot)

  syncNow()

  return () => {
    textObserver.disconnect()
    rootTabObserver.disconnect()
    visibilityObserver.disconnect()
    window.removeEventListener(STRATEGY_SNAPSHOT_EVENT, onStrategySnapshot)
    for (const el of attachedInputs) {
      el.removeEventListener('input', onInputLike)
      el.removeEventListener('change', onInputLike)
    }
  }
}
