export function clamp01(x) {
  return Math.max(0, Math.min(1, x));
}

function lerp(a, b, t) {
  return a + (b - a) * t;
}

function hexToRgb(hex) {
  const h = String(hex || "").trim().replace("#", "");
  const raw = h.length === 3 ? h.split("").map((c) => c + c).join("") : h;
  const i = Number.parseInt(raw || "0", 16);
  return { r: (i >> 16) & 255, g: (i >> 8) & 255, b: i & 255 };
}

function makeHeatmapInterpolator(stops) {
  const S = (Array.isArray(stops) ? stops : [])
    .map((s) => ({ t: clamp01(Number(s?.t ?? 0)), rgb: hexToRgb(s?.color || "#000000") }))
    .sort((a, b) => a.t - b.t);
  return (t) => {
    const x = clamp01(Number(t ?? 0));
    for (let i = 0; i < S.length - 1; i++) {
      const a = S[i];
      const b = S[i + 1];
      if (x >= a.t && x <= b.t) {
        const u = (x - a.t) / Math.max(1e-12, (b.t - a.t));
        return {
          r: Math.round(lerp(a.rgb.r, b.rgb.r, u)),
          g: Math.round(lerp(a.rgb.g, b.rgb.g, u)),
          b: Math.round(lerp(a.rgb.b, b.rgb.b, u)),
        };
      }
    }
    const first = S[0]?.rgb || { r: 0, g: 0, b: 0 };
    const last = S[S.length - 1]?.rgb || first;
    return x < (S[0]?.t ?? 0) ? first : last;
  };
}

const HEATMAP_COLORMAPS = {
  cividis: makeHeatmapInterpolator([
    { t: 0.0, color: "#00224e" },
    { t: 0.25, color: "#2b3f66" },
    { t: 0.5, color: "#665c66" },
    { t: 0.75, color: "#a38f49" },
    { t: 1.0, color: "#ffd43b" },
  ]),
  viridis: makeHeatmapInterpolator([
    { t: 0.0, color: "#440154" },
    { t: 0.25, color: "#3b528b" },
    { t: 0.5, color: "#21918c" },
    { t: 0.75, color: "#5ec962" },
    { t: 1.0, color: "#fde725" },
  ]),
  blueOrange: makeHeatmapInterpolator([
    { t: 0.0, color: "#2b8cbe" },
    { t: 0.5, color: "#ffffff" },
    { t: 1.0, color: "#e34a33" },
  ]),
  redGreen: makeHeatmapInterpolator([
    { t: 0.0, color: "#b2182b" },
    { t: 0.5, color: "#f7f7f7" },
    { t: 1.0, color: "#1a9850" },
  ]),
};

const HEATMAP_LUT_CACHE = new Map();

export function heatmapGetLut(name, size) {
  const n = String(name || "cividis");
  const s = Math.max(2, Math.floor(Number(size || 512)));
  const key = `${n}:${s}`;
  const cached = HEATMAP_LUT_CACHE.get(key);
  if (cached) return cached;

  const interp = HEATMAP_COLORMAPS[n] || HEATMAP_COLORMAPS.cividis;
  const lut = new Uint8Array(s * 4);
  for (let i = 0; i < s; i++) {
    const t = i / (s - 1);
    const { r, g, b } = interp(t);
    lut[i * 4 + 0] = r;
    lut[i * 4 + 1] = g;
    lut[i * 4 + 2] = b;
    lut[i * 4 + 3] = 255;
  }
  HEATMAP_LUT_CACHE.set(key, lut);
  return lut;
}

export function quantileSorted(sorted, q) {
  const a = Array.isArray(sorted) ? sorted : [];
  if (!a.length) return NaN;
  const qq = clamp01(Number(q ?? 0));
  const idx = (a.length - 1) * qq;
  const lo = Math.floor(idx);
  const hi = Math.ceil(idx);
  if (lo === hi) return a[lo];
  return a[lo] + (a[hi] - a[lo]) * (idx - lo);
}

export function heatmapBinPeriod(v, vMin, vMax, bins) {
  const b = Math.max(1, Math.floor(Number(bins || 0)));
  if (b <= 1) return 0;
  const minV = Math.floor(Number(vMin || 0));
  const maxV = Math.floor(Number(vMax || 0));
  const denom = Math.max(0, maxV - minV);
  if (denom === 0) return 0;
  const x = Math.max(0, Math.floor(Number(v || 0)) - minV);
  return Math.min(b - 1, Math.floor((x * (b - 1)) / denom));
}

export function heatmapBinRange(bin, vMin, vMax, bins) {
  const b = Math.max(1, Math.floor(Number(bins || 0)));
  const minV = Math.floor(Number(vMin || 0));
  const maxV = Math.floor(Number(vMax || 0));
  const denom = Math.max(0, maxV - minV);
  if (b <= 1 || denom === 0) {
    const v = denom === 0 ? minV : maxV;
    return { lo: v, hi: v, center: v };
  }

  const bb = Math.max(0, Math.min(b - 1, Math.floor(Number(bin || 0))));
  const loX = Math.ceil((bb * denom) / (b - 1));
  const hiX = Math.ceil(((bb + 1) * denom) / (b - 1)) - 1;
  const lo = minV + Math.max(0, Math.min(denom, loX));
  const hi = minV + Math.max(0, Math.min(denom, Math.max(loX, hiX)));
  const center = Math.round((lo + hi) / 2);
  return { lo, hi, center };
}
