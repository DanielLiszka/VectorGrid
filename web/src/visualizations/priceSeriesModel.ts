export interface PriceBar {
  time: number;
  open: number;
  high: number;
  low: number;
  close: number;
}

export interface IndexedPriceBar extends PriceBar {
  volume: number;
  fromIndex: number;
  toIndex: number;
}

export interface MovingAverageIndicatorBar {
  time: number;
  fast: number | null;
  slow: number | null;
}

export interface CompositeIndicatorBar {
  time: number;
  halftrend: number | null;
  macz: number | null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function toNumber(value: unknown, fallback = Number.NaN): number {
  const parsed = typeof value === "number" ? value : Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function toOptionalNumber(value: unknown): number | null {
  if (value == null || value === "") return null;
  const parsed = typeof value === "number" ? value : Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

export function normalizeChartTimeSec(raw: unknown): number {
  const value = toNumber(raw);
  if (!Number.isFinite(value)) return Number.NaN;
  const abs = Math.abs(value);
  if (abs >= 1e17) return Math.floor(value / 1e9);
  if (abs >= 1e14) return Math.floor(value / 1e6);
  if (abs >= 1e11) return Math.floor(value / 1e3);
  return Math.floor(value);
}

function normalizePriceBarBase(row: Record<string, unknown>): PriceBar | null {
  const time = normalizeChartTimeSec(
    row.time ?? row.timestamp ?? row.ts ?? row.t,
  );
  const open = toNumber(row.open ?? row.o);
  const highRaw = toNumber(row.high ?? row.h);
  const lowRaw = toNumber(row.low ?? row.l);
  const close = toNumber(row.close ?? row.c);
  if (
    !Number.isFinite(time) ||
    !Number.isFinite(open) ||
    !Number.isFinite(highRaw) ||
    !Number.isFinite(lowRaw) ||
    !Number.isFinite(close)
  ) {
    return null;
  }
  return {
    time,
    open,
    high: Math.max(highRaw, open, close),
    low: Math.min(lowRaw, open, close),
    close,
  };
}

function dedupeSortedBars<T extends PriceBar>(bars: T[]): T[] {
  if (bars.length <= 1) return bars;
  let sorted = true;
  let hasDuplicateTime = false;
  for (let index = 1; index < bars.length; index += 1) {
    const previousTime = bars[index - 1]?.time ?? 0;
    const nextTime = bars[index]?.time ?? 0;
    if (previousTime === nextTime) hasDuplicateTime = true;
    if (previousTime <= nextTime) continue;
    sorted = false;
    break;
  }
  if (sorted && !hasDuplicateTime) return bars;
  if (!sorted) bars.sort((a, b) => a.time - b.time);
  const deduped: T[] = [];
  for (const row of bars) {
    const last = deduped[deduped.length - 1];
    if (last && row.time === last.time) deduped[deduped.length - 1] = row;
    else deduped.push(row);
  }
  return deduped;
}

export function normalizePriceBars(raw: unknown): PriceBar[] {
  const rows = Array.isArray(raw) ? raw : [];
  const out: PriceBar[] = [];
  for (const rowRaw of rows) {
    if (!isRecord(rowRaw)) continue;
    const bar = normalizePriceBarBase(rowRaw);
    if (bar) out.push(bar);
  }
  return dedupeSortedBars(out);
}

export function normalizeIndexedPriceBars(raw: unknown): IndexedPriceBar[] {
  const rows = Array.isArray(raw) ? raw : [];
  const out: IndexedPriceBar[] = [];
  for (const rowRaw of rows) {
    if (!isRecord(rowRaw)) continue;
    const bar = normalizePriceBarBase(rowRaw);
    if (!bar) continue;
    const fromIndex = Math.max(
      0,
      Math.floor(
        toNumber(rowRaw.from_index ?? rowRaw.fromIndex ?? rowRaw.from, 0),
      ),
    );
    const toIndex = Math.max(
      fromIndex,
      Math.floor(
        toNumber(rowRaw.to_index ?? rowRaw.toIndex ?? rowRaw.to, fromIndex),
      ),
    );
    out.push({
      ...bar,
      volume: toNumber(rowRaw.volume ?? rowRaw.v, 0),
      fromIndex,
      toIndex,
    });
  }
  return dedupeSortedBars(out);
}

export function normalizeMovingAverageIndicatorBars(
  raw: unknown,
): MovingAverageIndicatorBar[] {
  const rows = Array.isArray(raw) ? raw : [];
  const out: MovingAverageIndicatorBar[] = [];
  let sorted = true;
  for (const rowRaw of rows) {
    if (!isRecord(rowRaw)) continue;
    const time = normalizeChartTimeSec(
      rowRaw.time ?? rowRaw.timestamp ?? rowRaw.ts ?? rowRaw.t,
    );
    if (!Number.isFinite(time)) continue;
    const fast = toOptionalNumber(rowRaw.fast);
    const slow = toOptionalNumber(rowRaw.slow);
    if (fast == null && slow == null) continue;
    const previous = out[out.length - 1];
    if (previous && previous.time > time) sorted = false;
    out.push({ time, fast, slow });
  }
  if (!sorted) out.sort((a, b) => a.time - b.time);
  return out;
}

export function normalizeCompositeIndicatorBars(
  raw: unknown,
): CompositeIndicatorBar[] {
  const rows = Array.isArray(raw) ? raw : [];
  const out: CompositeIndicatorBar[] = [];
  let sorted = true;
  for (const rowRaw of rows) {
    if (!isRecord(rowRaw)) continue;
    const time = normalizeChartTimeSec(
      rowRaw.time ?? rowRaw.timestamp ?? rowRaw.ts ?? rowRaw.t,
    );
    if (!Number.isFinite(time)) continue;
    const halftrend = toOptionalNumber(rowRaw.halftrend);
    const macz = toOptionalNumber(rowRaw.macz_hist ?? rowRaw.macz);
    if (halftrend == null && macz == null) continue;
    const previous = out[out.length - 1];
    if (previous && previous.time > time) sorted = false;
    out.push({ time, halftrend, macz });
  }
  if (!sorted) out.sort((a, b) => a.time - b.time);
  return out;
}
