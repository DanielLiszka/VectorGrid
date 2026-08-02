<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { storeToRefs } from "pinia";
import {
  AlertTriangle,
  BriefcaseBusiness,
  CheckCircle2,
  Download,
  Play,
  RefreshCw,
} from "@lucide/vue";
import { useShellMount } from "../composables/useShellMount";
import {
  analyzePortfolioFeasibility,
  type PortfolioFeasibilityResult,
} from "../services/portfolioService";
import {
  loadSavedRunDetail,
  type HistorySavedRunSummary,
} from "../services/historyService";
import { useHistoryStore } from "../stores/historyStore";
import { useWorkflowArtifactsStore } from "../stores/workflowArtifactsStore";
import { fileBasename } from "../utils/core/path";
import {
  buildCurrentRunPortfolioSource,
  buildCurrentValidationPortfolioSource,
  buildPortfolioConstructionReadiness,
  buildPortfolioFeasibilityCsv,
  buildPortfolioFeasibilityExportPayload,
  buildSavedRunPortfolioSource,
  formatPortfolioEvidenceStatus,
  formatPortfolioNumber,
  formatPortfolioReplayStatus,
  type PortfolioWorkspaceSource,
} from "../utils/portfolioWorkspaceModel";
import CompactEvidenceValue from "./CompactEvidenceValue.vue";
import AppButton from "./ui/AppButton.vue";
import AppEmptyState from "./ui/AppEmptyState.vue";

const workflowArtifactsStore = useWorkflowArtifactsStore();
const historyStore = useHistoryStore();
const { currentRun, currentValidation } = storeToRefs(workflowArtifactsStore);
const {
  savedRunsRaw,
  savedRunsVisible,
  savedRunsMeta,
  statusLine: historyStatusLine,
  canRefreshSavedRuns,
  updatedAtUnixMs: historyUpdatedAtUnixMs,
} = storeToRefs(historyStore);

const { mountReady } = useShellMount("#portfolioVueShellMount", {
  paneSelector: "#portfolioShellGrid",
  paneDatasetKey: "vuePortfolio",
});

const includeCurrentRun = ref(true);
const includeCurrentValidation = ref(true);
const selectedSavedPaths = ref<string[]>([]);
const maxCandidates = ref(50);
const requireApprovedValidation = ref(true);
const requireReplayMatrix = ref(false);
const requireSourceProvenance = ref(false);
type PortfolioOperation = "analyze" | "refresh";
type PortfolioPanelState = "idle" | "loading" | "success" | "error";

const activeOperation = ref<PortfolioOperation | null>(null);
const lastOperation = ref<PortfolioOperation | null>(null);
const panelState = ref<PortfolioPanelState>("idle");
const refreshBaselineUpdatedAt = ref(0);
const refreshBaselineStatus = ref("");
const statusLine = ref("Select one or more sources, then run Analyze.");
const errorLine = ref("");
const result = ref<PortfolioFeasibilityResult | null>(null);
const analyzedSources = ref<PortfolioWorkspaceSource[]>([]);

