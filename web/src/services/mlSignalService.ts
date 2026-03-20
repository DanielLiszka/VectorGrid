import { getActivePinia } from "pinia";
import { invokeCommand } from "../tauri/invoke";
import {
  applyWorkflowArtifactsPayload,
  type WorkflowRunArtifact,
} from "../stores/workflowArtifactsStore";
import { useOptimizeStore } from "../stores/optimizeStore";
import { useStrategyStore } from "../stores/strategyStore";
import { useWorkflowArtifactsStore } from "../stores/workflowArtifactsStore";
import { computeConfigFingerprint } from "../utils/core/artifactIdentity";
import { buildDatasetMetaForArtifact } from "../utils/core/datasetMeta";

interface SaveRunArtifactResponse {
  path?: unknown;
}

interface MlSignalModelBundleSummary {
  bundle_id?: unknown;
  manifest_path?: unknown;
  state?: unknown;
  model_family?: unknown;
  model_format?: unknown;
  issues?: unknown;
}

interface MlSignalModelStatusResponse {
  models_dir?: unknown;
  status?: unknown;
}

interface SweepAxisF64 {
  start: number;
  end: number;
  step: number;
}

interface SweepAxisUsize {
  start: number;
  end: number;
  step: number;
}

function normalizeText(value: unknown): string {
  return String(value ?? "").trim();
}

function parseNumber(value: unknown, fallback: number): number {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function parseOptionalPositive(value: unknown): number | null {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed <= 0) return null;
  return parsed;
}

function parseProbabilityGate(value: unknown): number | null {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return null;
  if (parsed < 0 || parsed > 1) return null;
  return parsed;
}

function parseOptionalInteger(value: unknown): number | null {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return null;
  const floored = Math.floor(parsed);
  return floored >= 0 ? floored : null;
}

function basename(path: string): string {
  const normalized = String(path || "").trim();
  if (!normalized) return "";
  return normalized.split(/[\\/]/).pop() || normalized;
}

async function openOrRevealPath(path: string): Promise<{
  path: string;
  openedWith: "open_path" | "reveal_path";
}> {
  const nextPath = normalizeText(path);
  if (!nextPath) throw new Error("Path is required.");
  try {
    await invokeCommand("open_path", { path: nextPath });
    return { path: nextPath, openedWith: "open_path" };
  } catch {
    await invokeCommand("reveal_path", { path: nextPath });
    return { path: nextPath, openedWith: "reveal_path" };
  }
}

function buildOptionalSweepAxisF64(
  startRaw: unknown,
  endRaw: unknown,
  stepRaw: unknown,
): SweepAxisF64 | null {
  const start = Number(startRaw);
  const end = Number(endRaw);
  const step = Number(stepRaw);
  if (
    !Number.isFinite(start) ||
    !Number.isFinite(end) ||
    !Number.isFinite(step)
  )
    return null;
  if (step <= 0 || end < start) return null;
  return { start, end, step };
}

function buildOptionalSweepAxisUsize(
  startRaw: unknown,
  endRaw: unknown,
  stepRaw: unknown,
): SweepAxisUsize | null {
  const start = parseOptionalInteger(startRaw);
  const end = parseOptionalInteger(endRaw);
  const step = parseOptionalInteger(stepRaw);
  if (start == null || end == null || step == null) return null;
  if (step <= 0 || end < start) return null;
  return { start: Math.max(1, start), end: Math.max(1, end), step };
}

function readCandidateRows(value: unknown): Array<Record<string, unknown>> {
  if (!value || typeof value !== "object") return [];
  const row = value as Record<string, unknown>;
  if (Array.isArray(row.candidates)) {
    return row.candidates.filter(
      (entry): entry is Record<string, unknown> =>
        !!entry && typeof entry === "object",
    );
  }
  const validation =
    row.validation && typeof row.validation === "object"
      ? (row.validation as Record<string, unknown>)
      : null;
  if (validation && Array.isArray(validation.candidates)) {
    return validation.candidates.filter(
      (entry): entry is Record<string, unknown> =>
        !!entry && typeof entry === "object",
    );
  }
  return [];
}

