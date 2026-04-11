export interface ExecutionRiskNotice {
  title: string
  body: string
}

function readBoolean(value: unknown, fallback: boolean): boolean {
  if (typeof value === 'boolean') return value
  const raw = String(value ?? '').trim().toLowerCase()
  if (!raw) return fallback
  if (raw === 'true' || raw === '1' || raw === 'yes' || raw === 'on') return true
  if (raw === 'false' || raw === '0' || raw === 'no' || raw === 'off') return false
  return fallback
}

export function describeExecutionRisk(strategy: unknown): ExecutionRiskNotice | null {
  const row = strategy && typeof strategy === 'object' ? (strategy as Record<string, unknown>) : {}
  const longOnly = readBoolean(row.long_only ?? row.longOnly, true)
  const tradeOnNext = readBoolean(row.trade_on_next_bar ?? row.tradeOnNext, true)

  if (!tradeOnNext && !longOnly) {
    return {
      title: 'Optimistic execution mode',
      body: 'Trade on next bar is off and short exposure is enabled. Same-bar execution can materially overstate backtest performance, so treat these runs as exploratory until they are rechecked with next-bar execution.',
    }
  }

  if (!tradeOnNext) {
    return {
      title: 'Same-bar execution enabled',
      body: 'Trade on next bar is off. Signals execute on the same bar that generated them, which can materially overstate backtest performance versus next-bar execution.',
    }
  }

  if (!longOnly) {
    return {
      title: 'Short exposure enabled',
      body: 'Short exposure is enabled. Review borrow, financing, and execution assumptions before trusting raw compounded return metrics.',
    }
  }

  return null
}