const currentRunSource = computed(() =>
  buildCurrentRunPortfolioSource(currentRun.value),
);
const currentValidationSource = computed(() =>
  buildCurrentValidationPortfolioSource(
    currentRun.value,
    currentValidation.value,
  ),
);
const savedRunRows = computed<HistorySavedRunSummary[]>(() => {
  const visible = savedRunsVisible.value.length
    ? savedRunsVisible.value
    : savedRunsRaw.value;
  return visible.slice(0, 80);
});
const selectedSavedSet = computed(() => new Set(selectedSavedPaths.value));
const selectedSavedCount = computed(() => selectedSavedPaths.value.length);
const effectiveMaxCandidates = computed(() => {
  const parsed = Number(maxCandidates.value);
  if (!Number.isFinite(parsed) || parsed <= 0) return 50;
  return Math.min(250, Math.floor(parsed));
});
const sourceCountPreview = computed(() => {
  let count = 0;
  if (includeCurrentRun.value && currentRunSource.value) count += 1;
  if (includeCurrentValidation.value && currentValidationSource.value)
    count += 1;
  count += selectedSavedPaths.value.length;
  return count;
});
const analyzing = computed(() => activeOperation.value === "analyze");
const refreshingSources = computed(() => activeOperation.value === "refresh");
const operationBusy = computed(() => activeOperation.value !== null);
const canAnalyze = computed(
  () => sourceCountPreview.value > 0 && !operationBusy.value,
);
const canRefreshSources = computed(
  () => canRefreshSavedRuns.value && !operationBusy.value,
);
const showingRefreshCompletion = computed(
  () =>
    panelState.value === "success" &&
    lastOperation.value === "refresh" &&
    statusLine.value === "Saved run sources refreshed.",
);
const statusTone = computed(() => {
  if (panelState.value === "error") return "bad";
  if (panelState.value === "loading") return "loading";
  if (panelState.value === "idle") return "neutral";
  if (showingRefreshCompletion.value) return "good";
  const status = result.value?.status || "";
  if (status === "replay_parity_passed") return "good";
  if (
    status === "artifact_provenance_blocked" ||
    status === "unsupported_candidate_family"
  )
    return "bad";
  return "warn";
});
const statusTitle = computed(() => {
  if (panelState.value === "loading") {
    return lastOperation.value === "refresh"
      ? "Refreshing saved sources"
      : "Analyzing portfolio feasibility";
  }
  if (panelState.value === "error") {
    return lastOperation.value === "refresh"
      ? "Saved source refresh failed"
      : "Analysis failed";
  }
  if (showingRefreshCompletion.value) {
    return "Saved sources refreshed";
  }
  if (result.value) return formatPortfolioEvidenceStatus(result.value.status);
  return "Not analyzed";
});
const statusIcon = computed(() => {
  if (statusTone.value === "good") return CheckCircle2;
  if (statusTone.value === "neutral") return BriefcaseBusiness;
  if (statusTone.value === "loading") return RefreshCw;
  return AlertTriangle;
});
const statusRole = computed(() =>
  panelState.value === "error" ? "alert" : "status",
);
const statusLive = computed(() =>
  panelState.value === "error" ? "assertive" : "polite",
);
const latestRequestGates = computed(() => ({
  maxCandidates: effectiveMaxCandidates.value,
  requireApprovedValidation: requireApprovedValidation.value,
  requireReplayMatrix: requireReplayMatrix.value,
  requireSourceProvenance: requireSourceProvenance.value,
}));
const constructionReadiness = computed(() =>
  buildPortfolioConstructionReadiness(result.value),
);
const canExport = computed(
  () =>
    !!result.value &&
    analyzedSources.value.length > 0 &&
    !operationBusy.value,
);

watch(savedRunRows, (rows) => {
  const allowed = new Set(rows.map((row) => row.path).filter(Boolean));
  selectedSavedPaths.value = selectedSavedPaths.value.filter((path) =>
    allowed.has(path),
  );
});

watch(
  [historyUpdatedAtUnixMs, historyStatusLine],
  ([updatedAt, historyStatus]) => {
    if (!refreshingSources.value) return;
    if (
      updatedAt === refreshBaselineUpdatedAt.value &&
      historyStatus === refreshBaselineStatus.value
    ) {
      return;
    }

    activeOperation.value = null;
    if (historyStatus === "Refreshed saved runs.") {
      panelState.value = "success";
      errorLine.value = "";
      statusLine.value = "Saved run sources refreshed.";
      return;
    }

    panelState.value = "error";
    errorLine.value =
      String(historyStatus || "").trim() || "Failed to refresh saved runs.";
    statusLine.value = "Saved run source refresh failed.";
  },
);

onMounted(() => {
  if (!savedRunsRaw.value.length) refreshSavedRuns();
});

function savedRunLabel(entry: HistorySavedRunSummary): string {
  return (
    [entry.ts, entry.strategyKind, entry.timeframe]
      .filter(Boolean)
      .join(" | ") ||
    fileBasename(entry.path) ||
    "Saved run artifact"
  );
}

function savedRunDetail(entry: HistorySavedRunSummary): string {
  return entry.hasValidation ? "validation" : "no validation";
}

function toggleSavedPath(path: string, checked: boolean): void {
  const next = new Set(selectedSavedPaths.value);
  if (checked) next.add(path);
  else next.delete(path);
  selectedSavedPaths.value = Array.from(next);
}

