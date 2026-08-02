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
  ottLine: number | null;
  ottPrice: number | null;
}

export type StrategyIndicatorAggregation = "last_finite";
export type StrategyIndicatorSeriesKind = "line" | "histogram";
export type StrategyIndicatorSeriesUnit =
  | "price"
  | "price_delta"
  | "number"
  | "ratio"
  | "degrees"
  | "state"
  | "z_score"
  | "oscillator";

export interface StrategyIndicatorBar {
  time: number;
  fromIndex: number;
  toIndex: number;
}

export interface StrategyIndicatorValuePoint {
  time: number;
  value: number;
}

export interface StrategyIndicatorWhitespacePoint {
  time: number;
}

export type StrategyIndicatorPoint =
  | StrategyIndicatorValuePoint
  | StrategyIndicatorWhitespacePoint;

export interface StrategyIndicatorReferenceLine {
  value: number;
  label: string;
}

export interface StrategyIndicatorSeriesModel {
  id: string;
  label: string;
  kind: StrategyIndicatorSeriesKind;
  paneId: string;
  scaleId: string;
  unit: StrategyIndicatorSeriesUnit;
  defaultVisible: boolean;
  data: StrategyIndicatorPoint[];
  finitePointCount: number;
  referenceLines: StrategyIndicatorReferenceLine[];
}

