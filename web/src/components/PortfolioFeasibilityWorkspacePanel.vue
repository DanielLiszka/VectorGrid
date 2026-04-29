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
} from "lucide-vue-next";
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

const workflowArtifactsStore = useWorkflowArtifactsStore();
const historyStore = useHistoryStore();
const { currentRun, currentValidation } = storeToRefs(workflowArtifactsStore);
const {
  savedRunsRaw,
  savedRunsVisible,
  savedRunsMeta,
  statusLine: historyStatusLine,
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
const busy = ref(false);
const statusLine = ref("Portfolio feasibility is ready.");
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
const canAnalyze = computed(() => sourceCountPreview.value > 0 && !busy.value);
const statusTone = computed(() => {
  const status = result.value?.status || "";
  if (status === "replay_parity_passed") return "good";
  if (
    status === "artifact_provenance_blocked" ||
    status === "unsupported_candidate_family"
  )
    return "bad";
  return "warn";
});
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
  () => !!result.value && analyzedSources.value.length > 0,
);

watch(savedRunRows, (rows) => {
  const allowed = new Set(rows.map((row) => row.path).filter(Boolean));
  selectedSavedPaths.value = selectedSavedPaths.value.filter((path) =>
    allowed.has(path),
  );
});

onMounted(() => {
  if (!savedRunsRaw.value.length) historyStore.refreshSavedRuns();
});

function savedRunLabel(entry: HistorySavedRunSummary): string {
  return (
    [entry.ts, entry.strategyKind, entry.timeframe]
      .filter(Boolean)
      .join(" | ") || entry.path
  );
}