function selectLatestSavedRuns(): void {
  selectedSavedPaths.value = savedRunRows.value
    .slice(0, 5)
    .map((row) => row.path)
    .filter(Boolean);
}

function clearSavedRuns(): void {
  selectedSavedPaths.value = [];
}

function refreshSavedRuns(): void {
  if (!canRefreshSources.value) return;
  lastOperation.value = "refresh";
  activeOperation.value = "refresh";
  panelState.value = "loading";
  errorLine.value = "";
  refreshBaselineUpdatedAt.value = historyUpdatedAtUnixMs.value;
  refreshBaselineStatus.value = historyStatusLine.value;
  statusLine.value = "Refreshing saved run sources.";
  try {
    historyStore.refreshSavedRuns();
  } catch (error) {
    activeOperation.value = null;
    panelState.value = "error";
    errorLine.value =
      String(error instanceof Error ? error.message : error || "").trim() ||
      "Failed to refresh saved runs.";
    statusLine.value = "Saved run source refresh failed.";
  }
}

async function loadSelectedSavedSources(): Promise<PortfolioWorkspaceSource[]> {
  const out: PortfolioWorkspaceSource[] = [];
  for (const path of selectedSavedPaths.value) {
    const detail = await loadSavedRunDetail(path);
    const source = buildSavedRunPortfolioSource(detail);
    if (source) out.push(source);
  }
  return out;
}

async function runFeasibility(): Promise<void> {
  if (!canAnalyze.value) return;
  lastOperation.value = "analyze";
  activeOperation.value = "analyze";
  panelState.value = "loading";
  errorLine.value = "";
  result.value = null;
  analyzedSources.value = [];
  statusLine.value = `Preparing ${sourceCountPreview.value} source${sourceCountPreview.value === 1 ? "" : "s"}.`;
  try {
    const sources: PortfolioWorkspaceSource[] = [];
    if (includeCurrentRun.value && currentRunSource.value) {
      sources.push(currentRunSource.value);
    }
    if (includeCurrentValidation.value && currentValidationSource.value) {
      sources.push(currentValidationSource.value);
    }
    sources.push(...(await loadSelectedSavedSources()));
    if (!sources.length) {
      throw new Error(
        "Select at least one run, validation, or saved artifact source.",
      );
    }
    analyzedSources.value = sources;
    statusLine.value = `Analyzing ${sources.length} source${sources.length === 1 ? "" : "s"}.`;
    result.value = await analyzePortfolioFeasibility({
      sourceArtifacts: sources.map((source) => source.artifact),
      maxCandidates: effectiveMaxCandidates.value,
      requireApprovedValidation: requireApprovedValidation.value,
      requireReplayMatrix: requireReplayMatrix.value,
      requireSourceProvenance: requireSourceProvenance.value,
    });
    panelState.value = "success";
    statusLine.value = `Analyzed ${sources.length} source${sources.length === 1 ? "" : "s"}.`;
  } catch (error) {
    const message = String(
      error instanceof Error ? error.message : error || "",
    ).trim();
    errorLine.value = message || "Portfolio feasibility analysis failed.";
    panelState.value = "error";
    statusLine.value = "Portfolio feasibility analysis failed.";
  } finally {
    if (analyzing.value) activeOperation.value = null;
  }
}

function downloadText(filename: string, text: string, type: string): void {
  if (typeof document === "undefined") return;
  const blob = new Blob([text], { type });
  const url = URL.createObjectURL(blob);
  try {
    const link = document.createElement("a");
    link.href = url;
    link.download = filename;
    link.click();
  } finally {
    URL.revokeObjectURL(url);
  }
}

function exportFeasibilityJson(): void {
  if (!result.value || !canExport.value) return;
  const payload = buildPortfolioFeasibilityExportPayload(
    result.value,
    analyzedSources.value,
    latestRequestGates.value,
  );
  downloadText(
    `portfolio-feasibility-${payload.generated_at_unix_ms}.json`,
    `${JSON.stringify(payload, null, 2)}\n`,
    "application/json",
  );
  statusLine.value = "Exported compact portfolio feasibility JSON.";
}

function exportFeasibilityCsv(): void {
  if (!result.value || !canExport.value) return;
  const csv = buildPortfolioFeasibilityCsv(
    result.value,
    analyzedSources.value,
    latestRequestGates.value,
  );
  downloadText(`portfolio-feasibility-${Date.now()}.csv`, csv, "text/csv");
  statusLine.value = "Exported compact portfolio feasibility CSV.";
}
</script>

