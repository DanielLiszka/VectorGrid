export interface StrategyHeatmapAxes {
  fastAxisLabel: string;
  slowAxisLabel: string;
  fastAxisDescription: string;
  slowAxisDescription: string;
  supports3dSlices: boolean;
  threeDimensionalUnavailableReason: string;
}

type StrategyRequest = Record<string, unknown>;

const STANDALONE_STRATEGY_KINDS = new Set([
  "Asr",
  "Vcc",
  "Vemr",
  "Rocb",
  "Vcet",
  "Mkcs",
  "Kqsr",
  "Carm",
  "Kvdf",
  "Cdmc",
]);

export const STANDALONE_HEATMAP_3D_UNAVAILABLE_REASON =
  "3D slices are unavailable for standalone strategy heatmaps because each run exposes one strategy-local fast/slow surface rather than MA-type or parameter variants.";

function asRecord(value: unknown): StrategyRequest {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as StrategyRequest)
    : {};
}

function cleanText(value: unknown): string {
  return String(value ?? "").trim();
}

function normalizedKey(value: unknown): string {
  return cleanText(value)
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "");
}

function strategyKind(value: unknown): string {
  const request = asRecord(value);
  const rawKind =
    value && typeof value === "object" && !Array.isArray(value)
      ? (request.strategy_kind ?? request.strategyKind ?? request.kind)
      : value;
  const normalized = normalizedKey(rawKind);
  for (const kind of STANDALONE_STRATEGY_KINDS) {
    if (normalized === normalizedKey(kind)) return kind;
  }
  return cleanText(rawKind) || "DoubleMa";
}

export function isStandaloneStrategyHeatmap(
  requestOrKind: StrategyRequest | string | null | undefined,
): boolean {
  return STANDALONE_STRATEGY_KINDS.has(strategyKind(requestOrKind));
}

function camelCaseKey(snakeCaseKey: string): string {
  return snakeCaseKey.replace(/_([a-z0-9])/g, (_, next: string) =>
    next.toUpperCase(),
  );
}

function strategyConfig(
  request: StrategyRequest,
  kind: string,
): StrategyRequest {
  const key = kind.toLowerCase();
  const direct = asRecord(request[key]);
  if (Object.keys(direct).length) return direct;
  const camelDirect = asRecord(request[camelCaseKey(key)]);
  if (Object.keys(camelDirect).length) return camelDirect;

  const strategyConfigRoot = asRecord(
    request.strategy_config ?? request.strategyConfig,
  );
  const nested = asRecord(
    strategyConfigRoot[key] ?? strategyConfigRoot[camelCaseKey(key)],
  );
  return Object.keys(nested).length ? nested : strategyConfigRoot;
}

function booleanValue(value: unknown, fallback: boolean): boolean {
  if (typeof value === "boolean") return value;
  if (typeof value === "number" && Number.isFinite(value)) return value !== 0;
  if (typeof value === "string") {
    const normalized = value.trim().toLowerCase();
    if (["true", "1", "yes", "on"].includes(normalized)) return true;
    if (["false", "0", "no", "off"].includes(normalized)) return false;
  }
  return fallback;
}

function mappingEnabled(
  request: StrategyRequest,
  config: StrategyRequest,
  snakeCaseKey: string,
): boolean {
  const camelKey = camelCaseKey(snakeCaseKey);
  for (const source of [config, request]) {
    if (Object.prototype.hasOwnProperty.call(source, snakeCaseKey)) {
      return booleanValue(source[snakeCaseKey], true);
    }
    if (Object.prototype.hasOwnProperty.call(source, camelKey)) {
      return booleanValue(source[camelKey], true);
    }
  }
  return true;
}

function maType(request: StrategyRequest, axis: "fast" | "slow"): string {
  const plural = request[`${axis}_ma_types`] ?? request[`${axis}MaTypes`];
  const raw = Array.isArray(plural)
    ? plural[0]
    : (request[`${axis}_ma_type`] ?? request[`${axis}MaType`]);
  const value = cleanText(raw);
  return value && value !== "?" ? value.toUpperCase() : "";
}