function savedRunDetail(entry: HistorySavedRunSummary): string {
  const flags = [
    entry.hasValidation ? "validation" : "no validation",
    entry.configFingerprint ? `cfg ${entry.configFingerprint}` : "",
  ].filter(Boolean);
  return flags.join(" | ");
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
  historyStore.refreshSavedRuns();
  statusLine.value = "Refreshing saved run sources.";
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
  if (busy.value) return;
  errorLine.value = "";
  result.value = null;
  busy.value = true;
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
    statusLine.value = `Analyzed ${sources.length} source${sources.length === 1 ? "" : "s"}.`;
  } catch (error) {
    const message = String(
      error instanceof Error ? error.message : error || "",
    ).trim();
    errorLine.value = message || "Portfolio feasibility analysis failed.";
    statusLine.value = "Portfolio feasibility analysis failed.";
  } finally {
    busy.value = false;
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
  if (!result.value) return;
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
  if (!result.value) return;
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
    <section class="portfolioWorkspace" data-qa-panel="portfolio.feasibility">
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
          <button
            type="button"
            class="ghostButton"
            data-qa-action="portfolio.refreshSources"
            @click="refreshSavedRuns"
          >
            <RefreshCw aria-hidden="true" />
            <span>Refresh</span>
          </button>
          <button
            type="button"
            class="primaryButton"
            :disabled="!canAnalyze"
            data-qa-action="portfolio.runFeasibility"
            @click="runFeasibility"
          >
            <Play aria-hidden="true" />
            <span>{{ busy ? "Analyzing" : "Analyze" }}</span>
          </button>
        </div>
      </header>

      <div class="portfolioGrid">
        <section class="sourcePane" aria-label="Portfolio feasibility sources">
          <div class="sectionHeader">
            <div>
              <span class="eyebrow">Sources</span>
              <h3>{{ sourceCountPreview }} selected</h3>
            </div>
            <span class="muted">{{ savedRunsMeta || historyStatusLine }}</span>
          </div>

          <div class="sourceGroup">
            <label class="checkRow" :class="{ disabled: !currentRunSource }">
              <input
                v-model="includeCurrentRun"
                type="checkbox"
                :disabled="!currentRunSource"
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
              :class="{ disabled: !currentValidationSource }"
            >
              <input
                v-model="includeCurrentValidation"
                type="checkbox"
                :disabled="!currentValidationSource"
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
            <button
              type="button"
              class="ghostButton compact"
              @click="selectLatestSavedRuns"
            >
              Latest 5
            </button>
            <button
              type="button"
              class="ghostButton compact"
              :disabled="selectedSavedCount === 0"
              @click="clearSavedRuns"
            >
              Clear
            </button>
            <span>{{ selectedSavedCount }} saved</span>
          </div>

          <div class="savedRunList" data-qa-panel="portfolio.savedSources">
            <label
              v-for="entry in savedRunRows"
              :key="entry.path"
              class="savedRunRow"
              :class="{ active: selectedSavedSet.has(entry.path) }"
            >
              <input
                type="checkbox"
                :checked="selectedSavedSet.has(entry.path)"
                @change="
                  toggleSavedPath(
                    entry.path,
                    ($event.target as HTMLInputElement).checked,
                  )
                "
              />
              <span>
                <strong>{{ savedRunLabel(entry) }}</strong>
                <small>{{ savedRunDetail(entry) || entry.path }}</small>
              </span>
            </label>
            <div v-if="!savedRunRows.length" class="emptyState">
              Saved run artifacts are not loaded.
            </div>
          </div>
        </section>

        <section class="resultPane" aria-label="Portfolio feasibility result">
          <div class="optionsBar">
            <label>
              <span>Max candidates</span>
              <input
                v-model.number="maxCandidates"
                type="number"
                min="1"
                max="250"
                step="1"
              />
            </label>
            <label class="toggle">
              <input v-model="requireApprovedValidation" type="checkbox" />
              <span>Approved validation</span>
            </label>
            <label class="toggle">
              <input v-model="requireReplayMatrix" type="checkbox" />
              <span>Replay matrix</span>
            </label>
            <label class="toggle">
              <input v-model="requireSourceProvenance" type="checkbox" />
              <span>Source provenance</span>
            </label>
          </div>

          <div class="statusStrip" :class="statusTone">
            <component
              :is="statusTone === 'good' ? CheckCircle2 : AlertTriangle"
              aria-hidden="true"
            />
            <div>
              <strong>{{
                result
                  ? formatPortfolioEvidenceStatus(result.status)
                  : "Not analyzed"
              }}</strong>
              <span>{{ errorLine || statusLine }}</span>
            </div>
          </div>

          <div class="metricGrid">
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
              <button
                type="button"
                class="ghostButton compact"
                :disabled="!canExport"
                data-qa-action="portfolio.exportJson"
                @click="exportFeasibilityJson"
              >
                <Download aria-hidden="true" />
                JSON
              </button>
              <button
                type="button"
                class="ghostButton compact"
                :disabled="!canExport"
                data-qa-action="portfolio.exportCsv"
                @click="exportFeasibilityCsv"
              >
                <Download aria-hidden="true" />
                CSV
              </button>
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

.primaryButton,
.ghostButton {
  min-height: 34px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.36rem;
  border-radius: 8px;
  padding: 0.34rem 0.62rem;
}

.primaryButton svg,
.ghostButton svg {
  width: 0.96rem;
  height: 0.96rem;
}

.primaryButton {
  color: var(--on-accent);
  background: var(--accent);
  border-color: color-mix(in oklab, var(--accent) 72%, var(--border));
}

.ghostButton.compact {
  min-height: 30px;
  padding: 0.22rem 0.5rem;
  font-size: 0.76rem;
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
.savedRunRow small {
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
.savedRunRow span {
  min-width: 0;
  display: grid;
  gap: 0.12rem;
}

.checkRow strong,
.savedRunRow strong {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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

.emptyState {
  border: 1px dashed var(--border);
  border-radius: 8px;
  padding: 0.72rem;
  color: var(--muted);
  background: color-mix(in oklab, var(--surface) 95%, var(--panel));
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
