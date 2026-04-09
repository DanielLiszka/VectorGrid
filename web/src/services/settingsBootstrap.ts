import {
  CONTROLS_COLLAPSED_KEY,
  DESKTOP_NOTIFICATION_MODE_KEY,
  DENSITY_MODE_KEY,
  HELP_MODE_KEY,
  THEME_PREF_KEY,
  WINDOW_EFFECT_KEY,
  WORKFLOW_ADVANCED_KEY,
} from '../utils/core/storageKeys'
import { readStorageString } from '../utils/storage'
import { resolveRuntimePinia } from '../stores/runtimePinia'
import { useSettingsStore } from '../stores/settingsStore'
import { readSupportedWindowEffect } from '../utils/windowEffects'

const THEME_KEYS = new Set(['light', 'dark', 'system'])
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
  return readStorageString(key, '').trim()
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
    effect: readSupportedWindowEffect(readStorageValue(WINDOW_EFFECT_KEY)),
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
  store.applyBootstrapState(state)
}