function genericAxes(request: StrategyRequest): StrategyHeatmapAxes {
  const fastType = maType(request, "fast");
  const slowType = maType(request, "slow");
  const fastAxisLabel = fastType ? `Fast ${fastType} length` : "Fast length";
  const slowAxisLabel = slowType ? `Slow ${slowType} length` : "Slow length";
  return {
    fastAxisLabel,
    slowAxisLabel,
    fastAxisDescription: `${fastAxisLabel} used by each grid column.`,
    slowAxisDescription: `${slowAxisLabel} used by each grid row.`,
    supports3dSlices: true,
    threeDimensionalUnavailableReason: "",
  };
}

function unmappedAxis(axis: "Fast" | "Slow"): {
  label: string;
  description: string;
} {
  return {
    label: `${axis} grid (unmapped)`,
    description: `Changing the ${axis.toLowerCase()} grid coordinate does not alter any mapped strategy periods for this request.`,
  };
}

function mappedPeriodAxis(
  axis: "Fast" | "Slow",
  names: string[],
  description: string,
): { label: string; description: string } {
  if (!names.length) return unmappedAxis(axis);
  const joinedNames = names.join(" / ");
  const labelNames = `${joinedNames.charAt(0).toUpperCase()}${joinedNames.slice(1)}`;
  return {
    label: `${labelNames} ${names.length === 1 ? "period" : "periods"}`,
    description,
  };
}

function standaloneAxes(
  fastAxisLabel: string,
  slowAxisLabel: string,
  fastAxisDescription: string,
  slowAxisDescription: string,
): StrategyHeatmapAxes {
  return {
    fastAxisLabel,
    slowAxisLabel,
    fastAxisDescription,
    slowAxisDescription,
    supports3dSlices: false,
    threeDimensionalUnavailableReason: STANDALONE_HEATMAP_3D_UNAVAILABLE_REASON,
  };
}

