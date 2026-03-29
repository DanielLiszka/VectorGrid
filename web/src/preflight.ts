import {
  CONTROLS_COLLAPSED_KEY,
  CONTROLS_WIDTH_KEY,
  DENSITY_MODE_KEY,
  HELP_MODE_KEY,
  THEME_PREF_KEY,
  WINDOW_EFFECT_KEY,
  WORKFLOW_ADVANCED_KEY,
  WORKFLOW_TAB_KEY,
  WORKSPACE_TAB_KEY,
} from './utils/core/storageKeys'

type ThemePref = 'light' | 'dark' | 'system'
type HelpMode = 'full' | 'concise' | 'off'
type DensityMode = 'comfortable' | 'standard' | 'dense'

const THEME_VALUES = new Set<ThemePref>(['light', 'dark', 'system'])
const HELP_MODE_VALUES = new Set<HelpMode>(['full', 'concise', 'off'])
const DENSITY_MODE_VALUES = new Set<DensityMode>(['comfortable', 'standard', 'dense'])
const WORKFLOW_VALUES = new Set(['data', 'optimize', 'validate', 'analyze', 'history', 'settings'])
const WORKSPACE_VALUES = new Set(['workflow', 'library', 'aggregate', 'pro', 'settings'])
const TRUE_VALUES = new Set(['1', 'true', 'yes', 'on'])

function readStorageString(key: string): string {
  try {
    return String(localStorage.getItem(key) ?? '').trim()
  } catch {
    return ''
  }
}

function setRootDataAttr(name: string, value: string): void {
  const root = document.documentElement
  if (value) root.dataset[name as keyof DOMStringMap] = value
  else delete root.dataset[name as keyof DOMStringMap]
}

function applyThemePreflight(): void {
  const raw = readStorageString(THEME_PREF_KEY).toLowerCase()
  const pref: ThemePref = THEME_VALUES.has(raw as ThemePref) ? (raw as ThemePref) : 'light'
  setRootDataAttr('themePref', pref)
  const media = window.matchMedia?.('(prefers-color-scheme: dark)')
  const theme = pref === 'system' ? (media?.matches ? 'dark' : 'light') : pref
  setRootDataAttr('theme', theme)
}

function applyControlsWidthPreflight(): void {
  const width = Number(readStorageString(CONTROLS_WIDTH_KEY))
  if (Number.isFinite(width) && width >= 280 && width <= 820) {
    document.documentElement.style.setProperty('--controls-w', `${Math.round(width)}px`)
  }
}

function applyBooleanDatasetPreflight(key: string, datasetKey: string): void {
  const raw = readStorageString(key).toLowerCase()
  setRootDataAttr(datasetKey, TRUE_VALUES.has(raw) ? '1' : '')
}

function applyWindowEffectPreflight(): void {
  const effect = readStorageString(WINDOW_EFFECT_KEY).toLowerCase()
  if (!effect || effect === 'none') {
    setRootDataAttr('windowEffect', '')
    return
  }
  const ua = navigator.userAgent.toLowerCase()
  let allowed = new Set(['none'])
  if (ua.includes('windows')) allowed = new Set(['none', 'mica', 'acrylic', 'blur'])
  else if (ua.includes('mac')) allowed = new Set(['none', 'vibrancy'])
  setRootDataAttr('windowEffect', allowed.has(effect) ? effect : '')
}

function applyWorkflowPreflight(): void {
  const workflow = readStorageString(WORKFLOW_TAB_KEY).toLowerCase()
  const workspace = readStorageString(WORKSPACE_TAB_KEY).toLowerCase()
  setRootDataAttr('workflow', WORKFLOW_VALUES.has(workflow) ? workflow : '')
  setRootDataAttr('workspace', WORKSPACE_VALUES.has(workspace) ? workspace : '')
}

function applyDensityAndHelpPreflight(): void {
  const density = readStorageString(DENSITY_MODE_KEY).toLowerCase()
  const helpMode = readStorageString(HELP_MODE_KEY).toLowerCase()
  setRootDataAttr('density', DENSITY_MODE_VALUES.has(density as DensityMode) ? density : '')
  setRootDataAttr('helpMode', HELP_MODE_VALUES.has(helpMode as HelpMode) ? helpMode : '')
}

applyThemePreflight()
applyControlsWidthPreflight()
applyBooleanDatasetPreflight(CONTROLS_COLLAPSED_KEY, 'controlsCollapsed')
applyBooleanDatasetPreflight(WORKFLOW_ADVANCED_KEY, 'workflowAdvanced')
applyWindowEffectPreflight()
applyWorkflowPreflight()
applyDensityAndHelpPreflight()
