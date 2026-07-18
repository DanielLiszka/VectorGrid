export function num(value: unknown, fallback: number): number {
  if (value === null || value === undefined) return fallback
  if (typeof value === "string" && value.trim() === "") return fallback
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : fallback
}

export interface ParsedU16LikeRange {
  low: number
  high: number
  step: number
  count: number
}

export function parseU16LikeRange(start: unknown, end: unknown, step: unknown): ParsedU16LikeRange | null {
  const startInt = Math.floor(Number(start))
  const endInt = Math.floor(Number(end))
  const stepInt = Math.floor(Number(step))
  if (!Number.isFinite(startInt) || !Number.isFinite(endInt) || !Number.isFinite(stepInt)) {
    return null
  }
  if (stepInt === 0 || startInt === endInt) {
    return {
      low: startInt,
      high: startInt,
      step: 1,
      count: 1,
    }
  }

  const normalizedStep = Math.max(1, stepInt)
  const low = Math.min(startInt, endInt)
  const high = Math.max(startInt, endInt)
  const count = Math.max(1, Math.floor((high - low) / normalizedStep) + 1)
  return {
    low,
    high,
    step: normalizedStep,
    count,
  }
}

export function expandRangeU16Like(start: unknown, end: unknown, step: unknown): number[] {
  const parsed = parseU16LikeRange(start, end, step)
  if (!parsed) return []

  const out: number[] = []
  let current = parsed.low
  for (;;) {
    out.push(current)
    if (current === parsed.high) break
    const next = current + parsed.step
    if (next <= current || next > parsed.high) break
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

export function countPairsFastLtSlowRanges(
  fast: ParsedU16LikeRange | null,
  slow: ParsedU16LikeRange | null,
): number {
  if (!fast || !slow || fast.count <= 0 || slow.count <= 0) return 0

  let total = 0
  let fastValue = fast.low
  for (let index = 0; index < fast.count; index += 1) {
    if (fastValue < slow.high) {
      const minSlowValue = Math.max(slow.low, fastValue + 1)
      const firstSlowStep = Math.ceil((minSlowValue - slow.low) / slow.step)
      const firstSlowValue = slow.low + (Math.max(0, firstSlowStep) * slow.step)
      if (firstSlowValue <= slow.high) {
        total += Math.floor((slow.high - firstSlowValue) / slow.step) + 1
      }
    }
    fastValue += fast.step
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
