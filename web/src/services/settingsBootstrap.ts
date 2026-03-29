import {
  CONTROLS_COLLAPSED_KEY,
  DESKTOP_NOTIFICATION_MODE_KEY,
  DENSITY_MODE_KEY,
  HELP_MODE_KEY,
  THEME_PREF_KEY,
  WINDOW_EFFECT_KEY,
  WORKFLOW_ADVANCED_KEY,
} from '../utils/core/storageKeys'
import { resolveRuntimePinia } from '../stores/runtimePinia'
import { useSettingsStore } from '../stores/settingsStore'

const THEME_KEYS = new Set(['light', 'dark', 'system'])
const EFFECT_KEYS = new Set(['none', 'mica', 'acrylic', 'vibrancy'])
const HELP_MODE_KEYS = new Set(['full', 'concise', 'off'])
const DENSITY_MODE_KEYS = new Set(['comfortable', 'standard', 'dense'])
const TRUE_VALUES = new Set(['1', 'true', 'yes', 'on'])
const FALSE_VALUES = new Set(['0', 'false', 'no', 'off'])

export interface SettingsBootstrapState {
  theme: string
  effect: string
  helpMode: string
  densityMode: string
  desktopNotificationMode: string
  workflowAdvanced: boolean | null
  controlsCollapsed: boolean | null
}

function normalize(value: unknown): string {
  return String(value ?? '').trim().toLowerCase()
}

function normalizeFromSet(value: unknown, allowed: Set<string>): string {
  const next = normalize(value)
  return allowed.has(next) ? next : ''
}

function readStorageValue(key: string): string {
  try {
    return String(localStorage.getItem(key) ?? '').trim()
  } catch {
    return ''
  }
}

function readBooleanFlag(key: string): boolean | null {
  const raw = normalize(readStorageValue(key))
  if (!raw) return null
  if (TRUE_VALUES.has(raw)) return true
  if (FALSE_VALUES.has(raw)) return false
  return null
}

export function readSettingsBootstrapState(): SettingsBootstrapState {
  return {
    theme: normalizeFromSet(readStorageValue(THEME_PREF_KEY), THEME_KEYS),
    effect: normalizeFromSet(readStorageValue(WINDOW_EFFECT_KEY), EFFECT_KEYS),
    helpMode: normalizeFromSet(readStorageValue(HELP_MODE_KEY), HELP_MODE_KEYS),
    densityMode: normalizeFromSet(readStorageValue(DENSITY_MODE_KEY), DENSITY_MODE_KEYS),
    desktopNotificationMode: normalizeFromSet(
      readStorageValue(DESKTOP_NOTIFICATION_MODE_KEY),
      new Set(['off', 'failures', 'important']),
    ),
    workflowAdvanced: readBooleanFlag(WORKFLOW_ADVANCED_KEY),
    controlsCollapsed: readBooleanFlag(CONTROLS_COLLAPSED_KEY),
  }
}

export function applySettingsBootstrapFromStorage(): void {
  const state = readSettingsBootstrapState()
  if (
    !state.theme
    && !state.effect
    && !state.helpMode
    && !state.densityMode
    && !state.desktopNotificationMode
    && state.workflowAdvanced == null
    && state.controlsCollapsed == null
  ) return
  const pinia = resolveRuntimePinia()
  if (!pinia) return
  const store = useSettingsStore(pinia)

  if (state.theme) {
    store.setSelectField('settingsThemeSelect', state.theme)
  }

  if (state.effect) {
    store.setSelectField('settingsEffectSelect', state.effect)
  }

  if (state.helpMode) {
    store.setSelectField('settingsHelpMode', state.helpMode)
  }

  if (state.densityMode) {
    store.setSelectField('settingsDensityMode', state.densityMode)
  }

  if (state.desktopNotificationMode) {
    store.setSelectField('settingsDesktopNotifications', state.desktopNotificationMode)
  }

  if (state.workflowAdvanced != null) {
    store.setCheckboxField('settingsWorkflowAdvanced', state.workflowAdvanced)
  }

  if (state.controlsCollapsed != null) {
    store.setCheckboxField('settingsControlsCollapsed', state.controlsCollapsed)
  }
}
