export function maRequiresVolumeField(maId) {
  const id = String(maId || "").trim().toLowerCase();
  return id === "vwma" || id === "vpwma";
}

export function maRequiresHighLowField(maId) {
  const id = String(maId || "").trim().toLowerCase();
  return id === "frama";
}

export function requiredFieldsForMaSource(source) {
  const s = String(source || "").trim().toLowerCase();
  if (s === "close") return ["close"];
  if (s === "open") return ["open"];
  if (s === "high") return ["high"];
  if (s === "low") return ["low"];
  if (s === "hl2") return ["high", "low"];
  if (s === "hlc3") return ["high", "low", "close"];
  if (s === "ohlc4") return ["open", "high", "low", "close"];
  if (s === "hlcc" || s === "hlcc4") return ["high", "low", "close"];
  return [];
}

export function validateCandleFieldRequirements(meta, fastMaType, slowMaType, maSource) {
  if (!meta || typeof meta !== "object") return;

  const missing = new Set();
  const require = (field) => {
    if (field === "open" && !meta.has_open) missing.add("open");
    if (field === "high" && !meta.has_high) missing.add("high");
    if (field === "low" && !meta.has_low) missing.add("low");
    if (field === "close" && !meta.has_close) missing.add("close");
    if (field === "volume" && !meta.has_volume) missing.add("volume");
  };

  if (maRequiresVolumeField(fastMaType) || maRequiresVolumeField(slowMaType)) require("volume");
  if (maRequiresHighLowField(fastMaType) || maRequiresHighLowField(slowMaType)) {
    require("high");
    require("low");
  }
  for (const f of requiredFieldsForMaSource(maSource)) require(f);

  if (missing.size) {
    const parts = [];
    if (missing.has("volume")) parts.push("volume (needed by VWMA/VPWMA)");
    if (missing.has("high") || missing.has("low")) parts.push("high/low (needed by FRAMA and some sources)");
    if (missing.has("open")) parts.push("open (needed by some sources)");
    if (missing.has("close")) parts.push("close");
    throw new Error(`Loaded CSV is missing required candle fields: ${parts.join(", ")}.`);
  }
}

export function fieldPresent(meta, field) {
  if (!meta || typeof meta !== "object") return false;
  if (field === "open") return !!meta.has_open;
  if (field === "high") return !!meta.has_high;
  if (field === "low") return !!meta.has_low;
  if (field === "close") return !!meta.has_close;
  if (field === "volume") return !!meta.has_volume;
  return false;
}
