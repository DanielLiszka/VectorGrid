<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { storeToRefs } from "pinia";
import { useDataStore } from "../stores/dataStore";
import { useOptimizeStore } from "../stores/optimizeStore";
import { useStrategyStore } from "../stores/strategyStore";
import { useWorkflowArtifactsStore } from "../stores/workflowArtifactsStore";
import { useWorkflowStore } from "../stores/workflowStore";
import {
  exportMlSignalDataset,
  generateMlSignalScoreCache,
  listMlSignalModelBundles,
  openMlSignalArtifactPath,
  refreshMlSignalModelStatus,
  revealMlSignalArtifactPath,
  runMlSignalPolicySweep,
  runMlSignalValidation,
} from "../services/mlSignalService";
import { invokeCommand, tauriRuntimeAvailable } from "../tauri/invoke";

interface BundleOption {
  bundleId: string;
  manifestPath: string;
  state: string;
  label: string;
}

const optimizeStore = useOptimizeStore();
const dataStore = useDataStore();
const strategyStore = useStrategyStore();
const workflowArtifactsStore = useWorkflowArtifactsStore();
const workflowStore = useWorkflowStore();

const {
  mlSignalDatasetExportPath,
  mlSignalScoreInputPath,
  mlSignalScoreCachePath,
  mlSignalModelBundleId,
  mlSignalModelStatus,
  mlSignalDatasetMeta,
  mlSignalScoreMeta,
  mlSignalValidationMeta,
  mlSignalSweepMeta,
  mlSignalMaSource,
  mlSignalHorizonBars,
  mlSignalSplitTrainBars,
  mlSignalSplitCalibrationBars,
  mlSignalSplitTestBars,
  mlSignalSplitGapBars,
  mlSignalSplitStepBars,
  mlSignalHoldoutModulus,
  mlSignalHoldoutResidue,
  mlSignalLongThreshold,
  mlSignalShortThreshold,
  mlSignalDelta,
  mlSignalMaxHoldBars,
  mlSignalMinPsr,
  mlSignalMinDsr,
  mlSignalSweepLongStart,
  mlSignalSweepLongEnd,
  mlSignalSweepLongStep,
  mlSignalSweepShortStart,
  mlSignalSweepShortEnd,
  mlSignalSweepShortStep,
  mlSignalSweepDeltaStart,
  mlSignalSweepDeltaEnd,
  mlSignalSweepDeltaStep,
  mlSignalSweepHoldStart,
  mlSignalSweepHoldEnd,
  mlSignalSweepHoldStep,
  mlSignalSweepTopK,
  mlSignalExitOnFlat,
  mlSignalExitOnOpposite,
  statusLine,
  progressMeta,
  runDisabled,
  validateDisabled,
  canCancel,
} = storeToRefs(optimizeStore);
const { currentRun, currentValidation } = storeToRefs(workflowArtifactsStore);
const { mode, workflowStep } = storeToRefs(workflowStore);

const bundleOptions = ref<BundleOption[]>([]);
const bundleRefreshBusy = ref(false);
const hasTauriRuntime = computed(() => tauriRuntimeAvailable());
const datasetLoaded = computed(() => {
  const dataId = String(dataStore.dataId || "").trim();
  return !!dataId && dataId !== "—";
});
const strategyExecutionSummary = computed(() => {
  const parts: string[] = [];
  parts.push(strategyStore.longOnly ? "long-only" : "long+short");
  parts.push(strategyStore.allowFlip ? "flip enabled" : "no flip");
  parts.push(
    strategyStore.tradeOnNext ? "next-bar execution" : "same-bar execution",
  );
  parts.push(`fees ${String(strategyStore.feeBps || "0").trim() || "0"} bps`);
  parts.push(
    `spread ${String(strategyStore.spreadBps || "0").trim() || "0"} bps`,
  );
  parts.push(
    `slippage ${String(strategyStore.slippageBps || "0").trim() || "0"} bps`,
  );
  if (String(strategyStore.stopLossPct || "").trim())
    parts.push(`stop ${String(strategyStore.stopLossPct).trim()}`);
  if (String(strategyStore.takeProfitPct || "").trim())
    parts.push(`take ${String(strategyStore.takeProfitPct).trim()}`);
  parts.push(
    `cooldown ${String(strategyStore.cooldownBars || "0").trim() || "0"}`,
  );
  parts.push(
    `persistence ${String(strategyStore.signalPersistenceBars || "1").trim() || "1"}`,
  );
  return parts.join(" • ");
});

const canExportDataset = computed(
  () =>
    datasetLoaded.value &&
    !!String(mlSignalDatasetExportPath.value || "").trim() &&
    !runDisabled.value,
);
const canBuildScoreCache = computed(
  () =>
    datasetLoaded.value &&
    !!String(mlSignalScoreInputPath.value || "").trim() &&
    !!String(mlSignalScoreCachePath.value || "").trim() &&
    !runDisabled.value,
);
const canRunValidation = computed(
  () =>
    datasetLoaded.value &&
    !!String(mlSignalScoreCachePath.value || "").trim() &&
    !validateDisabled.value,
);
const canRunSweep = computed(
  () =>
    datasetLoaded.value &&
    !!String(mlSignalScoreCachePath.value || "").trim() &&
    !validateDisabled.value,
);

function basename(path: string): string {
  const normalized = String(path || "").trim();
  if (!normalized) return "—";
  return normalized.split(/[\\/]/).pop() || normalized;
}