<template>
  <Teleport v-if="mountReady" to="#portfolioVueShellMount">
    <section
      class="portfolioWorkspace"
      data-qa-panel="portfolio.feasibility"
      :data-state="panelState"
      :data-operation="activeOperation || undefined"
      :aria-busy="operationBusy ? 'true' : 'false'"
    >
      <header class="portfolioHeader">
        <div class="titleBlock">
          <span class="eyebrow">Portfolio Research</span>
          <h2><BriefcaseBusiness aria-hidden="true" /> Feasibility</h2>
          <p>
            Admission evidence, replay gaps, and provenance checks for shortlist
            candidates.
          </p>
        </div>
        <div class="headerActions">
          <AppButton
            size="sm"
            variant="secondary"
            :icon="RefreshCw"
            :loading="refreshingSources"
            loading-label="Refreshing…"
            :disabled="!canRefreshSources"
            :disabled-reason="
              !canRefreshSavedRuns
                ? 'Saved-run refresh is unavailable.'
                : operationBusy
                  ? 'Another portfolio action is in progress.'
                  : ''
            "
            data-qa-action="portfolio.refreshSources"
            @click="refreshSavedRuns"
          >
            Refresh
          </AppButton>
          <AppButton
            size="sm"
            variant="primary"
            :icon="Play"
            :loading="analyzing"
            loading-label="Analyzing…"
            :disabled="!canAnalyze"
            :disabled-reason="
              operationBusy
                ? 'Another portfolio action is in progress.'
                : sourceCountPreview === 0
                  ? 'Select at least one portfolio source.'
                  : ''
            "
            data-qa-action="portfolio.runFeasibility"
            @click="runFeasibility"
          >
            Analyze
          </AppButton>
        </div>
      </header>

      <div class="portfolioGrid">
        <section
          class="sourcePane"
          aria-label="Portfolio feasibility sources"
          :aria-busy="refreshingSources ? 'true' : 'false'"
          :data-state="
            refreshingSources
              ? 'loading'
              : panelState === 'error' && lastOperation === 'refresh'
                ? 'error'
                : 'ready'
          "
        >
          <div class="sectionHeader">
            <div>
              <span class="eyebrow">Sources</span>
              <h3>{{ sourceCountPreview }} selected</h3>
            </div>
            <span class="muted">Saved-run catalog · {{ savedRunsMeta || historyStatusLine }}</span>
          </div>

          <div class="sourceGroup">
            <label
              class="checkRow"
              :class="{ disabled: operationBusy || !currentRunSource }"
            >
              <input
                v-model="includeCurrentRun"
                type="checkbox"
                :disabled="operationBusy || !currentRunSource"
              />
              <span>
                <strong>Current run</strong>
                <small>{{
                  currentRunSource?.detail || "No active run artifact"
                }}</small>
              </span>
            </label>
            <label
              class="checkRow"
              :class="{
                disabled: operationBusy || !currentValidationSource,
              }"
            >
              <input
                v-model="includeCurrentValidation"
                type="checkbox"
                :disabled="operationBusy || !currentValidationSource"
              />
              <span>
                <strong>Current validation</strong>
                <small>{{
                  currentValidationSource?.detail ||
                  "No active validation artifact"
                }}</small>
              </span>
            </label>
          </div>

          <div class="sourceToolbar">
            <AppButton
              size="sm"
              variant="subtle"
              :disabled="operationBusy || savedRunRows.length === 0"
              :disabled-reason="
                operationBusy
                  ? 'Another portfolio action is in progress.'
                  : 'No saved-run artifacts are available.'
              "
              data-qa-action="portfolio.saved.selectLatest"
              @click="selectLatestSavedRuns"
            >
              Latest 5
            </AppButton>
            <AppButton
              size="sm"
              variant="subtle"
              :disabled="operationBusy || selectedSavedCount === 0"
              :disabled-reason="
                operationBusy
                  ? 'Another portfolio action is in progress.'
                  : 'No saved-run artifacts are selected.'
              "
              data-qa-action="portfolio.saved.clear"
              @click="clearSavedRuns"
            >
              Clear
            </AppButton>
            <span>
              {{ selectedSavedCount }} catalog
              {{ selectedSavedCount === 1 ? "artifact" : "artifacts" }} selected
            </span>
          </div>

          <div class="savedRunList" data-qa-panel="portfolio.savedSources">
            <div
              v-for="(entry, index) in savedRunRows"
              :key="entry.path"
              class="savedRunRow"
              :class="{ active: selectedSavedSet.has(entry.path) }"
            >
              <input
                :id="`portfolio-saved-run-${index}`"
                type="checkbox"
                :checked="selectedSavedSet.has(entry.path)"
                :disabled="operationBusy"
                @change="
                  toggleSavedPath(
                    entry.path,
                    ($event.target as HTMLInputElement).checked,
                  )
                "
              />
              <div class="savedRunContent">
                <label
                  class="savedRunSelectionLabel"
                  :for="`portfolio-saved-run-${index}`"
                >
                  <strong>{{ savedRunLabel(entry) }}</strong>
                  <small>{{ savedRunDetail(entry) }}</small>
                </label>
                <div class="savedRunEvidence">
                  <CompactEvidenceValue
                    label="saved run path"
                    :value="entry.path"
                    kind="path"
                  >
                    <template #actions>
                      <AppButton
                        size="sm"
                        variant="subtle"
                        class="compactEvidenceAction"
                        data-qa-action="portfolio.saved.reveal"
                        :disabled="operationBusy"
                        :disabled-reason="
                          operationBusy
                            ? 'Another portfolio action is in progress.'
                            : ''
                        "
                        :aria-label="`Reveal saved run ${savedRunLabel(entry)}`"
                        @click.stop="historyStore.revealSavedRun(entry.path)"
                      >
                        Reveal
                      </AppButton>
                    </template>
                  </CompactEvidenceValue>
                  <CompactEvidenceValue
                    v-if="entry.configFingerprint"
                    label="config fingerprint"
                    :value="entry.configFingerprint"
                    kind="hash"
                  />
                </div>
              </div>
            </div>
            <AppEmptyState
              v-if="!savedRunRows.length"
              title="No saved run artifacts"
              body="The saved-run catalog is empty or has not been loaded yet."
              hint="Use Refresh to check the local artifact catalog."
            />
          </div>
        </section>

        <section
          class="resultPane"
          aria-label="Portfolio feasibility result"
          :aria-busy="analyzing ? 'true' : 'false'"
          :data-state="
            analyzing
              ? 'loading'
              : panelState === 'error' && lastOperation === 'analyze'
                ? 'error'
                : result
                  ? 'success'
                  : 'idle'
          "
        >
          <div class="optionsBar">
            <label>
              <span>Max candidates</span>
              <input
                v-model.number="maxCandidates"
                type="number"
                min="1"
                max="250"
                step="1"
                :disabled="operationBusy"
              />
            </label>
            <label class="toggle">
              <input
                v-model="requireApprovedValidation"
                type="checkbox"
                :disabled="operationBusy"
              />
              <span>Approved validation</span>
            </label>
            <label class="toggle">
              <input
                v-model="requireReplayMatrix"
                type="checkbox"
                :disabled="operationBusy"
              />
              <span>Replay matrix</span>
            </label>
            <label class="toggle">
              <input
                v-model="requireSourceProvenance"
                type="checkbox"
                :disabled="operationBusy"
              />
              <span>Source provenance</span>
            </label>
          </div>

          <div
            class="statusStrip"
            :class="statusTone"
            :data-state="panelState"
            :role="statusRole"
            :aria-live="statusLive"
            aria-atomic="true"
            :aria-busy="operationBusy ? 'true' : 'false'"
          >
            <component :is="statusIcon" aria-hidden="true" />
            <div>
              <strong>{{ statusTitle }}</strong>
              <span>{{ errorLine || statusLine }}</span>
            </div>
          </div>

          <AppEmptyState
            v-if="
              !result &&
              !analyzing &&
              !(panelState === 'error' && lastOperation === 'analyze')
            "
            title="No feasibility analysis yet"
            body="Select a current run, validation, or saved artifact source, then run Analyze."
            hint="This workspace evaluates admission evidence only; it does not place trades or allocate capital."
          />

          <AppEmptyState
            v-else-if="
              !result &&
              panelState === 'error' &&
              lastOperation === 'analyze'
            "
            title="No feasibility result available"
            body="Review the error status above, adjust the source selection if needed, and retry."
          />

          <div v-if="result" class="metricGrid">
            <article>
              <span>Sources</span>
              <strong>{{
                result?.sourceArtifactsSeen ?? analyzedSources.length
              }}</strong>
            </article>
            <article>
              <span>Eligible</span>
              <strong>{{ result?.eligibleCandidates.length ?? 0 }}</strong>
            </article>
            <article>
              <span>Rejected</span>
              <strong>{{ result?.rejectedCandidates.length ?? 0 }}</strong>
            </article>
            <article>
              <span>Missing</span>
              <strong>{{ result?.missingEvidence.length ?? 0 }}</strong>
            </article>
          </div>

          <section v-if="result" class="resultSection">
            <div class="nextStep">
              <span class="eyebrow">Next step</span>
              <p>
                {{ result.nextRequiredStep || "No follow-up step reported." }}
              </p>
            </div>
          </section>

          <section v-if="result" class="resultSection">
            <div class="exportToolbar">
              <AppButton
                size="sm"
                variant="subtle"
                :icon="Download"
                :disabled="!canExport"
                disabled-reason="Run a successful feasibility analysis before exporting."
                data-qa-action="portfolio.exportJson"
                @click="exportFeasibilityJson"
              >
                JSON
              </AppButton>
              <AppButton
                size="sm"
                variant="subtle"
                :icon="Download"
                :disabled="!canExport"
                disabled-reason="Run a successful feasibility analysis before exporting."
                data-qa-action="portfolio.exportCsv"
                @click="exportFeasibilityCsv"
              >
                CSV
              </AppButton>
            </div>
          </section>

          <section
            v-if="result"
            class="resultSection"
            data-qa-panel="portfolio.constructionReadiness"
          >
            <div class="readinessHeader">
              <div>
                <span class="eyebrow">Construction research</span>
                <h3>{{ constructionReadiness.label }}</h3>
              </div>
              <span class="muted">diagnostic only</span>
            </div>
            <div class="readinessGrid">
              <article
                v-for="gate in constructionReadiness.gates"
                :key="gate.key"
                class="readinessGate"
                :class="gate.tone"
                :title="gate.detail"
              >
                <span>{{ gate.label }}</span>
                <strong>{{ gate.status }}</strong>
              </article>
            </div>
          </section>

          <section v-if="result?.warnings.length" class="resultSection">
            <h3>Warnings</h3>
            <ul class="messageList">
              <li v-for="warning in result.warnings" :key="warning">
                {{ warning }}
              </li>
            </ul>
          </section>

          <section
            v-if="result?.eligibleCandidates.length"
            class="resultSection"
          >
            <h3>Eligible Candidates</h3>
            <div class="tableShell">
              <table>
                <thead>
                  <tr>
                    <th>Candidate</th>
                    <th>Role</th>
                    <th>Symbol</th>
                    <th>Sharpe</th>
                    <th>PSR</th>
                    <th>Replay</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="candidate in result.eligibleCandidates"
                    :key="candidate.candidateId"
                  >
                    <td class="mono">{{ candidate.candidateId }}</td>
                    <td>{{ candidate.sourceRole }}</td>
                    <td>{{ candidate.symbolRoot || "-" }}</td>
                    <td>
                      {{
                        formatPortfolioNumber(
                          candidate.summaryMetrics.sharpe,
                          2,
                        )
                      }}
                    </td>
                    <td>
                      {{ formatPortfolioNumber(candidate.admission.psr, 2) }}
                    </td>
                    <td>
                      {{ formatPortfolioReplayStatus(candidate.replayStatus) }}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          <section
            v-if="result?.rejectedCandidates.length"
            class="resultSection"
          >
            <h3>Rejected Candidates</h3>
            <ul class="messageList">
              <li
                v-for="candidate in result.rejectedCandidates"
                :key="`${candidate.sourceArtifactIndex}:${candidate.candidateId}`"
              >
                <span class="mono">{{
                  candidate.candidateId || "candidate"
                }}</span>
                <span>{{ candidate.reason }}</span>
              </li>
            </ul>
          </section>

          <section v-if="result?.missingEvidence.length" class="resultSection">
            <h3>Missing Evidence</h3>
            <ul class="messageList">
              <li
                v-for="(missing, index) in result.missingEvidence"
                :key="`${missing.evidenceKind}:${index}`"
              >
                <span class="mono">{{ missing.evidenceKind }}</span>
                <span>{{ missing.message }}</span>
              </li>
            </ul>
          </section>
        </section>
      </div>
    </section>
  </Teleport>
