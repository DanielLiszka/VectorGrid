interface CandleMeta {
  has_open?: unknown
  has_high?: unknown
  has_low?: unknown
  has_close?: unknown
  has_volume?: unknown
}

export function maRequiresVolumeField(maId: unknown): boolean {
  const id = String(maId || '').trim().toLowerCase()
  return id === 'vwma' || id === 'vpwma'
}

export function maRequiresHighLowField(maId: unknown): boolean {
  const id = String(maId || '').trim().toLowerCase()
  return id === 'frama'
}

export function requiredFieldsForMaSource(source: unknown): string[] {
  const normalized = String(source || '').trim().toLowerCase()
  if (normalized === 'close') return ['close']
  if (normalized === 'open') return ['open']
  if (normalized === 'high') return ['high']
  if (normalized === 'low') return ['low']
  if (normalized === 'hl2') return ['high', 'low']
  if (normalized === 'hlc3') return ['high', 'low', 'close']
  if (normalized === 'ohlc4') return ['open', 'high', 'low', 'close']
  if (normalized === 'hlcc' || normalized === 'hlcc4') return ['high', 'low', 'close']
  return []
}

export function validateCandleFieldRequirements(
  metaValue: unknown,
  fastMaType: unknown,
  slowMaType: unknown,
  maSource: unknown,
): void {
  if (!metaValue || typeof metaValue !== 'object') return
  const meta = metaValue as CandleMeta

  const missing = new Set<string>()
  const requireField = (field: string) => {
    if (field === 'open' && !meta.has_open) missing.add('open')
    if (field === 'high' && !meta.has_high) missing.add('high')
    if (field === 'low' && !meta.has_low) missing.add('low')
    if (field === 'close' && !meta.has_close) missing.add('close')
    if (field === 'volume' && !meta.has_volume) missing.add('volume')
  }

  if (maRequiresVolumeField(fastMaType) || maRequiresVolumeField(slowMaType)) requireField('volume')
  if (maRequiresHighLowField(fastMaType) || maRequiresHighLowField(slowMaType)) {
    requireField('high')
    requireField('low')
  }
  for (const field of requiredFieldsForMaSource(maSource)) requireField(field)

  if (missing.size === 0) return
  const parts: string[] = []
  if (missing.has('volume')) parts.push('volume (needed by VWMA/VPWMA)')
  if (missing.has('high') || missing.has('low')) parts.push('high/low (needed by FRAMA and some sources)')
  if (missing.has('open')) parts.push('open (needed by some sources)')
  if (missing.has('close')) parts.push('close')
  throw new Error(`Loaded CSV is missing required candle fields: ${parts.join(', ')}.`)
}

export function fieldPresent(metaValue: unknown, field: string): boolean {
  if (!metaValue || typeof metaValue !== 'object') return false
  const meta = metaValue as CandleMeta
  if (field === 'open') return !!meta.has_open
  if (field === 'high') return !!meta.has_high
  if (field === 'low') return !!meta.has_low
  if (field === 'close') return !!meta.has_close
  if (field === 'volume') return !!meta.has_volume
  return false
}