export function resolveStrategyHeatmapAxes(
  requestRaw: StrategyRequest | null | undefined,
): StrategyHeatmapAxes {
  const request = asRecord(requestRaw);
  const kind = strategyKind(request);
  const config = strategyConfig(request, kind);

  if (kind === "Asr") {
    const fast = mappedPeriodAxis(
      "Fast",
      mappingEnabled(request, config, "map_fast_to_squeeze_period")
        ? ["Squeeze"]
        : [],
      "Grid columns set the adaptive squeeze period.",
    );
    const slowNames = [
      mappingEnabled(request, config, "map_slow_to_trend_period")
        ? "Trend"
        : "",
      mappingEnabled(request, config, "map_slow_to_er_period") ? "ER" : "",
      mappingEnabled(request, config, "map_slow_to_z_period") ? "Z-score" : "",
    ].filter(Boolean);
    const slow = mappedPeriodAxis(
      "Slow",
      slowNames,
      `Grid rows set the ${slowNames.join(", ")} ${slowNames.length === 1 ? "period" : "periods"}.`,
    );
    return standaloneAxes(
      fast.label,
      slow.label,
      fast.description,
      slow.description,
    );
  }

  if (kind === "Vcc") {
    const fastNames = [
      mappingEnabled(request, config, "map_fast_to_squeeze_period")
        ? "Squeeze"
        : "",
      mappingEnabled(request, config, "map_fast_to_momentum_period")
        ? "momentum"
        : "",
    ].filter(Boolean);
    const fast = mappedPeriodAxis(
      "Fast",
      fastNames,
      `Grid columns set the ${fastNames.join(" and ")} ${fastNames.length === 1 ? "period" : "periods"}.`,
    );
    const slow = mappedPeriodAxis(
      "Slow",
      mappingEnabled(request, config, "map_slow_to_trend_period")
        ? ["Trend"]
        : [],
      "Grid rows set the trend period.",
    );
    return standaloneAxes(
      fast.label,
      slow.label,
      fast.description,
      slow.description,
    );
  }

  if (kind === "Vemr") {
    const fastNames = [
      mappingEnabled(request, config, "map_fast_to_z_period") ? "Z-score" : "",
      mappingEnabled(request, config, "map_fast_to_bb_period") ? "BB" : "",
    ].filter(Boolean);
    const fast = mappedPeriodAxis(
      "Fast",
      fastNames,
      `Grid columns set the ${fastNames.join(" and ")} ${fastNames.length === 1 ? "period" : "periods"}.`,
    );
    const slow = mappedPeriodAxis(
      "Slow",
      mappingEnabled(request, config, "map_slow_to_atr_period") ? ["ATR"] : [],
      "Grid rows set the ATR period.",
    );
    return standaloneAxes(
      fast.label,
      slow.label,
      fast.description,
      slow.description,
    );
  }

  if (kind === "Rocb") {
    const fast = mappedPeriodAxis(
      "Fast",
      mappingEnabled(request, config, "map_fast_to_channel_period")
        ? ["Channel"]
        : [],
      "Grid columns set the breakout channel period.",
    );
    const slow = mappedPeriodAxis(
      "Slow",
      mappingEnabled(request, config, "map_slow_to_atr_period") ? ["ATR"] : [],
      "Grid rows set the ATR stop period.",
    );
    return standaloneAxes(
      fast.label,
      slow.label,
      fast.description,
      slow.description,
    );
  }

  if (kind === "Vcet") {
    const fast = mappedPeriodAxis(
      "Fast",
      mappingEnabled(request, config, "map_fast_to_vidya_short_period")
        ? ["VIDYA short"]
        : [],
      "Grid columns set the short VIDYA period.",
    );
    const slowNames = [
      mappingEnabled(request, config, "map_slow_to_vidya_long_period")
        ? "VIDYA long"
        : "",
      mappingEnabled(request, config, "map_slow_to_ce_period")
        ? "Chandelier"
        : "",
    ].filter(Boolean);
    const slow = mappedPeriodAxis(
      "Slow",
      slowNames,
      `Grid rows set the ${slowNames.join(" and ")} ${slowNames.length === 1 ? "period" : "periods"}.${slowNames.includes("VIDYA long") ? " Long VIDYA is also kept above short VIDYA." : ""}`,
    );
    return standaloneAxes(
      fast.label,
      slow.label,
      fast.description,
      slow.description,
    );
  }

  if (kind === "Mkcs") {
    const fast = mappedPeriodAxis(
      "Fast",
      mappingEnabled(request, config, "map_fast_to_stddev_period")
        ? ["StdDev"]
        : [],
      "Grid columns set the standard-deviation period.",
    );
    const slow = mappingEnabled(
      request,
      config,
      "map_slow_to_breakout_lookback",
    )
      ? {
          label: "Breakout lookback",
          description: "Grid rows set the breakout lookback.",
        }
      : unmappedAxis("Slow");
    return standaloneAxes(
      fast.label,
      slow.label,
      fast.description,
      slow.description,
    );
  }

  if (kind === "Kqsr") {
    return standaloneAxes(
      "Fast oscillator period floor",
      "Slow oscillator period floor",
      "Grid columns floor the KDJ, Stochastic RSI, and IFT-RSI periods; configured minima can keep effective periods higher.",
      "Grid rows floor the QQE and PRB periods; configured minima can keep effective periods higher.",
    );
  }

  if (kind === "Carm") {
    return standaloneAxes(
      "Fast indicator period floor",
      "Slow indicator period floor",
      "Grid columns floor the fast Coppock, AO, ROCR, and APO periods and can raise paired slow periods to preserve ordering; configured minima can also keep effective periods higher.",
      "Grid rows floor the slow Coppock, AO, APO, and OTTO periods; configured minima and fast/slow ordering can keep effective periods higher.",
    );
  }

  if (kind === "Kvdf") {
    return standaloneAxes(
      "FVG / fast DTI period floors",
      "Keltner / DevStop / DTI period floors",
      "Grid columns floor FVG lookback and smoothing plus the fast DTI stages; configured minima can keep effective periods higher.",
      "Grid rows floor Keltner, DevStop, and the root DTI period; configured minima can keep effective periods higher.",
    );
  }

  if (kind === "Cdmc") {
    return standaloneAxes(
      "DPO period floor",
      "Cycle / decycler period floors",
      "Grid columns floor the DPO period; its configured minimum can keep the effective period higher.",
      "Grid rows floor correlation-cycle and decycler periods; configured minima can keep effective periods higher.",
    );
  }

  return genericAxes(request);
}