function extractMlPolicy(
  validationReq: Record<string, unknown>,
  validationRes: unknown,
): Record<string, unknown> | null {
  if (
    Array.isArray(validationReq.candidates) &&
    validationReq.candidates.length > 0 &&
    validationReq.candidates[0] &&
    typeof validationReq.candidates[0] === "object"
  ) {
    return validationReq.candidates[0] as Record<string, unknown>;
  }
  const best = readCandidateRows(validationRes)[0];
  if (!best) return null;
  return best.params && typeof best.params === "object"
    ? (best.params as Record<string, unknown>)
    : null;
}

function extractMlPolicySweepSummary(
  validationRes: unknown,
): Record<string, unknown> | null {
  if (!validationRes || typeof validationRes !== "object") return null;
  const row = validationRes as Record<string, unknown>;
  const sweepSize = parseOptionalInteger(row.sweep_size);
  const topK = parseOptionalInteger(row.top_k);
  if (sweepSize == null && topK == null) return null;
  const best = readCandidateRows(validationRes)[0];
  return {
    sweep_size: sweepSize ?? 0,
    top_k: topK ?? 0,
    best_label: normalizeText(
      best?.params && typeof best.params === "object"
        ? (best.params as Record<string, unknown>).label
        : "",
    ),
    best_robust_score: Number(best?.robust_score ?? 0),
  };
}

function readPiniaState(storeId: string): Record<string, unknown> | null {
  const pinia = getActivePinia();
  if (!pinia) return null;
  const root = pinia.state.value as Record<string, unknown>;
  const row = root[storeId];
  if (!row || typeof row !== "object") return null;
  return row as Record<string, unknown>;
}

function markOptimizeUpdated(): void {
  const pinia = getActivePinia();
  if (!pinia) return;
  const optimize = useOptimizeStore(pinia);
  optimize.source = "vue";
  optimize.updatedAtUnixMs = Date.now();
}

function patchOptimizeState(patch: Record<string, unknown>): void {
  const pinia = getActivePinia();
  if (!pinia) return;
  const optimize = useOptimizeStore(pinia);
  Object.assign(optimize, patch);
  optimize.source = "vue";
  optimize.updatedAtUnixMs = Date.now();
}

function setMlBusy(message: string): void {
  patchOptimizeState({
    runDisabled: true,
    validateDisabled: true,
    canCancel: true,
    progressMeta: message,
    statusLine: message,
  });
}

function clearMlBusy(statusLine?: string, progressMeta?: string): void {
  patchOptimizeState({
    runDisabled: false,
    validateDisabled: false,
    canCancel: false,
    ...(statusLine != null ? { statusLine } : {}),
    ...(progressMeta != null ? { progressMeta } : {}),
  });
}

function requireDataContext(): {
  dataId: string;
  csvPath: string;
  timeframe: string;
  datasetLabel: string;
} {
  const data = readPiniaState("data");
  const dataId = normalizeText(data?.dataId);
  if (!dataId || dataId === "—") {
    throw new Error("Load a dataset first.");
  }
  return {
    dataId,
    csvPath: normalizeText(data?.csvPath),
    timeframe: normalizeText(data?.dataTimeframe),
    datasetLabel: normalizeText(data?.datasetLabel),
  };
}

function buildRunDatasetContextState(): {
  currentDataMeta: Record<string, unknown> | null;
  currentBaseDataMeta: Record<string, unknown> | null;
  currentBaseDatasetCtx: Record<string, unknown>;
  currentDatasetCtx: Record<string, unknown>;
} {
  const data = readPiniaState("data");
  const currentDataMeta =
    data?.baseDataMeta && typeof data.baseDataMeta === "object"
      ? (data.baseDataMeta as Record<string, unknown>)
      : null;
  const label = normalizeText(data?.datasetLabel);
  const timeframe = normalizeText(
    data?.dataTimeframe || currentDataMeta?.timeframe,
  );
  const source = normalizeText(data?.datasetContext);
  const symbol = normalizeText(data?.polygonSymbol || label);

  const datasetCtx: Record<string, unknown> = {};
  if (label && label !== "—") datasetCtx.label = label;
  if (symbol && symbol !== "—") datasetCtx.symbol = symbol;
  if (timeframe) {
    datasetCtx.timeframe = timeframe;
    datasetCtx.base_timeframe = timeframe;
    datasetCtx.derived_timeframe = timeframe;
    datasetCtx.resample_applied = false;
  }
  if (source && source !== "—") datasetCtx.source = source;

  return {
    currentDataMeta,
    currentBaseDataMeta: currentDataMeta,
    currentBaseDatasetCtx: datasetCtx,
    currentDatasetCtx: datasetCtx,
  };
}

