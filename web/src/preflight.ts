import {
  CONTROLS_COLLAPSED_KEY,
  CONTROLS_WIDTH_KEY,
  DENSITY_MODE_KEY,
  HELP_MODE_KEY,
  THEME_PREF_KEY,
  UI_MODE_KEY,
  WINDOW_EFFECT_KEY,
  WORKFLOW_ADVANCED_KEY,
  WORKFLOW_TAB_KEY,
  WORKSPACE_TAB_KEY,
} from './utils/core/storageKeys'
import { readStorageString as readSharedStorageString } from './utils/storage'
import { applyRootThemePreference, normalizeThemePreference } from './utils/themeController'
import { normalizeWindowEffect } from './utils/windowEffects'

type HelpMode = 'full' | 'concise' | 'off'
type DensityMode = 'comfortable' | 'standard' | 'dense'
type UiMode = 'advanced'

const HELP_MODE_VALUES = new Set<HelpMode>(['full', 'concise', 'off'])
const DENSITY_MODE_VALUES = new Set<DensityMode>(['comfortable', 'standard', 'dense'])
const UI_MODE_VALUES = new Set<UiMode>(['advanced'])
const DEFAULT_UI_MODE: UiMode = 'advanced'
const WORKFLOW_VALUES = new Set(['data', 'optimize', 'validate', 'analyze', 'finalize', 'history', 'settings'])
const WORKSPACE_VALUES = new Set(['workflow', 'library', 'aggregate', 'pro', 'live', 'settings'])
const TRUE_VALUES = new Set(['1', 'true', 'yes', 'on'])

function readStorageString(key: string): string {
  return readSharedStorageString(key, '').trim()
}

function setRootDataAttr(name: string, value: string): void {
  const root = document.documentElement
  if (value) root.dataset[name as keyof DOMStringMap] = value
  else delete root.dataset[name as keyof DOMStringMap]
}

function applyThemePreflight(): void {
  const raw = readStorageString(THEME_PREF_KEY).toLowerCase()
  applyRootThemePreference(normalizeThemePreference(raw))
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
  const effect = normalizeWindowEffect(readStorageString(WINDOW_EFFECT_KEY))
  if (effect === 'none') {
    setRootDataAttr('windowEffect', '')
    return
  }
  setRootDataAttr('windowEffect', effect)
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

function applyUiModePreflight(): void {
  const uiMode = readStorageString(UI_MODE_KEY).toLowerCase()
  setRootDataAttr('uiMode', UI_MODE_VALUES.has(uiMode as UiMode) ? uiMode : DEFAULT_UI_MODE)
}

applyThemePreflight()
applyControlsWidthPreflight()
applyBooleanDatasetPreflight(CONTROLS_COLLAPSED_KEY, 'controlsCollapsed')
applyBooleanDatasetPreflight(WORKFLOW_ADVANCED_KEY, 'workflowAdvanced')
applyWindowEffectPreflight()
applyWorkflowPreflight()
applyDensityAndHelpPreflight()
applyUiModePreflight()
