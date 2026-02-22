export function num(v, fallback) {
  const x = Number(v);
  return Number.isFinite(x) ? x : fallback;
}

export function expandRangeU16Like(start, end, step) {
  const s0 = Math.floor(Number(start));
  const e0 = Math.floor(Number(end));
  const st0 = Math.floor(Number(step));
  if (!Number.isFinite(s0) || !Number.isFinite(e0) || !Number.isFinite(st0)) return [];

  if (st0 === 0 || s0 === e0) return [s0];
  const st = Math.max(1, st0);
  const lo = Math.min(s0, e0);
  const hi = Math.max(s0, e0);
  const out = [];
  let v = lo;
  for (;;) {
    out.push(v);
    if (v === hi) break;
    const next = v + st;
    if (next <= v || next > hi) break;
    v = next;
  }
  return out;
}

export function countPairsFastLtSlow(fastPeriods, slowPeriods) {
  const f = Array.isArray(fastPeriods) ? fastPeriods : [];
  const s = Array.isArray(slowPeriods) ? slowPeriods : [];
  if (!f.length || !s.length) return 0;

  const slowSorted = [...s].sort((a, b) => a - b);
  let total = 0;
  for (const ff of f) {
    let lo = 0;
    let hi = slowSorted.length;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (slowSorted[mid] <= ff) lo = mid + 1;
      else hi = mid;
    }
    total += slowSorted.length - lo;
  }
  return total;
}

export function countF64RangeValues(start, end, step) {
  const a = Number(start);
  const b = Number(end);
  const s = Number(step);
  if (!Number.isFinite(a) || !Number.isFinite(b) || !Number.isFinite(s)) return 1;
  if (s === 0 || a === b) return 1;
  const lo = Math.min(a, b);
  const hi = Math.max(a, b);
  const st = Math.abs(s);
  if (!(st > 0) || !Number.isFinite(st)) return 1;
  const n = Math.floor((hi - lo) / st) + 1;
  return Math.max(1, Math.min(1_000_000, n));
}