function currentRoundTripCostBps(): number {
  const pinia = getActivePinia();
  if (!pinia) return 0;
  const strategy = useStrategyStore(pinia);
  const fee = Math.max(0, parseNumber(strategy.feeBps, 0));
  const spread = Math.max(0, parseNumber(strategy.spreadBps, 0));
  const slippage = Math.max(0, parseNumber(strategy.slippageBps, 0));
  return (fee + slippage + spread * 0.5) * 2;
}

function buildMlStrategyConfig(): Record<string, unknown> {
  const pinia = getActivePinia();
  if (!pinia) return {};
  const strategy = useStrategyStore(pinia);
  return {
    long_only: Boolean(strategy.longOnly),
    allow_flip: Boolean(strategy.allowFlip),
    trade_on_next_bar: Boolean(strategy.tradeOnNext),
    cost_preset: normalizeText(strategy.costPreset) || undefined,
    fee_bps: parseNumber(strategy.feeBps, 0),
    spread_bps: parseNumber(strategy.spreadBps, 0),
    slippage_bps: parseNumber(strategy.slippageBps, 0),
    stop_loss_pct: parseOptionalPositive(strategy.stopLossPct),
    take_profit_pct: parseOptionalPositive(strategy.takeProfitPct),
    cooldown_bars: Math.max(
      0,
      Math.floor(parseNumber(strategy.cooldownBars, 0)),
    ),
    signal_persistence_bars: Math.max(
      1,
      Math.floor(parseNumber(strategy.signalPersistenceBars, 1)),
    ),
    eps_rel: parseNumber(strategy.epsRel, 0),
  };
}

function buildValidationContext(): Record<string, unknown> {
  const data = requireDataContext();
  const ctx: Record<string, unknown> = {
    strategy_kind: "MlSignal",
  };
  if (data.csvPath) ctx.csv_path = data.csvPath;
  if (data.timeframe) ctx.timeframe = data.timeframe;
  if (data.datasetLabel && data.datasetLabel !== "—")
    ctx.label = data.datasetLabel;
  return ctx;
}

function buildMlSignalArtifactPayload(
  runReq: Record<string, unknown>,
  runRes: unknown,
  validationReq?: Record<string, unknown> | null,
  validationRes?: unknown,
): Record<string, unknown> {
  const runResRecord =
    runRes && typeof runRes === "object"
      ? (runRes as Record<string, unknown>)
      : null;
  return {
    bundle_id:
      normalizeText(runReq.model_bundle_id) ||
      normalizeText(runResRecord?.model_bundle_id ?? ""),
    feature_schema_version: parseNumber(
      runResRecord?.feature_schema_version ?? 1,
      1,
    ),
    feature_schema_path: normalizeText(runResRecord?.feature_schema_path ?? ""),
    score_cache_path:
      normalizeText(validationReq?.score_cache_path) ||
      normalizeText(runResRecord?.output_path ?? ""),
    score_manifest_path: normalizeText(runResRecord?.manifest_path ?? ""),
    policy:
      validationReq && validationRes
        ? extractMlPolicy(validationReq, validationRes)
        : null,
    policy_sweep:
      validationReq && validationRes
        ? extractMlPolicySweepSummary(validationRes)
        : null,
  };
}