export interface StrategyIndicatorBundleModel {
  strategyKind: string;
  aggregation: StrategyIndicatorAggregation;
  bars: StrategyIndicatorBar[];
  series: StrategyIndicatorSeriesModel[];
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

function strictFiniteNumber(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function strictNonNegativeInteger(value: unknown): number | null {
  const parsed = strictFiniteNumber(value);
  if (parsed == null || parsed < 0 || !Number.isInteger(parsed)) return null;
  return parsed;
}

function strictText(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

function normalizedIdentifier(value: unknown): string {
  return strictText(value).toLowerCase();
}

function normalizeStrategyIndicatorKind(
  value: unknown,
): StrategyIndicatorSeriesKind | null {
  const normalized = normalizedIdentifier(value);
  return normalized === "line" || normalized === "histogram"
    ? normalized
    : null;
}

function normalizeStrategyIndicatorUnit(
  value: unknown,
): StrategyIndicatorSeriesUnit | null {
  const normalized = normalizedIdentifier(value).replace(/[\s-]+/gu, "_");
  if (normalized === "zscore") return "z_score";
  switch (normalized) {
    case "price":
    case "price_delta":
    case "number":
    case "ratio":
    case "degrees":
    case "state":
    case "z_score":
    case "oscillator":
      return normalized;
    default:
      return null;
  }
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

function dedupeSortedTimeRows<T extends { time: number }>(rows: T[]): T[] {
  if (rows.length <= 1) return rows;
  let sorted = true;
  let hasDuplicateTime = false;
  for (let index = 1; index < rows.length; index += 1) {
    const previousTime = rows[index - 1]?.time ?? 0;
    const nextTime = rows[index]?.time ?? 0;
    if (previousTime === nextTime) hasDuplicateTime = true;
    if (previousTime <= nextTime) continue;
    sorted = false;
    break;
  }
  if (sorted && !hasDuplicateTime) return rows;
  if (!sorted) rows.sort((a, b) => a.time - b.time);
  const deduped: T[] = [];
  for (const row of rows) {
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
  return dedupeSortedTimeRows(out);
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
  return dedupeSortedTimeRows(out);
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
  return dedupeSortedTimeRows(out);
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
    const ottLine = toOptionalNumber(rowRaw.ott_line ?? rowRaw.ottLine);
    const ottPrice = toOptionalNumber(rowRaw.ott_price ?? rowRaw.ottPrice);
    if (halftrend == null && macz == null && ottLine == null && ottPrice == null) continue;
    const previous = out[out.length - 1];
    if (previous && previous.time > time) sorted = false;
    out.push({ time, halftrend, macz, ottLine, ottPrice });
  }
  if (!sorted) out.sort((a, b) => a.time - b.time);
  return dedupeSortedTimeRows(out);
}

interface StrategyIndicatorBarSource extends StrategyIndicatorBar {
  sourceIndex: number;
}

function normalizeStrategyIndicatorBars(
  rowsRaw: unknown[],
): StrategyIndicatorBarSource[] | null {
  const rows: StrategyIndicatorBarSource[] = [];
  for (let sourceIndex = 0; sourceIndex < rowsRaw.length; sourceIndex += 1) {
    const row = rowsRaw[sourceIndex];
    if (!isRecord(row)) continue;
    const timeRaw = row.time ?? row.timestamp ?? row.ts ?? row.t;
    const time =
      strictFiniteNumber(timeRaw) == null
        ? Number.NaN
        : normalizeChartTimeSec(timeRaw);
    const fromIndex = strictNonNegativeInteger(
      row.from_index ?? row.fromIndex ?? row.from,
    );
    const toIndex = strictNonNegativeInteger(
      row.to_index ?? row.toIndex ?? row.to,
    );
    if (
      !Number.isFinite(time) ||
      fromIndex == null ||
      toIndex == null ||
      toIndex < fromIndex
    ) {
      continue;
    }
    rows.push({ time, fromIndex, toIndex, sourceIndex });
  }
  if (rowsRaw.length > 0 && rows.length === 0) return null;

  rows.sort((left, right) => left.time - right.time || left.sourceIndex - right.sourceIndex);
  const deduped: StrategyIndicatorBarSource[] = [];
  for (const row of rows) {
    const previous = deduped[deduped.length - 1];
    if (previous?.time === row.time) deduped[deduped.length - 1] = row;
    else deduped.push(row);
  }
  return deduped;
}

function normalizeStrategyIndicatorReferenceLines(
  raw: unknown,
): StrategyIndicatorReferenceLine[] | null {
  if (raw == null) return [];
  if (!Array.isArray(raw)) return null;
  const lines: StrategyIndicatorReferenceLine[] = [];
  for (const lineRaw of raw) {
    if (!isRecord(lineRaw)) return null;
    const value = strictFiniteNumber(lineRaw.value);
    const label = strictText(lineRaw.label);
    if (value == null || !label) return null;
    lines.push({ value, label });
  }
  return lines;
}

function normalizeStrategyIndicatorSeries(
  raw: unknown,
  rawBarCount: number,
  bars: StrategyIndicatorBarSource[],
): StrategyIndicatorSeriesModel | null {
  if (!isRecord(raw)) return null;
  const id = normalizedIdentifier(raw.id);
  const label = strictText(raw.label);
  const kind = normalizeStrategyIndicatorKind(raw.kind);
  const paneId = normalizedIdentifier(raw.pane_id ?? raw.paneId);
  const scaleId = normalizedIdentifier(raw.scale_id ?? raw.scaleId);
  const unit = normalizeStrategyIndicatorUnit(raw.unit);
  const defaultVisibleRaw = raw.default_visible ?? raw.defaultVisible;
  const valuesRaw = raw.values;
  const referenceLines = normalizeStrategyIndicatorReferenceLines(
    raw.reference_lines ?? raw.referenceLines,
  );
  if (
    !id ||
    !label ||
    !kind ||
    !paneId ||
    !scaleId ||
    !unit ||
    typeof defaultVisibleRaw !== "boolean" ||
    !Array.isArray(valuesRaw) ||
    valuesRaw.length !== rawBarCount ||
    referenceLines == null
  ) {
    return null;
  }

  const values: Array<number | null> = [];
  for (const valueRaw of valuesRaw) {
    if (valueRaw === null) {
      values.push(null);
      continue;
    }
    const value = strictFiniteNumber(valueRaw);
    if (value == null) return null;
    values.push(value);
  }

  let finitePointCount = 0;
  const data: StrategyIndicatorPoint[] = bars.map((bar) => {
    const value = values[bar.sourceIndex];
    if (value == null) return { time: bar.time };
    finitePointCount += 1;
    return { time: bar.time, value };
  });
  return {
    id,
    label,
    kind,
    paneId,
    scaleId,
    unit,
    defaultVisible: defaultVisibleRaw,
    data,
    finitePointCount,
    referenceLines,
  };
}

/**
 * Normalizes the additive standalone-strategy chart contract.
 *
 * Invalid bundle framing returns `null`. Invalid individual series are omitted
 * while valid siblings retain the shared bar/value mapping; if no usable
 * series remains, the bundle returns `null` so legacy chart evidence is not
 * hidden by malformed optional data.
 */
export function normalizeStrategyIndicatorBundle(
  raw: unknown,
): StrategyIndicatorBundleModel | null {
  if (!isRecord(raw)) return null;
  const strategyKind = strictText(raw.strategy_kind ?? raw.strategyKind);
  const aggregation = normalizedIdentifier(raw.aggregation).replace(/-/gu, "_");
  const barsRaw = raw.bars;
  const seriesRaw = raw.series;
  if (
    !strategyKind ||
    aggregation !== "last_finite" ||
    !Array.isArray(barsRaw) ||
    !Array.isArray(seriesRaw)
  ) {
    return null;
  }
  const barsWithSources = normalizeStrategyIndicatorBars(barsRaw);
  if (!barsWithSources) return null;

  const idCounts = new Map<string, number>();
  for (const series of seriesRaw) {
    if (!isRecord(series)) continue;
    const id = normalizedIdentifier(series.id);
    if (id) idCounts.set(id, (idCounts.get(id) ?? 0) + 1);
  }

  const series: StrategyIndicatorSeriesModel[] = [];
  for (const seriesRawItem of seriesRaw) {
    if (!isRecord(seriesRawItem)) continue;
    const id = normalizedIdentifier(seriesRawItem.id);
    if (!id || idCounts.get(id) !== 1) continue;
    const normalized = normalizeStrategyIndicatorSeries(
      seriesRawItem,
      barsRaw.length,
      barsWithSources,
    );
    if (normalized) series.push(normalized);
  }
  if (series.length === 0) return null;

  return {
    strategyKind,
    aggregation: "last_finite",
    bars: barsWithSources.map(({ time, fromIndex, toIndex }) => ({
      time,
      fromIndex,
      toIndex,
    })),
    series,
  };
}
