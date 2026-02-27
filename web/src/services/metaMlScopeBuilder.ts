function normalizeText(value: unknown): string {
  return String(value ?? '').trim()
}

export interface MetaMlScopeBuildInputs {
  strategyKind?: unknown
  timeframe?: unknown
}

export function buildMetaMlScope(inputs: MetaMlScopeBuildInputs): Record<string, unknown> {
  const strategyKind = normalizeText(inputs.strategyKind || 'CompositeMo') || 'CompositeMo'
  const timeframe = normalizeText(inputs.timeframe)
  return {
    strategy_kind: strategyKind,
    timeframe: timeframe || null,
  }
}