function buildRunArtifactPayload(
  req: Record<string, unknown>,
  res: unknown,
  ts: string,
): {
  artifact: Record<string, unknown>;
  csvPath: string;
} {
  const data = requireDataContext();
  const datasetMeta = buildDatasetMetaForArtifact(
    data.csvPath,
    buildRunDatasetContextState(),
  );
  const artifact: Record<string, unknown> = {
    version: 1,
    kind: "vectorgrid_run_artifact",
    ts,
    config_fingerprint: computeConfigFingerprint(req, null),
    run: { req, res },
    validation: null,
    report_xlsx_path: "",
    data: { csv_path: data.csvPath },
    dataset_meta: datasetMeta,
    ml_signal: buildMlSignalArtifactPayload(req, res),
    tags: [],
  };
  return { artifact, csvPath: data.csvPath };
}

async function persistRunArtifact(
  req: Record<string, unknown>,
  res: unknown,
  ts: string,
): Promise<{ path: string; csvPath: string }> {
  const { artifact, csvPath } = buildRunArtifactPayload(req, res, ts);
  try {
    const saved = (await invokeCommand("save_run_artifact", {
      artifact,
    })) as SaveRunArtifactResponse;
    return {
      path: normalizeText(saved?.path),
      csvPath,
    };
  } catch {
    return { path: "", csvPath };
  }
}

function applyVueRunArtifacts(
  ts: string,
  req: Record<string, unknown>,
  res: unknown,
  artifactPath: string,
  csvPath: string,
): void {
  const datasetMeta = buildDatasetMetaForArtifact(
    csvPath,
    buildRunDatasetContextState(),
  );
  applyWorkflowArtifactsPayload(
    {
      currentRun: {
        ts,
        artifact_path: artifactPath,
        report_xlsx_path: "",
        csv_path: csvPath,
        dataset_meta: datasetMeta,
        ml_signal: buildMlSignalArtifactPayload(req, res),
        tags: [],
        req,
        res,
      },
      currentValidation: null,
    },
    "vue",
  );
}

async function persistValidationArtifact(
  runReq: Record<string, unknown>,
  runRes: unknown,
  validationReq: Record<string, unknown>,
  validationRes: unknown,
): Promise<{ path: string; ts: string; csvPath: string }> {
  const pinia = getActivePinia();
  const artifacts = pinia ? useWorkflowArtifactsStore(pinia) : null;
  const currentRun = artifacts?.currentRun as WorkflowRunArtifact | null;
  const data = requireDataContext();
  const ts = normalizeText(currentRun?.ts) || new Date().toISOString();
  const csvPath = normalizeText(currentRun?.csvPath || data.csvPath);
  const datasetMeta = buildDatasetMetaForArtifact(
    csvPath,
    buildRunDatasetContextState(),
  );
  const artifact: Record<string, unknown> = {
    version: 1,
    kind: "vectorgrid_run_artifact",
    ts,
    config_fingerprint: computeConfigFingerprint(runReq, validationReq),
    run: { req: runReq, res: runRes },
    validation: { req: validationReq, res: validationRes },
    report_xlsx_path: "",
    data: { csv_path: csvPath },
    dataset_meta: datasetMeta,
    ml_signal: buildMlSignalArtifactPayload(
      runReq,
      runRes,
      validationReq,
      validationRes,
    ),
    tags: currentRun?.tags ?? [],
  };
  let path = normalizeText(currentRun?.artifactPath);
  try {
    const saved = (await invokeCommand("save_run_artifact", {
      artifact,
    })) as SaveRunArtifactResponse;
    path = normalizeText(saved?.path) || path;
  } catch {
    // Keep validation usable even if artifact persistence fails.
  }
  return { path, ts, csvPath };
}

function applyVueValidationArtifacts(
  runReq: Record<string, unknown>,
  runRes: unknown,
  validationReq: Record<string, unknown>,
  validationRes: unknown,
  persisted: { path: string; ts: string; csvPath: string },
): void {
  const pinia = getActivePinia();
  const artifacts = pinia ? useWorkflowArtifactsStore(pinia) : null;
  applyWorkflowArtifactsPayload(
    {
      currentRun: {
        ts: persisted.ts,
        artifact_path: persisted.path,
        report_xlsx_path: "",
        csv_path: persisted.csvPath,
        dataset_meta: buildDatasetMetaForArtifact(
          persisted.csvPath,
          buildRunDatasetContextState(),
        ),
        ml_signal: buildMlSignalArtifactPayload(
          runReq,
          runRes,
          validationReq,
          validationRes,
        ),
        tags: artifacts?.currentRun?.tags ?? [],
        req: runReq,
        res: runRes,
      },
      currentValidation: {
        ts: persisted.ts,
        req: validationReq,
        res: validationRes,
      },
    },
    "vue",
  );
}