</template>

<style scoped>
.portfolioWorkspace {
  min-width: 0;
  min-height: 0;
  display: grid;
  gap: 0.78rem;
}

.portfolioHeader {
  display: flex;
  justify-content: space-between;
  gap: 0.75rem;
  align-items: flex-start;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 8px;
  padding: 0.78rem 0.86rem;
  background: color-mix(in oklab, var(--surface) 94%, var(--panel));
}

.titleBlock {
  min-width: 0;
  display: grid;
  gap: 0.18rem;
}

.titleBlock h2,
.sectionHeader h3,
.resultSection h3 {
  margin: 0;
}

.titleBlock h2 {
  display: inline-flex;
  align-items: center;
  gap: 0.42rem;
  font-size: 1.05rem;
}

.titleBlock h2 svg {
  width: 1.05rem;
  height: 1.05rem;
}

.titleBlock p,
.nextStep p {
  margin: 0;
  color: var(--muted-strong);
  font-size: 0.82rem;
  line-height: 1.35;
}

.eyebrow {
  font-size: 0.66rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted);
}

.headerActions,
.sourceToolbar,
.exportToolbar,
.readinessHeader {
  display: flex;
  align-items: center;
  gap: 0.46rem;
  flex-wrap: wrap;
}

.exportToolbar {
  justify-content: flex-end;
}

