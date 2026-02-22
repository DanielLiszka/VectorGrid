import { fmtPct } from "./format.js";
import { num } from "./ranges.js";

export function compactProgressText(text, maxChars) {
  const s = String(text || "").trim();
  const max = Math.max(64, Math.floor(Number(maxChars || 360)));
  if (s.length <= max) return s;
  return `${s.slice(0, Math.max(1, max - 1)).trimEnd()}…`;
}

export function backendProgressLabel(backendRaw) {
  const b = String(backendRaw || "CpuOnly").trim() || "CpuOnly";
  if (b === "GpuKernel") return "GPU kernel";
  if (b === "GpuOnly") return "GPU sweep";
  if (b === "Auto") return "Auto backend";
  return "CPU";
}

export function samplerProgressLabel(samplerRaw) {
  const s = String(samplerRaw || "Nsga2").trim() || "Nsga2";
  if (s === "Nsga2") return "NSGA-II";
  if (s === "Tpe") return "TPE";
  if (s === "Qmc") return "QMC";
  if (s === "Random") return "Random";
  return s;
}

export function formatRiskProgressSummary(strategy) {
  const st = strategy && typeof strategy === "object" ? strategy : null;
  if (!st) return "";
  const sl = Number(st.stop_loss_pct);
  const tp = Number(st.take_profit_pct);
  const trail = Number(st.trailing_stop_pct);
  const cooldown = Math.max(0, Math.floor(num(st.cooldown_bars, 0)));
  const persist = Math.max(1, Math.floor(num(st.signal_persistence_bars, 1)));
  const parts = [];
  if (Number.isFinite(sl) && sl > 0) parts.push(`SL ${fmtPct(sl, 2)}`);
  if (Number.isFinite(tp) && tp > 0) parts.push(`TP ${fmtPct(tp, 2)}`);
  if (Number.isFinite(trail) && trail > 0) parts.push(`Trail ${fmtPct(trail, 2)}`);
  if (cooldown > 0) parts.push(`CD ${cooldown}b`);
  if (persist > 1) parts.push(`Persist ${persist}b`);
  if (!parts.length) return "";
  return `risk ${parts.join(" / ")}`;
}

export function formatDoubleConstraintsProgressSummary(constraints) {
  const c = constraints && typeof constraints === "object" ? constraints : null;
  if (!c) return "";
  const parts = [];
  const minTrades = Number(c.min_trades);
  if (Number.isFinite(minTrades) && minTrades > 0) parts.push(`trades>=${Math.floor(minTrades)}`);
  const maxDd = Number(c.max_drawdown);
  if (Number.isFinite(maxDd) && maxDd >= 0) parts.push(`dd<=${fmtPct(maxDd, 2)}`);
  if (!parts.length) return "";
  return `filters ${parts.join(", ")}`;
}

export function formatMaPairProgressSummary(fastTypes, slowTypes) {
  const fast = (Array.isArray(fastTypes) ? fastTypes : [])
    .map((x) => String(x || "").trim().toUpperCase())
    .filter((x) => !!x);
  const slow = (Array.isArray(slowTypes) ? slowTypes : [])
    .map((x) => String(x || "").trim().toUpperCase())
    .filter((x) => !!x);
  if (fast.length === 1 && slow.length === 1) return `${fast[0]} vs ${slow[0]}`;
  if (fast.length || slow.length) return `MA sets f${Math.max(1, fast.length)}×s${Math.max(1, slow.length)}`;
  return "";
}