function currentRunSnapshot(): {
  req: Record<string, unknown>;
  res: unknown;
} | null {
  const pinia = getActivePinia();
  if (!pinia) return null;
  const artifacts = useWorkflowArtifactsStore(pinia);
  const run = artifacts.currentRun;
  if (!run || !run.req || typeof run.req !== "object") return null;
  return { req: run.req as Record<string, unknown>, res: run.res };
}

export async function exportMlSignalDataset(): Promise<unknown> {
  const pinia = getActivePinia();
  if (!pinia) throw new Error("Pinia is not active.");
  const optimize = useOptimizeStore(pinia);
  const data = requireDataContext();
  const outputPath = normalizeText(optimize.mlSignalDatasetExportPath);
  if (!outputPath) throw new Error("Choose a dataset export CSV path first.");
  const splitTrainBars = parseOptionalInteger(optimize.mlSignalSplitTrainBars);
  const splitCalibrationBars = parseOptionalInteger(
    optimize.mlSignalSplitCalibrationBars,
  );
  const splitTestBars = parseOptionalInteger(optimize.mlSignalSplitTestBars);
  const splitPlan =
    (splitTrainBars ?? 0) > 0 &&
    (splitCalibrationBars ?? 0) > 0 &&
    (splitTestBars ?? 0) > 0
      ? {
          version: 1,
          train_bars: splitTrainBars,
          calibration_bars: splitCalibrationBars,
          test_bars: splitTestBars,
          gap_bars: Math.max(
            0,
            parseOptionalInteger(optimize.mlSignalSplitGapBars) ?? 10,
          ),
          step_bars: Math.max(
            1,
            parseOptionalInteger(optimize.mlSignalSplitStepBars) ?? 63,
          ),
        }
      : null;
  const holdoutModulus = parseOptionalInteger(optimize.mlSignalHoldoutModulus);
  const holdoutResidue = parseOptionalInteger(optimize.mlSignalHoldoutResidue);
  const tickerHoldout =
    (holdoutModulus ?? 0) > 0
      ? {
          modulus: holdoutModulus,
          residue:
            holdoutModulus && holdoutModulus > 0
              ? Math.max(0, holdoutResidue ?? 0) % holdoutModulus
              : 0,
        }
      : null;

  const req = {
    data_id: data.dataId,
    output_path: outputPath,
    ma_source: normalizeText(optimize.mlSignalMaSource) || "close",
    horizon_bars: Math.max(
      2,
      Math.floor(parseNumber(optimize.mlSignalHorizonBars, 10)),
    ),
    round_trip_cost_bps: currentRoundTripCostBps(),
    split_plan: splitPlan,
    ticker_holdout: tickerHoldout,
  };

  setMlBusy("Exporting ML feature dataset...");
  try {
    const res = await invokeCommand("export_ml_signal_dataset", { req });
    const rowCount = parseNumber(
      (res as Record<string, unknown>)?.row_count,
      0,
    );
    const splitCount = parseOptionalInteger(
      (res as Record<string, unknown>)?.split_count,
    );
    const splitManifestPath = normalizeText(
      (res as Record<string, unknown>)?.split_manifest_path,
    );
    const tickerHoldoutEnabled = Boolean(
      (res as Record<string, unknown>)?.ticker_holdout,
    );
    const metaParts = [
      `Dataset export: ${rowCount.toFixed(0)} rows -> ${outputPath}`,
      splitCount && splitCount > 0 ? `splits ${splitCount}` : "",
      tickerHoldoutEnabled ? "ticker holdout on" : "",
      splitManifestPath ? basename(splitManifestPath) : "",
    ].filter((part) => part);
    patchOptimizeState({
      mlSignalDatasetMeta: metaParts.join(" • "),
      progressMeta: `ML dataset export • ${rowCount.toFixed(0)} rows`,
      statusLine: "ML feature dataset exported.",
    });
    return res;
  } catch (error) {
    patchOptimizeState({
      statusLine: `ML dataset export failed: ${String(error || "")}`.trim(),
    });
    throw error;
  } finally {
    clearMlBusy();
  }
}