.readinessHeader {
  justify-content: space-between;
  align-items: flex-start;
}

.portfolioGrid {
  min-width: 0;
  display: grid;
  grid-template-columns: minmax(280px, 0.34fr) minmax(0, 1fr);
  gap: 0.72rem;
}

.sourcePane,
.resultPane {
  min-width: 0;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--panel) 98%, var(--surface));
  padding: 0.7rem;
  display: grid;
  align-content: start;
  gap: 0.68rem;
}

.sectionHeader {
  display: flex;
  justify-content: space-between;
  gap: 0.75rem;
  align-items: flex-start;
}

.muted,
.sourceToolbar span,
.checkRow small,
.savedRunSelectionLabel small {
  color: var(--muted);
  font-size: 0.76rem;
}

.sourceGroup {
  display: grid;
  gap: 0.42rem;
}

.checkRow,
.savedRunRow {
  min-width: 0;
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 0.5rem;
  align-items: start;
  border: 1px solid color-mix(in oklab, var(--border) 86%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface) 94%, var(--panel));
  padding: 0.5rem 0.56rem;
}

.checkRow.disabled {
  opacity: 0.58;
}

.checkRow span,
.savedRunSelectionLabel {
  min-width: 0;
  display: grid;
  gap: 0.12rem;
}