function parseNumber(value: unknown, fallback: number): number {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function formatPolicySummary(
  policy: Record<string, unknown> | null | undefined,
): string {
  if (!policy) return "—";
  const label = String(policy.label || "").trim();
  const tLong = Number(policy.t_long ?? policy.tLong);
  const tShort = Number(policy.t_short ?? policy.tShort);
  const delta = Number(policy.delta);
  const hold = Number(policy.max_hold_bars ?? policy.maxHoldBars);
  const parts = [
    label,
    Number.isFinite(tLong) ? `tL ${tLong.toFixed(2)}` : "",
    Number.isFinite(tShort) ? `tS ${tShort.toFixed(2)}` : "",
    Number.isFinite(delta) ? `d ${delta.toFixed(2)}` : "",
    Number.isFinite(hold) ? `hold ${Math.max(1, Math.floor(hold))}` : "",
  ].filter((part) => part);
  return parts.join(" • ") || "—";
}

function formatSweepSummary(
  sweep: Record<string, unknown> | null | undefined,
): string {
  if (!sweep) return "—";
  const sweepSize = Number(sweep.sweep_size ?? sweep.sweepSize);
  const topK = Number(sweep.top_k ?? sweep.topK);
  const bestLabel = String((sweep.best_label ?? sweep.bestLabel) || "").trim();
  const bestScore = Number(sweep.best_robust_score ?? sweep.bestRobustScore);
  const parts = [
    Number.isFinite(sweepSize)
      ? `size ${Math.max(0, Math.floor(sweepSize))}`
      : "",
    Number.isFinite(topK) ? `top ${Math.max(0, Math.floor(topK))}` : "",
    bestLabel,
    Number.isFinite(bestScore) ? `best ${bestScore.toFixed(4)}` : "",
  ].filter((part) => part);
  return parts.join(" • ") || "—";
}

function formatCandidatePolicy(candidate: Record<string, unknown>): string {
  const params =
    candidate.params && typeof candidate.params === "object"
      ? (candidate.params as Record<string, unknown>)
      : null;
  return formatPolicySummary(params);
}

function formatMetric(value: unknown, digits = 4): string {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed.toFixed(digits) : "—";
}

function formatNestedMetric(
  row: Record<string, unknown>,
  parentKey: string,
  metricKey: string,
  digits = 4,
): string {
  const parent =
    row[parentKey] && typeof row[parentKey] === "object"
      ? (row[parentKey] as Record<string, unknown>)
      : null;
  return formatMetric(parent?.[metricKey], digits);
}

const currentMlArtifact = computed(() => currentRun.value?.mlSignal || null);
const currentValidationResult = computed(() => {
  const validation = currentValidation.value;
  if (!validation?.res || typeof validation.res !== "object") return null;
  const row = validation.res as Record<string, unknown>;
  return row.validation && typeof row.validation === "object"
    ? (row.validation as Record<string, unknown>)
    : row;
});
const currentValidationCandidates = computed(() => {
  const row = currentValidationResult.value;
  if (!row) return [];
  const candidates = Array.isArray(row.candidates) ? row.candidates : [];
  return candidates
    .filter(
      (entry): entry is Record<string, unknown> =>
        !!entry && typeof entry === "object",
    )
    .slice(0, 3);
});
const currentValidationVerdictSummary = computed(() => {
  const result = currentValidationResult.value;
  if (!result) return "No run verdict yet.";
  const verdict =
    result.run_verdict && typeof result.run_verdict === "object"
      ? (result.run_verdict as Record<string, unknown>)
      : null;
  if (!verdict) return "No run verdict yet.";
  const approved = Number(verdict.approved_candidate_count);
  const survived = Number(verdict.candidates_survived_hard_gates);
  const total = Number(verdict.candidates_considered);
  const parts = [
    Number.isFinite(approved)
      ? `approved ${Math.max(0, Math.floor(approved))}`
      : "",
    Number.isFinite(survived)
      ? `survived ${Math.max(0, Math.floor(survived))}`
      : "",
    Number.isFinite(total)
      ? `considered ${Math.max(0, Math.floor(total))}`
      : "",
  ].filter((part) => part);
  return parts.join(" • ") || "No run verdict yet.";
});
const currentValidationSummary = computed(() => {
  const validation = currentValidation.value;
  if (!validation?.res || typeof validation.res !== "object")
    return "No ML validation artifact attached yet.";
  const row = validation.res as Record<string, unknown>;
  const sweepSize = Number(row.sweep_size);
  const candidates = Array.isArray(row.candidates)
    ? row.candidates
    : row.validation &&
        typeof row.validation === "object" &&
        Array.isArray((row.validation as Record<string, unknown>).candidates)
      ? ((row.validation as Record<string, unknown>).candidates as unknown[])
      : [];
  if (Number.isFinite(sweepSize)) {
    return `Policy sweep attached • ${Math.max(0, Math.floor(sweepSize))} candidates searched • ${candidates.length} retained`;
  }
  return `Single validation attached • ${candidates.length} candidate${candidates.length === 1 ? "" : "s"}`;
});

const datasetExportSplitSummary = computed(() => {
  const trainBars = Number(mlSignalSplitTrainBars.value);
  const calibrationBars = Number(mlSignalSplitCalibrationBars.value);
  const testBars = Number(mlSignalSplitTestBars.value);
  const splitEnabled =
    Number.isFinite(trainBars) &&
    trainBars > 0 &&
    Number.isFinite(calibrationBars) &&
    calibrationBars > 0 &&
    Number.isFinite(testBars) &&
    testBars > 0;
  if (!splitEnabled) return "Plain dataset export";
  const gapBars = Math.max(0, Number(mlSignalSplitGapBars.value || 0) || 0);
  const stepBars = Math.max(1, Number(mlSignalSplitStepBars.value || 1) || 1);
  return `train ${Math.floor(trainBars)} • calibration ${Math.floor(calibrationBars)} • test ${Math.floor(testBars)} • gap ${Math.floor(gapBars)} • step ${Math.floor(stepBars)}`;
});

const datasetExportHoldoutSummary = computed(() => {
  const modulus = Math.max(0, Number(mlSignalHoldoutModulus.value || 0) || 0);
  if (modulus <= 0) return "Ticker holdout off";
  const residueRaw = Math.max(0, Number(mlSignalHoldoutResidue.value || 0) || 0);
  const residue = modulus > 0 ? residueRaw % modulus : 0;
  return `ticker_id % ${Math.floor(modulus)} == ${Math.floor(residue)}`;
});

const datasetExportArtifactSummary = computed(() => {
  const artifact = currentMlArtifact.value;
  const exportPath = String(artifact?.datasetExportPath || "").trim();
  if (!exportPath) return "No dataset export artifact attached yet.";
  const parts = [basename(exportPath)];
  if (artifact?.featureSchemaPath) parts.push(`schema ${basename(String(artifact.featureSchemaPath))}`);
  if (artifact?.exportManifestPath) parts.push(`manifest ${basename(String(artifact.exportManifestPath))}`);
  if (artifact?.splitManifestPath) parts.push(`splits ${basename(String(artifact.splitManifestPath))}`);
  return parts.join(" • ");
});

const featureSchemaContractSummary = computed(() => {
  const artifact = currentMlArtifact.value;
  const schemaPath = String(artifact?.featureSchemaPath || "").trim();
  const version = Number(artifact?.featureSchemaVersion);
  if (schemaPath && Number.isFinite(version) && version > 0) {
    return `Schema v${Math.floor(version)} is required for score-cache compatibility and saved-run reopen consistency.`;
  }
  if (schemaPath) {
    return "Feature schema path is attached, but the schema version is missing.";
  }
  return "No feature schema attached to the current ML artifact.";
});

const currentScoreCoverageSummary = computed(() => {
  const artifact = currentMlArtifact.value;
  if (!artifact) return "No score-cache summary attached yet.";
  const parts = [
    String(artifact.scoreSpace || "").trim(),
    String(artifact.scoreScope || "").trim(),
    Number.isFinite(Number(artifact.matchedRows))
      ? `matched ${Math.max(0, Math.floor(Number(artifact.matchedRows)))}`
      : "",
    Number.isFinite(Number(artifact.skippedRows))
      ? `skipped ${Math.max(0, Math.floor(Number(artifact.skippedRows)))}`
      : "",
    Number.isFinite(Number(artifact.barsScored))
      ? `bars ${Math.max(0, Math.floor(Number(artifact.barsScored)))}`
      : "",
    Number.isFinite(Number(artifact.coverageRatio))
      ? `coverage ${(Number(artifact.coverageRatio) * 100).toFixed(1)}%`
      : "",
    Number.isFinite(Number(artifact.duplicateTimestamps))
      ? `duplicates ${Math.max(
          0,
          Math.floor(Number(artifact.duplicateTimestamps)),
        )}`
      : "",
  ].filter((part) => part);
  return parts.join(" • ") || "No score-cache summary attached yet.";
});

const validationContextSummary = computed(() => {
  const parts = ["MlSignal"];
  const label = String(dataStore.datasetLabel || "").trim();
  const timeframe = String(dataStore.dataTimeframe || "").trim();
  const csvPath = String(dataStore.csvPath || "").trim();
  if (label && label !== "—") parts.push(label);
  if (timeframe) parts.push(`timeframe ${timeframe}`);
  if (csvPath) parts.push(`csv ${basename(csvPath)}`);
  return parts.join(" • ");
});

const currentPolicyControlSummary = computed(() => {
  return [
    `tL ${parseNumber(mlSignalLongThreshold.value, 0.55).toFixed(2)}`,
    `tS ${parseNumber(mlSignalShortThreshold.value, 0.55).toFixed(2)}`,
    `d ${parseNumber(mlSignalDelta.value, 0.05).toFixed(2)}`,
    `hold ${Math.max(
      1,
      Math.floor(parseNumber(mlSignalMaxHoldBars.value, 10)),
    )}`,
    mlSignalExitOnFlat.value ? "exit flat on" : "exit flat off",
    mlSignalExitOnOpposite.value ? "exit opposite on" : "exit opposite off",
  ].join(" • ");
});

const scoreRequirementSummary = computed(() => {
  const base =
    "Matching data_id, loaded csv_path, and timeframe are required before validation starts.";
  if (strategyStore.longOnly) {
    return `${base} Current strategy is long-only, so long probability coverage is sufficient. Raw-margin-only caches are still rejected for threshold validation.`;
  }
  return `${base} Current strategy is long+short, so validation requires both long and short probability coverage. Raw-margin-only caches are rejected for threshold validation.`;
});

const currentSweepAxisSummary = computed(() => {
  return [
    `long ${parseNumber(mlSignalSweepLongStart.value, 0.5).toFixed(2)}-${parseNumber(
      mlSignalSweepLongEnd.value,
      0.7,
    ).toFixed(2)}@${parseNumber(mlSignalSweepLongStep.value, 0.1).toFixed(2)}`,
    `short ${parseNumber(
      mlSignalSweepShortStart.value,
      0.5,
    ).toFixed(2)}-${parseNumber(mlSignalSweepShortEnd.value, 0.7).toFixed(
      2,
    )}@${parseNumber(mlSignalSweepShortStep.value, 0.1).toFixed(2)}`,
    `delta ${parseNumber(
      mlSignalSweepDeltaStart.value,
      0,
    ).toFixed(2)}-${parseNumber(mlSignalSweepDeltaEnd.value, 0.1).toFixed(
      2,
    )}@${parseNumber(mlSignalSweepDeltaStep.value, 0.05).toFixed(2)}`,
    `hold ${Math.max(
      1,
      Math.floor(parseNumber(mlSignalSweepHoldStart.value, 5)),
    )}-${Math.max(
      1,
      Math.floor(parseNumber(mlSignalSweepHoldEnd.value, 15)),
    )}@${Math.max(
      1,
      Math.floor(parseNumber(mlSignalSweepHoldStep.value, 5)),
    )}`,
  ].join(" • ");
});

const currentSweepBudgetSummary = computed(() => {
  const topK = Math.max(1, Math.floor(parseNumber(mlSignalSweepTopK.value, 10)));
  return `UI retain top ${topK}. Rust still defaults top_k to 25 when omitted, but this panel always sends an explicit top_k. Candidate expansion stays capped at 20,000.`;
});

const currentArtifactStageSummary = computed(() => {
  const stage = String(currentMlArtifact.value?.stage || "").trim();
  switch (stage) {
    case "dataset_export":
      return "Stage 1 owns the dataset CSV, feature schema, export manifest, and optional split manifest.";
    case "score_cache":
      return "Stage 2 owns the score cache and score manifest. Dataset-export sidecars carry forward for replay and compatibility checks.";
    case "validation":
      return "Stage 3 owns the validation result and current policy selection. Dataset-export and score-cache attachments carry forward for replay.";
    case "policy_sweep":
      return "Stage 4 owns the retained sweep summary and best-policy view. Dataset-export and score-cache attachments carry forward for replay.";
    default:
      return "ML artifacts keep stage-owned outputs plus carry-forward dataset and score-cache attachments for replay.";
  }
});

async function browseSaveCsv(
  field: "mlSignalDatasetExportPath" | "mlSignalScoreCachePath",
  defaultName: string,
): Promise<void> {
  if (!hasTauriRuntime.value) return;
  const picked = await invokeCommand<string | null>("pick_save_csv", {
    defaultName,
  });
  const path = String(picked || "").trim();
  if (!path) return;
  optimizeStore.setInputField(field, path);
}

async function browseInputCsv(): Promise<void> {
  if (!hasTauriRuntime.value) return;
  const picked = await invokeCommand<string | null>("pick_csv_file");
  const path = String(picked || "").trim();
  if (!path) return;
  optimizeStore.setInputField("mlSignalScoreInputPath", path);
}

async function refreshBundleOptions(): Promise<void> {
  if (!hasTauriRuntime.value) return;
  bundleRefreshBusy.value = true;
  try {
    bundleOptions.value = await listMlSignalModelBundles();
  } finally {
    bundleRefreshBusy.value = false;
  }
}

async function refreshBundleStatus(): Promise<void> {
  if (!hasTauriRuntime.value) return;
  try {
    await refreshMlSignalModelStatus();
  } catch {
    // keep UI non-throwing
  }
}

async function onExportDataset(): Promise<void> {
  try {
    await exportMlSignalDataset();
  } catch {
    // status line is updated in the service
  }
}

async function onBuildScoreCache(): Promise<void> {
  try {
    await generateMlSignalScoreCache();
  } catch {
    // status line is updated in the service
  }
}

async function onRunValidation(): Promise<void> {
  try {
    await runMlSignalValidation();
  } catch {
    // status line is updated in the service
  }
}

async function onRunPolicySweep(): Promise<void> {
  try {
    await runMlSignalPolicySweep();
  } catch {
    // status line is updated in the service
  }
}

async function onOpenArtifactPath(path: string): Promise<void> {
  if (!hasTauriRuntime.value || !String(path || "").trim()) return;
  try {
    await openMlSignalArtifactPath(path);
  } catch {
    // non-throwing from panel
  }
}

async function onRevealArtifactPath(path: string): Promise<void> {
  if (!hasTauriRuntime.value || !String(path || "").trim()) return;
  try {
    await revealMlSignalArtifactPath(path);
  } catch {
    // non-throwing from panel
  }
}

function stageAnchorSelector(step: string): string {
  return `[data-ml-stage="${step}"]`;
}

async function syncStageScroll(step: string): Promise<void> {
  if (String(mode.value || "").trim() !== "ml-signal") return;
  const normalizedStep = String(step || "").trim();
  if (!normalizedStep || normalizedStep === "data") return;
  await nextTick();
  const target = document.querySelector<HTMLElement>(
    stageAnchorSelector(normalizedStep),
  );
  target?.scrollIntoView?.({ behavior: "smooth", block: "start" });
}

watch(mlSignalModelBundleId, () => {
  void refreshBundleStatus();
});

watch(workflowStep, (step) => {
  void syncStageScroll(String(step || "").trim());
});

onMounted(() => {
  void syncStageScroll(String(workflowStep.value || "").trim());
  if (!hasTauriRuntime.value) return;
  void refreshBundleOptions();
  void refreshBundleStatus();
});
</script>

<template>
  <div class="mlSignalRoot">
    <section class="pane paneProminent">
      <div class="paneHeader">
        <div>
          <div class="sectionEyebrow">Alternative Mode</div>
          <h3>Pattern Recognition + ML signal validation</h3>
        </div>
      </div>
      <div class="muted">
        This path is additive. It does not replace the existing optimization,
        validation, meta-analysis, or AI/ML analysis workflow. It exports
        research datasets, imports external probability scores, and validates
        score-to-position policy rules against the existing simulator.
      </div>
      <div class="muted mono">
        Loaded dataset: {{ dataStore.datasetLabel || "—" }} • data_id={{
          dataStore.dataId || "—"
        }}
        • timeframe={{ dataStore.dataTimeframe || "—" }}
      </div>
      <div class="muted mono">
        Execution settings come from Strategy Controls:
        {{ strategyExecutionSummary }}
      </div>
      <div v-if="!hasTauriRuntime" class="muted mono">
        Desktop-only commands are unavailable in this plain browser session.
      </div>
    </section>

    <section class="pane" data-ml-stage="data">
      <div class="paneHeader">
        <div>
          <div class="sectionEyebrow">Stage 1</div>
          <h3>Dataset export</h3>
        </div>
      </div>
      <div class="gridTwo">
        <div>
          <label for="mlSignalMaSource">MA source</label>
          <input
            id="mlSignalMaSource"
            :value="mlSignalMaSource"
            @input="
              optimizeStore.setInputField(
                'mlSignalMaSource',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
        <div>
          <label for="mlSignalHorizonBars">Horizon bars</label>
          <input
            id="mlSignalHorizonBars"
            type="number"
            min="2"
            step="1"
            :value="mlSignalHorizonBars"
            @input="
              optimizeStore.setInputField(
                'mlSignalHorizonBars',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
      </div>
      <div class="sectionNote mono">
        Optional split manifest: leave the train/calibration/test fields blank
        to export a plain dataset, or fill them to emit a walk-forward
        `split_manifest.json` sidecar.
      </div>
      <div class="summaryGrid" aria-label="ML dataset export contract">
        <article>
          <div class="panelMetricKey">Split plan</div>
          <div class="panelMetricValue mono">{{ datasetExportSplitSummary }}</div>
        </article>
        <article>
          <div class="panelMetricKey">Ticker holdout</div>
          <div class="panelMetricValue mono">{{ datasetExportHoldoutSummary }}</div>
        </article>
        <article>
          <div class="panelMetricKey">Current export artifact</div>
          <div class="panelMetricValue mono">{{ datasetExportArtifactSummary }}</div>
        </article>
      </div>
      <div class="muted">
        Gap bars separate train to calibration and calibration to test windows. Step bars move the next emitted split window forward when split-manifest generation is active. In the current export contract, blank gap and step fields resolve to gap 0 and step 1.
      </div>
      <div class="muted">
        Dataset export always writes the CSV, feature schema, and
        <span class="mono">.export_manifest.json</span> sidecar. The
        <span class="mono">.split_manifest.json</span> sidecar is emitted only
        when the split-plan fields are active. Score-cache manifests are Stage
        2 artifacts and remain separate from dataset-export manifests.
      </div>
      <div class="gridTwo">
        <div>
          <label for="mlSignalSplitTrainBars">Train bars</label>
          <input
            id="mlSignalSplitTrainBars"
            type="number"
            min="0"
            step="1"
            :value="mlSignalSplitTrainBars"
            @input="
              optimizeStore.setInputField(
                'mlSignalSplitTrainBars',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
        <div>
          <label for="mlSignalSplitCalibrationBars">Calibration bars</label>
          <input
            id="mlSignalSplitCalibrationBars"
            type="number"
            min="0"
            step="1"
            :value="mlSignalSplitCalibrationBars"
            @input="
              optimizeStore.setInputField(
                'mlSignalSplitCalibrationBars',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
      </div>
      <div class="gridTwo">
        <div>
          <label for="mlSignalSplitTestBars">Test bars</label>
          <input
            id="mlSignalSplitTestBars"
            type="number"
            min="0"
            step="1"
            :value="mlSignalSplitTestBars"
            @input="
              optimizeStore.setInputField(
                'mlSignalSplitTestBars',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
        <div>
          <label for="mlSignalSplitGapBars">Gap bars</label>
          <input
            id="mlSignalSplitGapBars"
            type="number"
            min="0"
            step="1"
            :value="mlSignalSplitGapBars"
            @input="
              optimizeStore.setInputField(
                'mlSignalSplitGapBars',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
      </div>
      <div class="gridTwo">
        <div>
          <label for="mlSignalSplitStepBars">Step bars</label>
          <input
            id="mlSignalSplitStepBars"
            type="number"
            min="1"
            step="1"
            :value="mlSignalSplitStepBars"
            @input="
              optimizeStore.setInputField(
                'mlSignalSplitStepBars',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
        <div>
          <label for="mlSignalHoldoutModulus">Ticker holdout modulus</label>
          <input
            id="mlSignalHoldoutModulus"
            type="number"
            min="0"
            step="1"
            :value="mlSignalHoldoutModulus"
            @input="
              optimizeStore.setInputField(
                'mlSignalHoldoutModulus',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
      </div>
      <div class="gridTwo">
        <div>
          <label for="mlSignalHoldoutResidue">Ticker holdout residue</label>
          <input
            id="mlSignalHoldoutResidue"
            type="number"
            min="0"
            step="1"
            :value="mlSignalHoldoutResidue"
            @input="
              optimizeStore.setInputField(
                'mlSignalHoldoutResidue',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
        <div class="muted">
          Holdout uses a stable `ticker_id % modulus == residue` slice for
          offline research only.
        </div>
      </div>
      <label for="mlSignalDatasetExportPath">Dataset CSV output</label>
      <div class="inputWithAction">
        <input
          id="mlSignalDatasetExportPath"
          :value="mlSignalDatasetExportPath"
          @input="
            optimizeStore.setInputField(
              'mlSignalDatasetExportPath',
              String(($event.target as HTMLInputElement).value || ''),
            )
          "
        />
        <button
          type="button"
          :disabled="!hasTauriRuntime"
          @click="
            void browseSaveCsv(
              'mlSignalDatasetExportPath',
              'ml_signal_dataset.csv',
            )
          "
        >
          Browse
        </button>
      </div>
      <div class="actions">
        <button
          type="button"
          class="primary"
          :disabled="!canExportDataset"
          @click="void onExportDataset()"
        >
          Export dataset
        </button>
      </div>
      <div class="muted mono">{{ mlSignalDatasetMeta }}</div>
    </section>

    <section class="pane" data-ml-stage="optimize">
      <div class="paneHeader">
        <div>
          <div class="sectionEyebrow">Stage 2</div>
          <h3>Score cache</h3>
        </div>
        <button
          type="button"
          :disabled="!hasTauriRuntime || bundleRefreshBusy"
          @click="void refreshBundleOptions()"
        >
          Refresh bundles
        </button>
      </div>
      <div class="gridTwo">
        <div>
          <label for="mlSignalBundleId">Model bundle</label>
          <select
            id="mlSignalBundleId"
            :value="mlSignalModelBundleId"
            @change="
              optimizeStore.setInputField(
                'mlSignalModelBundleId',
                String(($event.target as HTMLSelectElement).value || ''),
              )
            "
          >
            <option value="">Optional</option>
            <option
              v-for="option in bundleOptions"
              :key="`ml_bundle_${option.bundleId}`"
              :value="option.bundleId"
            >
              {{ option.label }}
            </option>
          </select>
        </div>
        <div>
          <label>&nbsp;</label>
          <button
            type="button"
            :disabled="!hasTauriRuntime"
            @click="void refreshBundleStatus()"
          >
            Refresh bundle status
          </button>
        </div>
      </div>
      <div class="muted mono">{{ mlSignalModelStatus }}</div>
      <div class="muted">
        Model bundles are discovered from the desktop app-local inventory.
        Bundle selection is optional for score-cache-backed validation. When a
        bundle is selected, the status line above should name the current bundle
        state and manifest path.
      </div>
      <label for="mlSignalScoreInputPath">Model score CSV input</label>
      <div class="inputWithAction">
        <input
          id="mlSignalScoreInputPath"
          :value="mlSignalScoreInputPath"
          @input="
            optimizeStore.setInputField(
              'mlSignalScoreInputPath',
              String(($event.target as HTMLInputElement).value || ''),
            )
          "
        />
        <button
          type="button"
          :disabled="!hasTauriRuntime"
          @click="void browseInputCsv()"
        >
          Browse
        </button>
      </div>
      <div class="muted">
        Score CSV ingest contract: include
        <span class="mono">timestamp</span> plus at least one of
        <span class="mono">p_long</span>,
        <span class="mono">p_short</span>,
        <span class="mono">raw_long</span>, or
        <span class="mono">raw_short</span>. If calibrated columns like
        <span class="mono">p_long_cal</span> /
        <span class="mono">p_short_cal</span> are present, they take priority
        over the uncalibrated probability columns.
      </div>
      <label for="mlSignalScoreCachePath">Score cache output</label>
      <div class="inputWithAction">
        <input
          id="mlSignalScoreCachePath"
          :value="mlSignalScoreCachePath"
          @input="
            optimizeStore.setInputField(
              'mlSignalScoreCachePath',
              String(($event.target as HTMLInputElement).value || ''),
            )
          "
        />
        <button
          type="button"
          :disabled="!hasTauriRuntime"
          @click="
            void browseSaveCsv(
              'mlSignalScoreCachePath',
              'ml_signal_score_cache.json',
            )
          "
        >
          Browse
        </button>
      </div>
      <div class="actions">
        <button
          type="button"
          class="primary"
          :disabled="!canBuildScoreCache"
          @click="void onBuildScoreCache()"
        >
          Build score cache
        </button>
      </div>
      <div class="muted">
        Score-cache build writes a JSON cache plus a
        <span class="mono">.manifest.json</span> sidecar. Raw-margin-only caches
        can build successfully, but threshold validation and policy sweep later
        require probability scores.
      </div>
      <div class="muted mono">{{ mlSignalScoreMeta }}</div>
    </section>

    <section class="pane" data-ml-stage="validate">
      <div class="paneHeader">
        <div>
          <div class="sectionEyebrow">Stage 3</div>
          <h3>Policy validation</h3>
        </div>
      </div>
      <div class="gridTwo">
        <div>
          <label for="mlSignalLongThreshold">Long threshold</label>
          <input
            id="mlSignalLongThreshold"
            type="number"
            min="0"
            max="1"
            step="0.01"
            :value="mlSignalLongThreshold"
            @input="
              optimizeStore.setInputField(
                'mlSignalLongThreshold',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
        <div>
          <label for="mlSignalShortThreshold">Short threshold</label>
          <input
            id="mlSignalShortThreshold"
            type="number"
            min="0"
            max="1"
            step="0.01"
            :value="mlSignalShortThreshold"
            @input="
              optimizeStore.setInputField(
                'mlSignalShortThreshold',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
      </div>
      <div class="gridTwo">
        <div>
          <label for="mlSignalDelta">Delta</label>
          <input
            id="mlSignalDelta"
            type="number"
            min="0"
            max="1"
            step="0.01"
            :value="mlSignalDelta"
            @input="
              optimizeStore.setInputField(
                'mlSignalDelta',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
        <div>
          <label for="mlSignalMaxHoldBars">Max hold bars</label>
          <input
            id="mlSignalMaxHoldBars"
            type="number"
            min="1"
            step="1"
            :value="mlSignalMaxHoldBars"
            @input="
              optimizeStore.setInputField(
                'mlSignalMaxHoldBars',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
      </div>
      <div class="gridTwo">
        <div>
          <label for="mlSignalMinPsr">Min PSR gate</label>
          <input
            id="mlSignalMinPsr"
            type="number"
            min="0"
            max="1"
            step="0.01"
            :value="mlSignalMinPsr"
            @input="
              optimizeStore.setInputField(
                'mlSignalMinPsr',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
        <div>
          <label for="mlSignalMinDsr">Min DSR gate</label>
          <input
            id="mlSignalMinDsr"
            type="number"
            min="0"
            max="1"
            step="0.01"
            :value="mlSignalMinDsr"
            @input="
              optimizeStore.setInputField(
                'mlSignalMinDsr',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
      </div>
      <div class="summaryGrid" aria-label="ML validation contract">
        <article>
          <div class="panelMetricKey">Validation context</div>
          <div class="panelMetricValue mono">{{ validationContextSummary }}</div>
        </article>
        <article>
          <div class="panelMetricKey">Current policy</div>
          <div class="panelMetricValue mono">
            {{ currentPolicyControlSummary }}
          </div>
        </article>
        <article>
          <div class="panelMetricKey">Score requirements</div>
          <div class="panelMetricValue mono">{{ scoreRequirementSummary }}</div>
        </article>
      </div>
      <div class="toggleRow">
        <label class="inlineToggle">
          <input
            type="checkbox"
            :checked="mlSignalExitOnFlat"
            @change="
              optimizeStore.setCheckboxField(
                'mlSignalExitOnFlat',
                !!($event.target as HTMLInputElement).checked,
              )
            "
          />
          Exit on flat target
        </label>
        <label class="inlineToggle">
          <input
            type="checkbox"
            :checked="mlSignalExitOnOpposite"
            @change="
              optimizeStore.setCheckboxField(
                'mlSignalExitOnOpposite',
                !!($event.target as HTMLInputElement).checked,
              )
            "
          />
          Exit or flip on opposite target
        </label>
      </div>
      <div class="actions">
        <button
          type="button"
          class="primary"
          :disabled="!canRunValidation"
          @click="void onRunValidation()"
        >
          Run ML validation
        </button>
        <button
          type="button"
          :disabled="!canCancel"
          @click="optimizeStore.cancelOptimization()"
        >
          Cancel
        </button>
      </div>
      <div class="muted mono">{{ mlSignalValidationMeta }}</div>
      <div class="muted mono">{{ progressMeta || "—" }}</div>
      <div class="muted mono" role="status" aria-live="polite">{{ statusLine }}</div>
    </section>

    <section class="pane" data-ml-stage="analyze">
      <div class="paneHeader">
        <div>
          <div class="sectionEyebrow">Stage 4</div>
          <h3>Policy sweep</h3>
        </div>
      </div>
      <div class="sectionNote mono">
        Sweep cached score streams across thresholds and hold windows, then keep
        the best `top_k` candidates by robust score.
      </div>
      <div class="summaryGrid" aria-label="ML policy sweep contract">
        <article>
          <div class="panelMetricKey">Sweep axes</div>
          <div class="panelMetricValue mono">{{ currentSweepAxisSummary }}</div>
        </article>
        <article>
          <div class="panelMetricKey">Inherited exit policy</div>
          <div class="panelMetricValue mono">
            {{ currentPolicyControlSummary }}
          </div>
        </article>
        <article>
          <div class="panelMetricKey">Budget</div>
          <div class="panelMetricValue mono">{{ currentSweepBudgetSummary }}</div>
        </article>
      </div>
      <div class="gridThree">
        <div>
          <label for="mlSignalSweepLongStart">Long start</label>
          <input
            id="mlSignalSweepLongStart"
            type="number"
            min="0"
            max="1"
            step="0.01"
            :value="mlSignalSweepLongStart"
            @input="
              optimizeStore.setInputField(
                'mlSignalSweepLongStart',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
        <div>
          <label for="mlSignalSweepLongEnd">Long end</label>
          <input
            id="mlSignalSweepLongEnd"
            type="number"
            min="0"
            max="1"
            step="0.01"
            :value="mlSignalSweepLongEnd"
            @input="
              optimizeStore.setInputField(
                'mlSignalSweepLongEnd',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
        <div>
          <label for="mlSignalSweepLongStep">Long step</label>
          <input
            id="mlSignalSweepLongStep"
            type="number"
            min="0.001"
            max="1"
            step="0.01"
            :value="mlSignalSweepLongStep"
            @input="
              optimizeStore.setInputField(
                'mlSignalSweepLongStep',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
      </div>
      <div class="gridThree">
        <div>
          <label for="mlSignalSweepShortStart">Short start</label>
          <input
            id="mlSignalSweepShortStart"
            type="number"
            min="0"
            max="1"
            step="0.01"
            :value="mlSignalSweepShortStart"
            @input="
              optimizeStore.setInputField(
                'mlSignalSweepShortStart',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
        <div>
          <label for="mlSignalSweepShortEnd">Short end</label>
          <input
            id="mlSignalSweepShortEnd"
            type="number"
            min="0"
            max="1"
            step="0.01"
            :value="mlSignalSweepShortEnd"
            @input="
              optimizeStore.setInputField(
                'mlSignalSweepShortEnd',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
        <div>
          <label for="mlSignalSweepShortStep">Short step</label>
          <input
            id="mlSignalSweepShortStep"
            type="number"
            min="0.001"
            max="1"
            step="0.01"
            :value="mlSignalSweepShortStep"
            @input="
              optimizeStore.setInputField(
                'mlSignalSweepShortStep',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
      </div>
      <div class="gridThree">
        <div>
          <label for="mlSignalSweepDeltaStart">Delta start</label>
          <input
            id="mlSignalSweepDeltaStart"
            type="number"
            min="0"
            max="1"
            step="0.01"
            :value="mlSignalSweepDeltaStart"
            @input="
              optimizeStore.setInputField(
                'mlSignalSweepDeltaStart',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
        <div>
          <label for="mlSignalSweepDeltaEnd">Delta end</label>
          <input
            id="mlSignalSweepDeltaEnd"
            type="number"
            min="0"
            max="1"
            step="0.01"
            :value="mlSignalSweepDeltaEnd"
            @input="
              optimizeStore.setInputField(
                'mlSignalSweepDeltaEnd',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
        <div>
          <label for="mlSignalSweepDeltaStep">Delta step</label>
          <input
            id="mlSignalSweepDeltaStep"
            type="number"
            min="0.001"
            max="1"
            step="0.01"
            :value="mlSignalSweepDeltaStep"
            @input="
              optimizeStore.setInputField(
                'mlSignalSweepDeltaStep',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
      </div>
      <div class="gridThree">
        <div>
          <label for="mlSignalSweepHoldStart">Hold start</label>
          <input
            id="mlSignalSweepHoldStart"
            type="number"
            min="1"
            step="1"
            :value="mlSignalSweepHoldStart"
            @input="
              optimizeStore.setInputField(
                'mlSignalSweepHoldStart',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
        <div>
          <label for="mlSignalSweepHoldEnd">Hold end</label>
          <input
            id="mlSignalSweepHoldEnd"
            type="number"
            min="1"
            step="1"
            :value="mlSignalSweepHoldEnd"
            @input="
              optimizeStore.setInputField(
                'mlSignalSweepHoldEnd',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
        <div>
          <label for="mlSignalSweepHoldStep">Hold step</label>
          <input
            id="mlSignalSweepHoldStep"
            type="number"
            min="1"
            step="1"
            :value="mlSignalSweepHoldStep"
            @input="
              optimizeStore.setInputField(
                'mlSignalSweepHoldStep',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
      </div>
      <div class="gridTwo">
        <div>
          <label for="mlSignalSweepTopK">Retain top K</label>
          <input
            id="mlSignalSweepTopK"
            type="number"
            min="1"
            step="1"
            :value="mlSignalSweepTopK"
            @input="
              optimizeStore.setInputField(
                'mlSignalSweepTopK',
                String(($event.target as HTMLInputElement).value || ''),
              )
            "
          />
        </div>
        <div class="muted">
          Exit behavior still comes from the single-policy toggles above. This
          keeps the v1 sweep bounded and predictable.
        </div>
      </div>
      <div class="actions">
        <button
          type="button"
          class="primary"
          :disabled="!canRunSweep"
          @click="void onRunPolicySweep()"
        >
          Run ML policy sweep
        </button>
      </div>
      <div class="muted mono">{{ mlSignalSweepMeta }}</div>
    </section>

    <section class="pane" data-ml-stage="finalize">
      <div class="paneHeader">
        <div>
          <div class="sectionEyebrow">Stage 5</div>
          <h3>Current ML artifact</h3>
        </div>
      </div>
      <div class="summaryList">
        <div class="summaryRow">
          <span class="summaryKey">Stage</span>
          <span class="summaryValue mono">
            {{ currentMlArtifact?.stage || "—" }}
          </span>
        </div>
        <div class="summaryRow">
          <span class="summaryKey">Dataset export</span>
          <div class="summaryValue summaryValueActions">
            <span
              class="mono"
              :title="currentMlArtifact?.datasetExportPath || ''"
            >
              {{ basename(currentMlArtifact?.datasetExportPath || "") }}
            </span>
            <div class="rowActions">
              <button
                type="button"
                :disabled="
                  !hasTauriRuntime || !currentMlArtifact?.datasetExportPath
                "
                @click="
                  void onOpenArtifactPath(
                    currentMlArtifact?.datasetExportPath || '',
                  )
                "
              >
                Open
              </button>
              <button
                type="button"
                :disabled="
                  !hasTauriRuntime || !currentMlArtifact?.datasetExportPath
                "
                @click="
                  void onRevealArtifactPath(
                    currentMlArtifact?.datasetExportPath || '',
                  )
                "
              >
                Reveal
              </button>
            </div>
          </div>
        </div>
        <div class="summaryRow">
          <span class="summaryKey">Export manifest</span>
          <div class="summaryValue summaryValueActions">
            <span
              class="mono"
              :title="currentMlArtifact?.exportManifestPath || ''"
            >
              {{ basename(currentMlArtifact?.exportManifestPath || "") }}
            </span>
            <div class="rowActions">
              <button
                type="button"
                :disabled="
                  !hasTauriRuntime || !currentMlArtifact?.exportManifestPath
                "
                @click="
                  void onOpenArtifactPath(
                    currentMlArtifact?.exportManifestPath || '',
                  )
                "
              >
                Open
              </button>
              <button
                type="button"
                :disabled="
                  !hasTauriRuntime || !currentMlArtifact?.exportManifestPath
                "
                @click="
                  void onRevealArtifactPath(
                    currentMlArtifact?.exportManifestPath || '',
                  )
                "
              >
                Reveal
              </button>
            </div>
          </div>
        </div>
        <div class="summaryRow">
          <span class="summaryKey">Split manifest</span>
          <div class="summaryValue summaryValueActions">
            <span
              class="mono"
              :title="currentMlArtifact?.splitManifestPath || ''"
            >
              {{ basename(currentMlArtifact?.splitManifestPath || "") }}
            </span>
            <div class="rowActions">
              <button
                type="button"
                :disabled="
                  !hasTauriRuntime || !currentMlArtifact?.splitManifestPath
                "
                @click="
                  void onOpenArtifactPath(
                    currentMlArtifact?.splitManifestPath || '',
                  )
                "
              >
                Open
              </button>
              <button
                type="button"
                :disabled="
                  !hasTauriRuntime || !currentMlArtifact?.splitManifestPath
                "
                @click="
                  void onRevealArtifactPath(
                    currentMlArtifact?.splitManifestPath || '',
                  )
                "
              >
                Reveal
              </button>
            </div>
          </div>
        </div>
        <div class="summaryRow">
          <span class="summaryKey">Bundle</span>
          <span
            class="summaryValue mono"
            :title="currentMlArtifact?.bundleId || ''"
          >
            {{ currentMlArtifact?.bundleId || "—" }}
          </span>
        </div>
        <div class="summaryRow">
          <span class="summaryKey">Score cache</span>
          <div class="summaryValue summaryValueActions">
            <span class="mono" :title="currentMlArtifact?.scoreCachePath || ''">
              {{ basename(currentMlArtifact?.scoreCachePath || "") }}
            </span>
            <div class="rowActions">
              <button
                type="button"
                :disabled="
                  !hasTauriRuntime || !currentMlArtifact?.scoreCachePath
                "
                @click="
                  void onOpenArtifactPath(
                    currentMlArtifact?.scoreCachePath || '',
                  )
                "
              >
                Open
              </button>
              <button
                type="button"
                :disabled="
                  !hasTauriRuntime || !currentMlArtifact?.scoreCachePath
                "
                @click="
                  void onRevealArtifactPath(
                    currentMlArtifact?.scoreCachePath || '',
                  )
                "
              >
                Reveal
              </button>
            </div>
          </div>
        </div>
        <div class="summaryRow">
          <span class="summaryKey">Score manifest</span>
          <div class="summaryValue summaryValueActions">
            <span
              class="mono"
              :title="currentMlArtifact?.scoreManifestPath || ''"
            >
              {{ basename(currentMlArtifact?.scoreManifestPath || "") }}
            </span>
            <div class="rowActions">
              <button
                type="button"
                :disabled="
                  !hasTauriRuntime || !currentMlArtifact?.scoreManifestPath
                "
                @click="
                  void onOpenArtifactPath(
                    currentMlArtifact?.scoreManifestPath || '',
                  )
                "
              >
                Open
              </button>
              <button
                type="button"
                :disabled="
                  !hasTauriRuntime || !currentMlArtifact?.scoreManifestPath
                "
                @click="
                  void onRevealArtifactPath(
                    currentMlArtifact?.scoreManifestPath || '',
                  )
                "
              >
                Reveal
              </button>
            </div>
          </div>
        </div>
        <div class="summaryRow">
          <span class="summaryKey">Feature schema</span>
          <div class="summaryValue summaryValueActions">
            <span
              class="mono"
              :title="currentMlArtifact?.featureSchemaPath || ''"
            >
              {{ basename(currentMlArtifact?.featureSchemaPath || "") }}
            </span>
            <div class="rowActions">
              <button
                type="button"
                :disabled="
                  !hasTauriRuntime || !currentMlArtifact?.featureSchemaPath
                "
                @click="
                  void onOpenArtifactPath(
                    currentMlArtifact?.featureSchemaPath || '',
                  )
                "
              >
                Open
              </button>
              <button
                type="button"
                :disabled="
                  !hasTauriRuntime || !currentMlArtifact?.featureSchemaPath
                "
                @click="
                  void onRevealArtifactPath(
                    currentMlArtifact?.featureSchemaPath || '',
                  )
                "
              >
                Reveal
              </button>
            </div>
          </div>
        </div>
        <div class="summaryRow">
          <span class="summaryKey">Schema version</span>
          <span class="summaryValue mono">
            {{ currentMlArtifact?.featureSchemaVersion || "—" }}
          </span>
        </div>
        <div class="summaryRow">
          <span class="summaryKey">Score summary</span>
          <span class="summaryValue mono">{{ currentScoreCoverageSummary }}</span>
        </div>
        <div class="muted">
          {{ featureSchemaContractSummary }}
        </div>
        <div class="muted">
          {{ currentArtifactStageSummary }}
        </div>
        <div class="summaryRow">
          <span class="summaryKey">Policy</span>
          <span class="summaryValue mono">
            {{ formatPolicySummary(currentMlArtifact?.policy) }}
          </span>
        </div>
        <div class="summaryRow">
          <span class="summaryKey">Sweep</span>
          <span class="summaryValue mono">
            {{ formatSweepSummary(currentMlArtifact?.policySweep) }}
          </span>
        </div>
        <div class="summaryRow">
          <span class="summaryKey">Validation</span>
          <span class="summaryValue mono">{{ currentValidationSummary }}</span>
        </div>
      </div>
      <div
        v-if="currentValidationCandidates.length > 0"
        class="candidateList"
        aria-label="Current ML candidates"
      >
        <div class="candidateHeader">Top retained candidates</div>
        <div class="muted mono">{{ currentValidationVerdictSummary }}</div>
        <div
          v-for="(candidate, idx) in currentValidationCandidates"
          :key="`ml_candidate_${idx}`"
          class="candidateCard mono"
        >
          <div class="candidateTitleRow">
            <span>{{ idx + 1 }}.</span>
            <span>{{ formatCandidatePolicy(candidate) }}</span>
            <span>robust {{ formatMetric(candidate.robust_score) }}</span>
          </div>
          <div class="candidateMetrics">
            <span
              >sharpe
              {{ formatNestedMetric(candidate, "in_sample", "sharpe") }}</span
            >
            <span
              >pnl {{ formatNestedMetric(candidate, "in_sample", "pnl") }}</span
            >
            <span
              >dd
              {{ formatNestedMetric(candidate, "in_sample", "max_dd") }}</span
            >
            <span
              >trades
              {{
                formatNestedMetric(candidate, "in_sample", "trades", 0)
              }}</span
            >
            <span>psr {{ formatMetric(candidate.psr) }}</span>
            <span>dsr {{ formatMetric(candidate.dsr) }}</span>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.mlSignalRoot {
  display: grid;
  gap: 0.85rem;
}

.pane {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel-footer);
  padding: 0.75rem 0.8rem;
  min-width: 0;
}

.paneProminent {
  background: linear-gradient(
    135deg,
    color-mix(in oklab, var(--accent-soft) 60%, var(--panel-footer)),
    color-mix(in oklab, var(--panel-footer) 92%, var(--surface))
  );
}

.paneHeader {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
  margin-bottom: 0.65rem;
}

.sectionEyebrow {
  font-size: 0.69rem;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--muted);
}

.pane h3 {
  margin: 0.12rem 0 0;
  font-size: 0.92rem;
}

.gridTwo {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr));
  gap: 0.7rem;
}

.gridThree {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
  gap: 0.7rem;
}

.gridTwo + label,
label + .inputWithAction,
label + input,
.inputWithAction + label {
  margin-top: 0.7rem;
}

.sectionNote {
  margin-top: 0.65rem;
}

.inputWithAction {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.45rem;
}

.actions {
  margin-top: 0.6rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.actions button {
  padding: 0.42rem 0.6rem;
  font-size: 0.76rem;
}

.actions .primary {
  border-color: color-mix(in oklab, var(--accent) 70%, var(--border));
}

.toggleRow {
  margin-top: 0.7rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
}

.inlineToggle {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
}

.muted {
  color: var(--muted);
  font-size: 0.8rem;
  margin-top: 0.4rem;
}

.mono {
  font-family: var(--mono);
}

.summaryGrid {
  margin-top: 0.65rem;
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.65rem;
}

.summaryGrid article {
  border: 1px solid var(--border);
  border-radius: 10px;
  background: color-mix(in oklab, var(--panel-footer) 92%, var(--surface));
  padding: 0.55rem 0.6rem;
  min-width: 0;
}

.panelMetricKey {
  font-size: 0.74rem;
  color: var(--muted);
}

.panelMetricValue {
  margin-top: 0.24rem;
  min-width: 0;
  overflow-wrap: anywhere;
}

.summaryList {
  display: grid;
  gap: 0.45rem;
}

.summaryRow {
  display: grid;
  grid-template-columns: minmax(7rem, auto) minmax(0, 1fr);
  gap: 0.7rem;
  align-items: start;
}

.summaryKey {
  font-size: 0.76rem;
  color: var(--muted);
}

.summaryValue {
  min-width: 0;
  overflow-wrap: anywhere;
}

.summaryValueActions {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.rowActions {
  display: flex;
  gap: 0.35rem;
  flex-wrap: wrap;
}

.rowActions button {
  padding: 0.2rem 0.45rem;
  font-size: 0.72rem;
}

.candidateList {
  margin-top: 0.75rem;
  display: grid;
  gap: 0.35rem;
}

.candidateHeader {
  font-size: 0.76rem;
  color: var(--muted);
}

.candidateCard {
  display: grid;
  gap: 0.35rem;
  padding: 0.45rem 0.5rem;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: color-mix(in oklab, var(--panel-footer) 92%, var(--surface));
}

.candidateTitleRow {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  gap: 0.5rem;
  align-items: start;
  font-size: 0.75rem;
  overflow-wrap: anywhere;
}

.candidateMetrics {
  display: flex;
  gap: 0.55rem;
  flex-wrap: wrap;
  font-size: 0.73rem;
  color: var(--muted);
}

@media (max-width: 980px) {
  .summaryGrid,
  .gridTwo,
  .gridThree,
  .inputWithAction {
    grid-template-columns: 1fr;
  }

  .paneHeader {
    flex-direction: column;
  }

  .candidateTitleRow {
    grid-template-columns: 1fr;
  }
}
</style>
