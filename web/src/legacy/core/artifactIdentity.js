export function parseTagsInput(raw) {
  const s = String(raw || "");
  const parts = s
    .split(/[,;\n]+/g)
    .flatMap((p) => String(p || "").trim().split(/\s+/g))
    .map((p) => p.trim())
    .filter((p) => !!p);

  const seen = new Set();
  const out = [];
  for (const t of parts) {
    const k = t.toLowerCase();
    if (seen.has(k)) continue;
    seen.add(k);
    out.push(t);
  }
  return out;
}

export function defaultBatchTag() {
  const d = new Date();
  const pad = (n) => String(Math.max(0, Math.floor(Number(n || 0)))).padStart(2, "0");
  const y = d.getUTCFullYear();
  const m = pad(d.getUTCMonth() + 1);
  const day = pad(d.getUTCDate());
  const h = pad(d.getUTCHours());
  const min = pad(d.getUTCMinutes());
  const s = pad(d.getUTCSeconds());
  return `batch-${y}${m}${day}-${h}${min}${s}`;
}

export function dedupTags(tags) {
  const xs = Array.isArray(tags) ? tags : [];
  const seen = new Set();
  const out = [];
  for (const t of xs) {
    const s = String(t || "").trim();
    if (!s) continue;
    const k = s.toLowerCase();
    if (seen.has(k)) continue;
    seen.add(k);
    out.push(s);
  }
  return out;
}

export function hashFNV1a32(text) {
  let h = 2166136261;
  const s = String(text || "");
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

function canonicalizeJsonValue(v) {
  if (v == null) return null;
  const t = typeof v;
  if (t === "number") return Number.isFinite(v) ? v : null;
  if (t === "string" || t === "boolean") return v;
  if (Array.isArray(v)) return v.map(canonicalizeJsonValue);
  if (t !== "object") return null;
  const out = {};
  const keys = Object.keys(v).sort();
  for (const k of keys) {
    const vv = v[k];
    if (vv === undefined) continue;
    out[k] = canonicalizeJsonValue(vv);
  }
  return out;
}

export function canonicalJsonString(v) {
  return JSON.stringify(canonicalizeJsonValue(v));
}

export function hashFNV1a64Hex(text) {
  const s = String(text || "");
  if (typeof BigInt !== "function") {
    const h32 = hashFNV1a32(s);
    return h32.toString(16).padStart(8, "0");
  }
  let h = 0xcbf29ce484222325n;
  const prime = 0x100000001b3n;
  for (let i = 0; i < s.length; i++) {
    h ^= BigInt(s.charCodeAt(i));
    h = (h * prime) & 0xffffffffffffffffn;
  }
  return h.toString(16).padStart(16, "0");
}

function stripRunReqForFingerprint(runReq) {
  const req = runReq && typeof runReq === "object" ? runReq : null;
  if (!req) return null;
  const out = canonicalizeJsonValue(req);
  if (!out || typeof out !== "object") return null;
  delete out.data_id;
  delete out.dataId;
  return out;
}

function stripValidationReqForFingerprint(valReq) {
  const req = valReq && typeof valReq === "object" ? valReq : null;
  if (!req) return null;
  const out = canonicalizeJsonValue(req);
  if (!out || typeof out !== "object") return null;
  delete out.data_id;
  delete out.dataId;
  delete out.candidates;
  const cross = out.cross_ticker && typeof out.cross_ticker === "object" ? out.cross_ticker : null;
  if (cross && Array.isArray(cross.datasets)) {
    const datasets = cross.datasets
      .map((d) => ({ csv_path: String(d?.csv_path || "").trim() }))
      .filter((d) => !!d.csv_path)
      .sort((a, b) => a.csv_path.localeCompare(b.csv_path));
    cross.datasets = datasets;
  }
  return out;
}

export function computeConfigFingerprint(runReq, valReq) {
  const payload = {
    version: 1,
    run_req: stripRunReqForFingerprint(runReq),
    validation_req: stripValidationReqForFingerprint(valReq),
  };
  const canon = canonicalJsonString(payload);
  const hex = hashFNV1a64Hex(canon);
  return `cfg1-fnv1a64-${hex}`;
}

export function splitHoldoutPaths(paths, holdoutPct, seed) {
  const xs = Array.isArray(paths) ? paths.map((p) => String(p || "").trim()).filter((p) => !!p) : [];
  const pct = Number.isFinite(Number(holdoutPct)) ? Math.max(0, Math.min(50, Number(holdoutPct))) : 0;
  if (!xs.length || pct <= 0) return { train: xs, holdout: [] };

  const n = xs.length;
  const k = Math.max(0, Math.min(n, Math.floor(n * (pct / 100))));
  if (k <= 0) return { train: xs, holdout: [] };
  if (k >= n) return { train: [], holdout: xs };

  const seed32 = hashFNV1a32(String(seed ?? 0));
  const scored = xs.map((p) => ({ p, s: (hashFNV1a32(p) ^ seed32) >>> 0 }));
  scored.sort((a, b) => a.s - b.s);

  const holdout = scored.slice(0, k).map((x) => x.p);
  const train = scored.slice(k).map((x) => x.p);
  return { train, holdout };
}