.checkRow strong,
.savedRunSelectionLabel strong {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.savedRunContent,
.savedRunEvidence {
  min-width: 0;
  display: grid;
}

.savedRunContent {
  gap: 0.42rem;
}

.savedRunSelectionLabel {
  cursor: pointer;
}

.savedRunEvidence {
  gap: 0.36rem;
}

.savedRunList {
  max-height: min(54vh, 520px);
  overflow: auto;
  display: grid;
  gap: 0.36rem;
  padding-right: 0.12rem;
}

.savedRunRow.active {
  border-color: color-mix(in oklab, var(--panel-focus-edge) 72%, var(--accent));
  background: color-mix(
    in oklab,
    var(--selection-fill) 78%,
    var(--surface-working)
  );
}

.optionsBar {
  display: flex;
  flex-wrap: wrap;
  gap: 0.55rem;
  align-items: end;
}

.optionsBar label {
  display: grid;
  gap: 0.18rem;
  font-size: 0.74rem;
  color: var(--muted-strong);
}

.optionsBar input[type="number"] {
  width: 7rem;
}

.optionsBar .toggle {
  min-height: 34px;
  grid-auto-flow: column;
  align-items: center;
  gap: 0.36rem;
}

.statusStrip {
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 0.6rem 0.66rem;
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 0.5rem;
  align-items: center;
}

.statusStrip svg {
  width: 1.05rem;
  height: 1.05rem;
}

.statusStrip div {
  min-width: 0;
  display: grid;
  gap: 0.12rem;
}

.statusStrip span {
  color: var(--muted-strong);
  font-size: 0.8rem;
}

.statusStrip.neutral {
  background: color-mix(in oklab, var(--surface) 96%, var(--panel));
}

.statusStrip.loading {
  border-color: color-mix(in oklab, var(--accent) 48%, var(--border));
  background: color-mix(in oklab, var(--surface-working) 64%, var(--surface));
}

.statusStrip.good {
  border-color: color-mix(in oklab, var(--ok) 58%, var(--border));
  background: color-mix(in oklab, var(--ok-soft) 62%, var(--surface));
}

.statusStrip.warn {
  border-color: color-mix(in oklab, var(--warning) 58%, var(--border));
  background: color-mix(in oklab, var(--warning-soft) 62%, var(--surface));
}

.statusStrip.bad {
  border-color: color-mix(in oklab, var(--danger) 58%, var(--border));
  background: color-mix(in oklab, var(--danger-soft) 62%, var(--surface));
}

.metricGrid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.46rem;
}

