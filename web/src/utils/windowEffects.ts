export type WindowEffect = 'none' | 'mica' | 'acrylic' | 'blur' | 'vibrancy'

type WindowEffectPlatform = 'windows' | 'macos' | 'other'

const WINDOWS_EFFECTS: readonly WindowEffect[] = ['none', 'mica', 'acrylic', 'blur']
const MACOS_EFFECTS: readonly WindowEffect[] = ['none', 'vibrancy']
const OTHER_EFFECTS: readonly WindowEffect[] = ['none']

const WINDOW_EFFECT_LABELS: Record<WindowEffect, string> = {
  none: 'None',
  mica: 'Mica',
  acrylic: 'Acrylic',
  blur: 'Blur',
  vibrancy: 'Vibrancy',
}

function currentUserAgent(): string {
  try {
    return String(navigator.userAgent || '').trim()
  } catch {
    return ''
  }
}

export function detectWindowEffectPlatform(userAgent = currentUserAgent()): WindowEffectPlatform {
  const normalized = String(userAgent || '').trim().toLowerCase()
  if (normalized.includes('windows')) return 'windows'
  if (normalized.includes('mac')) return 'macos'
  return 'other'
}

export function listSupportedWindowEffects(userAgent = currentUserAgent()): WindowEffect[] {
  const platform = detectWindowEffectPlatform(userAgent)
  if (platform === 'windows') return [...WINDOWS_EFFECTS]
  if (platform === 'macos') return [...MACOS_EFFECTS]
  return [...OTHER_EFFECTS]
}

export function normalizeWindowEffect(value: unknown, userAgent = currentUserAgent()): WindowEffect {
  const next = String(value ?? '').trim().toLowerCase() as WindowEffect
  return listSupportedWindowEffects(userAgent).includes(next) ? next : 'none'
}

export function readSupportedWindowEffect(value: unknown, userAgent = currentUserAgent()): '' | Exclude<WindowEffect, 'none'> {
  const effect = normalizeWindowEffect(value, userAgent)
  return effect === 'none' ? '' : effect
}

export function windowEffectLabel(effect: WindowEffect): string {
  return WINDOW_EFFECT_LABELS[effect]
}
