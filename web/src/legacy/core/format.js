export function pretty(obj) {
  try {
    return JSON.stringify(obj, null, 2);
  } catch {
    return String(obj);
  }
}

export function fmtFixed(n, digits) {
  const x = Number(n);
  if (!Number.isFinite(x)) return "0";
  const d = Math.max(0, Math.min(12, Math.floor(Number(digits ?? 3))));
  return x.toFixed(d);
}

export function fmtMoney(x, digits) {
  const n = Number(x);
  if (!Number.isFinite(n)) return "-";
  const d = Number.isFinite(digits) ? Math.max(0, Math.min(8, Math.floor(digits))) : 2;
  return n.toLocaleString(undefined, { minimumFractionDigits: d, maximumFractionDigits: d });
}

export function fmt(x, digits) {
  const n = Number(x);
  if (!Number.isFinite(n)) return "-";
  const d = Number.isFinite(digits) ? digits : 4;
  return n.toFixed(d);
}

export function fmtPct(x, digits) {
  const n = Number(x);
  if (!Number.isFinite(n)) return "-";
  const d = Number.isFinite(digits) ? digits : 1;
  return (n * 100).toFixed(d) + "%";
}

export function formatDurationSeconds(sec) {
  const s0 = Math.max(0, Math.floor(Number(sec || 0)));
  if (!Number.isFinite(s0) || s0 <= 0) return "";
  const h = Math.floor(s0 / 3600);
  const m = Math.floor((s0 % 3600) / 60);
  const s = s0 % 60;
  if (h > 0) return `${h}h${String(m).padStart(2, "0")}m`;
  if (m > 0) return `${m}m${String(s).padStart(2, "0")}s`;
  return `${s}s`;
}

export function formatCompactNumber(n) {
  const x0 = Number(n);
  if (!Number.isFinite(x0)) return "";
  const abs = Math.abs(x0);
  let x = x0;
  let suffix = "";
  if (abs >= 1e9) {
    x = x0 / 1e9;
    suffix = "G";
  } else if (abs >= 1e6) {
    x = x0 / 1e6;
    suffix = "M";
  } else if (abs >= 1e3) {
    x = x0 / 1e3;
    suffix = "k";
  }
  const ax = Math.abs(x);
  const digits = ax >= 100 ? 0 : (ax >= 10 ? 1 : 2);
  const s = x
    .toFixed(digits)
    .replace(/\.0+$/, "")
    .replace(/(\.\d*[1-9])0+$/, "$1");
  return `${s}${suffix}`;
}

export function formatPerSecond(rate, unit) {
  const r = Number(rate);
  if (!Number.isFinite(r) || r <= 0) return "";
  const c = formatCompactNumber(r);
  if (!c) return "";
  return unit ? `${c} ${unit}/s` : `${c}/s`;
}

export function ymdLocal(d) {
  const yyyy = d.getFullYear();
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${yyyy}-${mm}-${dd}`;
}

export function fmtYmdUtc(ms) {
  const n = Number(ms);
  if (!Number.isFinite(n)) return "—";
  const d = new Date(n);
  const t = d.getTime();
  if (!Number.isFinite(t)) return "—";
  return d.toISOString().slice(0, 10);
}