.metricGrid article {
  border: 1px solid color-mix(in oklab, var(--border) 86%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface) 96%, var(--panel));
  padding: 0.54rem 0.58rem;
  display: grid;
  gap: 0.1rem;
}

.metricGrid span {
  color: var(--muted);
  font-size: 0.68rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.metricGrid strong {
  font-size: 1rem;
}

.readinessGrid {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 0.42rem;
}

.readinessGate {
  border: 1px solid color-mix(in oklab, var(--border) 86%, transparent);
  border-radius: 8px;
  padding: 0.48rem 0.52rem;
  background: color-mix(in oklab, var(--surface) 96%, var(--panel));
  display: grid;
  gap: 0.12rem;
  min-width: 0;
}

.readinessGate span {
  color: var(--muted);
  font-size: 0.66rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.readinessGate strong {
  min-width: 0;
  overflow-wrap: anywhere;
  font-size: 0.82rem;
}

.readinessGate.pass {
  border-color: color-mix(in oklab, var(--ok) 50%, var(--border));
}

.readinessGate.warn {
  border-color: color-mix(in oklab, var(--warning) 52%, var(--border));
}

.readinessGate.blocked {
  border-color: color-mix(in oklab, var(--danger) 45%, var(--border));
}

.resultSection {
  min-width: 0;
  display: grid;
  gap: 0.4rem;
}

.nextStep {
  border-left: 3px solid var(--selection-rail);
  padding-left: 0.58rem;
}

.messageList {
  margin: 0;
  padding: 0;
  display: grid;
  gap: 0.34rem;
  list-style: none;
}

.messageList li {
  min-width: 0;
  border: 1px solid color-mix(in oklab, var(--border) 86%, transparent);
  border-radius: 8px;
  padding: 0.46rem 0.54rem;
  background: color-mix(in oklab, var(--surface) 94%, var(--panel));
  display: grid;
  gap: 0.12rem;
  font-size: 0.8rem;
}

.tableShell {
  min-width: 0;
  overflow: auto;
  border: 1px solid color-mix(in oklab, var(--border) 86%, transparent);
  border-radius: 8px;
}

table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.78rem;
}

th,
td {
  padding: 0.42rem 0.48rem;
  border-bottom: 1px solid color-mix(in oklab, var(--border) 78%, transparent);
  text-align: left;
  white-space: nowrap;
}

th {
  color: var(--muted);
  font-size: 0.68rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  background: color-mix(in oklab, var(--surface-sticky) 94%, var(--panel));
}

tr:last-child td {
  border-bottom: 0;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 1080px) {
  .portfolioGrid {
    grid-template-columns: 1fr;
  }

  .metricGrid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .readinessGrid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 720px) {
  .portfolioHeader,
  .sectionHeader {
    display: grid;
  }

  .metricGrid {
    grid-template-columns: 1fr;
  }

  .readinessGrid {
    grid-template-columns: 1fr;
  }
}
</style>