export async function generateMlSignalScoreCache(): Promise<unknown> {
  const pinia = getActivePinia();
  if (!pinia) throw new Error("Pinia is not active.");
  const optimize = useOptimizeStore(pinia);
  const data = requireDataContext();
  const inputPath = normalizeText(optimize.mlSignalScoreInputPath);
  const outputPath = normalizeText(optimize.mlSignalScoreCachePath);
  if (!inputPath) throw new Error("Choose a model-score CSV input path first.");
  if (!outputPath) throw new Error("Choose a score-cache output path first.");

  const req = {
    data_id: data.dataId,
    input_path: inputPath,
    output_path: outputPath,
    model_bundle_id: normalizeText(optimize.mlSignalModelBundleId) || null,
    strategy_kind: "MlSignal",
  };
  const runTs = new Date().toISOString();

  setMlBusy("Generating ML score cache...");
  try {
    const res = await invokeCommand("generate_ml_signal_score_cache", { req });
    const matchedRows = parseNumber(
      (res as Record<string, unknown>)?.matched_rows,
      0,
    );
    patchOptimizeState({
      mlSignalScoreMeta: `Score cache: ${matchedRows.toFixed(0)} matched rows -> ${outputPath}`,
      progressMeta: `ML score cache • ${matchedRows.toFixed(0)} rows`,
      statusLine: "ML score cache generated.",
    });
    const persisted = await persistRunArtifact(req, res, runTs);
    applyVueRunArtifacts(runTs, req, res, persisted.path, persisted.csvPath);
    return res;
  } catch (error) {
    patchOptimizeState({
      statusLine:
        `ML score cache generation failed: ${String(error || "")}`.trim(),
    });
    throw error;
  } finally {
    clearMlBusy();
  }
}

export async function runMlSignalValidation(): Promise<unknown> {
  const pinia = getActivePinia();
  if (!pinia) throw new Error("Pinia is not active.");
  const optimize = useOptimizeStore(pinia);
  const scoreCachePath = normalizeText(optimize.mlSignalScoreCachePath);
  if (!scoreCachePath)
    throw new Error("Generate or choose a score cache path first.");
  const data = requireDataContext();

  const validationReq = {
    data_id: data.dataId,
    score_cache_path: scoreCachePath,
    candidates: [
      {
        t_long: parseNumber(optimize.mlSignalLongThreshold, 0.55),
        t_short: parseNumber(optimize.mlSignalShortThreshold, 0.55),
        delta: parseNumber(optimize.mlSignalDelta, 0.05),
        max_hold_bars: Math.max(
          1,
          Math.floor(parseNumber(optimize.mlSignalMaxHoldBars, 10)),
        ),
        exit_on_flat: Boolean(optimize.mlSignalExitOnFlat),
        exit_on_opposite: Boolean(optimize.mlSignalExitOnOpposite),
        label: "ml_signal_v1",
        strategy: buildMlStrategyConfig(),
      },
    ],
    iterative: {
      stop_loss_pct: parseOptionalPositive(useStrategyStore(pinia).stopLossPct),
      take_profit_pct: parseOptionalPositive(
        useStrategyStore(pinia).takeProfitPct,
      ),
      intrabar_rule: "pessimistic",
      slippage_vol_mult: 0,
    },
    min_psr: parseProbabilityGate(optimize.mlSignalMinPsr),
    min_dsr: parseProbabilityGate(optimize.mlSignalMinDsr),
    validation_context: buildValidationContext(),
  };

  setMlBusy("Running ML score-cache validation...");
  try {
    const validationRes = await invokeCommand("run_ml_signal_validation", {
      req: validationReq,
    });
    const candidates = Array.isArray(
      (validationRes as Record<string, unknown>)?.candidates,
    )
      ? ((validationRes as Record<string, unknown>).candidates as unknown[])
      : [];
    patchOptimizeState({
      mlSignalValidationMeta: `ML validation: ${candidates.length} candidate${candidates.length === 1 ? "" : "s"} tested`,
      progressMeta: `ML validation • ${candidates.length} candidate${candidates.length === 1 ? "" : "s"}`,
      statusLine: "ML validation completed.",
    });

    const runSnapshot = currentRunSnapshot();
    const runReq = runSnapshot?.req ?? {
      strategy_kind: "MlSignal",
      data_id: data.dataId,
      score_cache_path: scoreCachePath,
    };
    const runRes = runSnapshot?.res ?? {
      score_cache_path: scoreCachePath,
    };
    const persisted = await persistValidationArtifact(
      runReq,
      runRes,
      validationReq,
      validationRes,
    );
    applyVueValidationArtifacts(
      runReq,
      runRes,
      validationReq,
      validationRes,
      persisted,
    );
    return validationRes;
  } catch (error) {
    patchOptimizeState({
      statusLine: `ML validation failed: ${String(error || "")}`.trim(),
    });
    throw error;
  } finally {
    clearMlBusy();
  }
}

