export function num(value: unknown, fallback: number): number {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : fallback
}

export function expandRangeU16Like(start: unknown, end: unknown, step: unknown): number[] {
  const startInt = Math.floor(Number(start))
  const endInt = Math.floor(Number(end))
  const stepInt = Math.floor(Number(step))
  if (!Number.isFinite(startInt) || !Number.isFinite(endInt) || !Number.isFinite(stepInt)) return []

  if (stepInt === 0 || startInt === endInt) return [startInt]
  const normalizedStep = Math.max(1, stepInt)
  const low = Math.min(startInt, endInt)
  const high = Math.max(startInt, endInt)
  const out: number[] = []
  let current = low
  for (;;) {
    out.push(current)
    if (current === high) break
    const next = current + normalizedStep
    if (next <= current || next > high) break
    current = next
  }
  return out
}

export function countPairsFastLtSlow(fastPeriods: unknown, slowPeriods: unknown): number {
  const fast = Array.isArray(fastPeriods) ? fastPeriods : []
  const slow = Array.isArray(slowPeriods) ? slowPeriods : []
  if (!fast.length || !slow.length) return 0

  const slowSorted = [...slow]
    .map((value) => Number(value))
    .filter((value) => Number.isFinite(value))
    .sort((left, right) => left - right)
  if (!slowSorted.length) return 0

  let total = 0
  for (const rawFast of fast) {
    const fastValue = Number(rawFast)
    if (!Number.isFinite(fastValue)) continue
    let low = 0
    let high = slowSorted.length
    while (low < high) {
      const mid = (low + high) >> 1
      if ((slowSorted[mid] ?? Number.POSITIVE_INFINITY) <= fastValue) low = mid + 1
      else high = mid
    }
    total += slowSorted.length - low
  }
  return total
}

export function countF64RangeValues(start: unknown, end: unknown, step: unknown): number {
  const a = Number(start)
  const b = Number(end)
  const s = Number(step)
  if (!Number.isFinite(a) || !Number.isFinite(b) || !Number.isFinite(s)) return 1
  if (s === 0 || a === b) return 1
  const low = Math.min(a, b)
  const high = Math.max(a, b)
  const absStep = Math.abs(s)
  if (!(absStep > 0) || !Number.isFinite(absStep)) return 1
  const count = Math.floor((high - low) / absStep) + 1
  return Math.max(1, Math.min(1_000_000, count))
}