export async function runMlSignalPolicySweep(): Promise<unknown> {
  const pinia = getActivePinia();
  if (!pinia) throw new Error("Pinia is not active.");
  const optimize = useOptimizeStore(pinia);
  const scoreCachePath = normalizeText(optimize.mlSignalScoreCachePath);
  if (!scoreCachePath)
    throw new Error("Generate or choose a score cache path first.");
  const data = requireDataContext();

  const sweepReq = {
    data_id: data.dataId,
    score_cache_path: scoreCachePath,
    t_long: buildOptionalSweepAxisF64(
      optimize.mlSignalSweepLongStart,
      optimize.mlSignalSweepLongEnd,
      optimize.mlSignalSweepLongStep,
    ),
    t_short: buildOptionalSweepAxisF64(
      optimize.mlSignalSweepShortStart,
      optimize.mlSignalSweepShortEnd,
      optimize.mlSignalSweepShortStep,
    ),
    delta: buildOptionalSweepAxisF64(
      optimize.mlSignalSweepDeltaStart,
      optimize.mlSignalSweepDeltaEnd,
      optimize.mlSignalSweepDeltaStep,
    ),
    max_hold_bars: buildOptionalSweepAxisUsize(
      optimize.mlSignalSweepHoldStart,
      optimize.mlSignalSweepHoldEnd,
      optimize.mlSignalSweepHoldStep,
    ),
    exit_on_flat_values: [Boolean(optimize.mlSignalExitOnFlat)],
    exit_on_opposite_values: [Boolean(optimize.mlSignalExitOnOpposite)],
    base_strategy: buildMlStrategyConfig(),
    iterative: {
      stop_loss_pct: parseOptionalPositive(useStrategyStore(pinia).stopLossPct),
      take_profit_pct: parseOptionalPositive(
        useStrategyStore(pinia).takeProfitPct,
      ),
      intrabar_rule: "pessimistic",
      slippage_vol_mult: 0,
    },
    min_psr: parseProbabilityGate(optimize.mlSignalMinPsr),
    min_dsr: parseProbabilityGate(optimize.mlSignalMinDsr),
    validation_context: buildValidationContext(),
    top_k: Math.max(1, Math.floor(parseNumber(optimize.mlSignalSweepTopK, 10))),
  };

  setMlBusy("Running ML policy sweep...");
  try {
    const sweepRes = await invokeCommand("run_ml_signal_policy_sweep", {
      req: sweepReq,
    });
    const sweepRow =
      sweepRes && typeof sweepRes === "object"
        ? (sweepRes as Record<string, unknown>)
        : {};
    const sweepSize = Math.max(
      0,
      Math.floor(parseNumber(sweepRow.sweep_size, 0)),
    );
    const candidates = readCandidateRows(sweepRes);
    const best = candidates[0];
    const bestScore = Number(best?.robust_score ?? Number.NaN);
    const bestLabel = normalizeText(
      best?.params && typeof best.params === "object"
        ? (best.params as Record<string, unknown>).label
        : "",
    );
    patchOptimizeState({
      mlSignalSweepMeta: [
        `ML policy sweep: ${sweepSize.toFixed(0)} candidates`,
        Number.isFinite(bestScore) ? `best ${bestScore.toFixed(4)}` : "",
        bestLabel || "",
      ]
        .filter((part) => part)
        .join(" • "),
      mlSignalValidationMeta: `ML validation: ${candidates.length} sweep candidate${candidates.length === 1 ? "" : "s"} retained`,
      progressMeta: `ML policy sweep • ${sweepSize.toFixed(0)} candidate${sweepSize === 1 ? "" : "s"}`,
      statusLine: "ML policy sweep completed.",
    });

    const runSnapshot = currentRunSnapshot();
    const runReq = runSnapshot?.req ?? {
      strategy_kind: "MlSignal",
      data_id: data.dataId,
      score_cache_path: scoreCachePath,
    };
    const runRes = runSnapshot?.res ?? {
      score_cache_path: scoreCachePath,
    };
    const persisted = await persistValidationArtifact(
      runReq,
      runRes,
      sweepReq,
      sweepRes,
    );
    applyVueValidationArtifacts(runReq, runRes, sweepReq, sweepRes, persisted);
    return sweepRes;
  } catch (error) {
    patchOptimizeState({
      statusLine: `ML policy sweep failed: ${String(error || "")}`.trim(),
    });
    throw error;
  } finally {
    clearMlBusy();
  }
}

export async function listMlSignalModelBundles(): Promise<
  Array<{
    bundleId: string;
    manifestPath: string;
    state: string;
    label: string;
  }>
> {
  const raw = (await invokeCommand("list_ml_signal_model_bundles")) as unknown;
  if (!Array.isArray(raw)) return [];
  return raw
    .map((entry) => {
      const row = (
        entry && typeof entry === "object" ? entry : {}
      ) as MlSignalModelBundleSummary;
      const bundleId = normalizeText(row.bundle_id);
      const state = normalizeText(row.state) || "unknown";
      const family = normalizeText(row.model_family);
      return {
        bundleId,
        manifestPath: normalizeText(row.manifest_path),
        state,
        label: family
          ? `${bundleId} • ${family} • ${state}`
          : `${bundleId} • ${state}`,
      };
    })
    .filter((row) => row.bundleId);
}

export async function refreshMlSignalModelStatus(): Promise<unknown> {
  const pinia = getActivePinia();
  if (!pinia) throw new Error("Pinia is not active.");
  const optimize = useOptimizeStore(pinia);
  const bundleId = normalizeText(optimize.mlSignalModelBundleId);
  if (!bundleId) {
    patchOptimizeState({
      mlSignalModelStatus:
        "Model bundle: optional for score-cache-backed validation.",
    });
    return null;
  }
  const raw = (await invokeCommand("ml_signal_model_status", {
    bundleId,
  })) as MlSignalModelStatusResponse;
  const status =
    raw && typeof raw.status === "object"
      ? (raw.status as Record<string, unknown>)
      : null;
  const state = normalizeText(status?.state) || "missing";
  const manifestPath = normalizeText(status?.manifest_path);
  patchOptimizeState({
    mlSignalModelStatus: manifestPath
      ? `Model bundle: ${bundleId} • ${state} • ${manifestPath}`
      : `Model bundle: ${bundleId} • ${state}`,
  });
  return raw;
}

export async function openMlSignalArtifactPath(path: string): Promise<unknown> {
  return openOrRevealPath(path);
}

export async function revealMlSignalArtifactPath(
  path: string,
): Promise<unknown> {
  const nextPath = normalizeText(path);
  if (!nextPath) throw new Error("Path is required.");
  await invokeCommand("reveal_path", { path: nextPath });
  return { path: nextPath, openedWith: "reveal_path" };
}

export function setMlSignalPathField(field: string, value: string): void {
  const pinia = getActivePinia();
  if (!pinia) return;
  const optimize = useOptimizeStore(pinia);
  optimize.setInputField(field, value);
  markOptimizeUpdated();
}
