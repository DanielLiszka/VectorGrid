<script setup lang="ts">
import { computed, ref, watch } from "vue";
import AppEmptyState from "../ui/AppEmptyState.vue";
import {
  VCET_CAMPAIGN_EVIDENCE_METRICS,
  VCET_CAMPAIGN_EVIDENCE_PARAMETER_AXES,
  buildVcetCampaignEvidenceMetricView,
  buildVcetCampaignEvidenceScatterView,
  formatVcetCampaignEvidenceAxisValue,
  formatVcetCampaignEvidenceCandidateMetric,
  formatVcetCampaignEvidenceParameter,
  formatVcetCampaignEvidenceSearchSequence,
  formatVcetCampaignEvidenceValue,
  resolveVcetCampaignEvidencePlateau,
  sampleVcetCampaignEvidenceScatterPoints,
  type VcetCampaignEvidenceCell,
  type VcetCampaignEvidenceMetricKey,
  type VcetCampaignEvidenceModel,
  type VcetCampaignEvidenceParameterAxisKey,
  type VcetCampaignEvidenceScatterPoint,
} from "../../visualizations/vcetCampaignEvidenceModel";

const props = defineProps<{
  model: VcetCampaignEvidenceModel;
}>();

const selectedMetric = ref<VcetCampaignEvidenceMetricKey>(
  "profitable_share_requested",
);
const activeCandidateId = ref("");
const scatterXAxis = ref<VcetCampaignEvidenceParameterAxisKey>("vidya_alpha");
const scatterYAxis = ref<VcetCampaignEvidenceParameterAxisKey>("ce_mult");
const tablePage = ref(1);
const TABLE_PAGE_SIZE = 100;

watch(
  () => props.model.signature,
  () => {
    activeCandidateId.value = props.model.selectedCandidateId;
    tablePage.value = 1;
  },
  { immediate: true },
);

const metricView = computed(() =>
  buildVcetCampaignEvidenceMetricView(props.model, selectedMetric.value),
);
const cellByCoordinate = computed(
  () =>
    new Map(
      metricView.value.cells.map((cell) => [
        `${cell.cell.fastLen}:${cell.cell.slowLen}`,
        cell,
      ]),
    ),
);
const displayedSlowValues = computed(() =>
  props.model.slowValues.slice().sort((left, right) => right - left),
);
const matrixRows = computed(() =>
  displayedSlowValues.value.map((slowLen) => ({
    slowLen,
    cells: props.model.fastValues.map((fastLen) => ({
      fastLen,
      evidence:
        cellByCoordinate.value.get(`${fastLen}:${slowLen}`) ?? null,
    })),
  })),
);
const activeMetricCell = computed(() => {
  const requested = activeCandidateId.value;
  return (
    metricView.value.cells.find(
      (cell) => cell.cell.candidate.candidateId === requested,
    ) ??
    metricView.value.cells.find((cell) => cell.cell.candidate.selected) ??
    metricView.value.cells[0] ??
    null
  );
});
const plateau = computed(() =>
  resolveVcetCampaignEvidencePlateau(
    props.model,
    selectedMetric.value,
    activeMetricCell.value?.cell.candidate.candidateId ?? "",
  ),
);
const scatterView = computed(() =>
  buildVcetCampaignEvidenceScatterView(
    props.model,
    scatterXAxis.value,
    scatterYAxis.value,
    selectedMetric.value,
  ),
);
const scatterSample = computed(() =>
  sampleVcetCampaignEvidenceScatterPoints(scatterView.value.points),
);
const metricDisplayByCandidate = computed(() => {
  const displays = new Map<
    string,
    Map<VcetCampaignEvidenceMetricKey, string>
  >();
  for (const candidate of props.model.candidates) {
    const candidateDisplays = new Map<
      VcetCampaignEvidenceMetricKey,
      string
    >();
    for (const metric of VCET_CAMPAIGN_EVIDENCE_METRICS) {
      candidateDisplays.set(
        metric.key,
        formatVcetCampaignEvidenceCandidateMetric(candidate, metric.key),
      );
    }
    displays.set(candidate.candidateId, candidateDisplays);
  }
  return displays;
});
const parameterDisplayByCandidate = computed(() => {
  const displays = new Map<
    string,
    Map<VcetCampaignEvidenceParameterAxisKey, string>
  >();
  for (const candidate of props.model.candidates) {
    const candidateDisplays = new Map<
      VcetCampaignEvidenceParameterAxisKey,
      string
    >();
    for (const axis of VCET_CAMPAIGN_EVIDENCE_PARAMETER_AXES) {
      candidateDisplays.set(
        axis.key,
        formatVcetCampaignEvidenceParameter(candidate.effective, axis.key),
      );
    }
    displays.set(candidate.candidateId, candidateDisplays);
  }
  return displays;
});
const tablePageCount = computed(() =>
  Math.max(1, Math.ceil(props.model.candidates.length / TABLE_PAGE_SIZE)),
);
watch(tablePageCount, (nextCount) => {
  tablePage.value = Math.max(1, Math.min(tablePage.value, nextCount));
});
const tableCandidates = computed(() => {
  const page = Math.max(1, Math.min(tablePage.value, tablePageCount.value));
  const start = (page - 1) * TABLE_PAGE_SIZE;
  return props.model.candidates.slice(start, start + TABLE_PAGE_SIZE);
});
const tableRangeSummary = computed(() => {
  const total = props.model.candidates.length;
  if (total === 0) return "No candidates";
  const start = (tablePage.value - 1) * TABLE_PAGE_SIZE + 1;
  const end = Math.min(total, start + TABLE_PAGE_SIZE - 1);
  return `Showing ${start.toLocaleString()}–${end.toLocaleString()} of ${total.toLocaleString()} candidates`;
});
const metricRange = computed(() => {
  const view = metricView.value;
  if (view.finiteMinimum == null || view.finiteMaximum == null) {
    return "No finite values";
  }
  return `${formatVcetCampaignEvidenceValue(view.finiteMinimum, view.metric)} → ${formatVcetCampaignEvidenceValue(view.finiteMaximum, view.metric)}`;
});
const summaryText = computed(
  () => {
    const candidateCount = props.model.candidates.length;
    const admittedCount = props.model.admittedCandidateCount;
    const design =
      props.model.designMode === "halton"
        ? `Sealed ${formatVcetCampaignEvidenceSearchSequence(props.model.searchSequence)} design`
        : "Fast×Slow period grid";
    return `${props.model.cohortLabel}. ${design} with ${candidateCount} candidate${candidateCount === 1 ? "" : "s"} across ${props.model.requestedDatasets} requested datasets. ${admittedCount} candidate${admittedCount === 1 ? "" : "s"} admitted.`;
  },
);
const searchGenerationSummary = computed(() => {
  const stats = props.model.searchGenerationStats;
  if (props.model.designMode !== "halton" || !stats) return "";
  if (props.model.searchSequence === "exhaustive_lexicographic") {
    return `${stats.exhaustiveCandidates.toLocaleString()} exhaustive candidates generated`;
  }
  return [
    `${stats.haltonAttempts.toLocaleString()} sequence attempts`,
    `${stats.duplicateCandidatesDiscarded.toLocaleString()} duplicates discarded`,
    `${stats.invalidPeriodPairsDiscarded.toLocaleString()} invalid period pairs discarded`,
  ].join(" · ");
});
const admissionLegend = computed(() => {
  if (props.model.admissionSource === "artifact") {
    return "A = admitted by backend artifact";
  }
  if (props.model.admissionSource === "derived") {
    return "A = admission reconstructed from retained thresholds";
  }
  return "Per-candidate admission identity unavailable";
});
const manifestLabel = computed(() => {
  const hash = String(props.model.candidateManifestHash || "").trim();
  if (!hash) return "Manifest unavailable";
  return hash.length > 22
    ? `Manifest ${hash.slice(0, 12)}…${hash.slice(-6)}`
    : `Manifest ${hash}`;
});

function scatterPointStyle(point: VcetCampaignEvidenceScatterPoint): {
  left: string;
  bottom: string;
} {
  return {
    left: `${4 + point.xRatio * 92}%`,
    bottom: `${5 + point.yRatio * 90}%`,
  };
}

function scatterAxisRange(
  minimum: number | null,
  maximum: number | null,
  axis: VcetCampaignEvidenceParameterAxisKey,
): string {
  return `${formatVcetCampaignEvidenceAxisValue(minimum, axis)} → ${formatVcetCampaignEvidenceAxisValue(maximum, axis)}`;
}

function candidateParameter(
  candidateId: string,
  key: VcetCampaignEvidenceParameterAxisKey,
): string {
  return (
    parameterDisplayByCandidate.value.get(candidateId)?.get(key) ??
    "Unavailable"
  );
}

function moveTablePage(delta: number): void {
  tablePage.value = Math.max(
    1,
    Math.min(tablePageCount.value, tablePage.value + delta),
  );
}

function selectCell(cell: VcetCampaignEvidenceCell): void {
  activeCandidateId.value = cell.candidate.candidateId;
}

function candidateMetric(
  candidateId: string,
  metricKey: VcetCampaignEvidenceMetricKey,
): string {
  return (
    metricDisplayByCandidate.value.get(candidateId)?.get(metricKey) ??
    "Unavailable"
  );
}
</script>

<template>
  <section
    class="vcetEvidence"
    aria-labelledby="vcetCampaignEvidenceTitle"
    :data-evidence-state="model.state"
    :data-schema-version="model.schemaVersion"
  >
    <div class="vcetEvidenceHeader">
      <div>
        <div class="panelEyebrow">Cross-universe evidence</div>
        <h3 id="vcetCampaignEvidenceTitle">VCET parameter landscape</h3>
        <p id="vcetCampaignEvidenceSummary" class="vcetEvidenceLead">
          {{ summaryText }}
        </p>
      </div>
      <label class="vcetMetricControl" for="vcetCampaignEvidenceMetric">
        <span>Heatmap metric</span>
        <select
          id="vcetCampaignEvidenceMetric"
          v-model="selectedMetric"
          data-qa="vcet-evidence-metric"
        >
          <option
            v-for="metric in VCET_CAMPAIGN_EVIDENCE_METRICS"
            :key="metric.key"
            :value="metric.key"
          >
            {{ metric.label }}
          </option>
        </select>
      </label>
    </div>

    <div class="vcetEvidenceFacts" aria-label="VCET evidence scope">
      <span>{{ model.cohortLabel }}</span>
      <span>Schema v{{ model.schemaVersion || "?" }}</span>
      <span
        >Coverage {{ model.evaluatedDatasets }}/{{ model.requestedDatasets }}</span
      >
      <span>Admitted {{ model.admittedCandidateCount }}/{{ model.candidates.length }}</span>
      <span class="mono" :title="model.candidateManifestHash || undefined">
        {{ manifestLabel }}
      </span>
      <span v-if="model.designMode === 'halton'">
        {{ formatVcetCampaignEvidenceSearchSequence(model.searchSequence) }}
      </span>
      <span v-if="model.designMode === 'halton'">
        Budget {{ (model.searchEffectiveBudget ?? model.candidates.length).toLocaleString() }}/{{
          (model.searchRequestedBudget ?? model.candidates.length).toLocaleString()
        }} · space {{ model.validSearchSpaceSize || "?" }}
      </span>
      <span>{{ metricRange }}</span>
    </div>

    <div class="vcetMetricExplanation">
      {{ metricView.metric.description }}
    </div>

    <div
      v-if="model.warnings.length > 0"
      class="vcetEvidenceWarning"
      role="note"
      aria-label="VCET evidence limitations"
    >
      <strong>{{ model.state === "empty" ? "Evidence unavailable" : "Evidence limitations" }}</strong>
      <ul>
        <li v-for="warning in model.warnings" :key="warning">
          {{ warning }}
        </li>
      </ul>
    </div>

    <AppEmptyState
      v-if="model.state === 'empty'"
      title="No selection-safe VCET landscape"
      body="The campaign artifact does not contain candidate rows that can be shown without mixing selection and holdout evidence."
      :hint="model.visualGridUnavailableReason"
    />

    <template v-else>
      <div class="vcetEvidenceLegend" aria-label="Heatmap marker legend">
        <span><b class="marker markerSelected">S</b> Backend selected</span>
        <span><b class="marker markerAdmitted">A</b> {{ admissionLegend.replace(/^A = /u, "") }}</span>
        <span v-if="model.designMode === 'period_grid'"
          >Slow increases upward · Fast increases rightward</span
        >
        <span v-else-if="searchGenerationSummary">{{ searchGenerationSummary }}</span>
      </div>

      <div
        v-if="model.visualGridAvailable"
        class="vcetMatrixScroll"
        tabindex="0"
        aria-label="Scrollable VCET Fast by Slow heatmap"
      >
        <table
          class="vcetMatrix"
          aria-label="VCET Fast by Slow candidate heatmap"
          aria-describedby="vcetCampaignEvidenceSummary vcetCampaignEvidencePlateau"
        >
          <thead>
            <tr>
              <th scope="col" class="axisCorner">Slow \ Fast</th>
              <th
                v-for="fastLen in model.fastValues"
                :key="`fast_${fastLen}`"
                scope="col"
              >
                {{ fastLen }}
              </th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="row in matrixRows" :key="`slow_${row.slowLen}`">
              <th scope="row">{{ row.slowLen }}</th>
              <td
                v-for="matrixCell in row.cells"
                :key="`${matrixCell.fastLen}_${row.slowLen}`"
              >
                <button
                  v-if="matrixCell.evidence"
                  type="button"
                  class="vcetMetricCell"
                  :class="[
                    `tone-${matrixCell.evidence.tone}`,
                    {
                      active:
                        matrixCell.evidence.cell.candidate.candidateId ===
                        activeMetricCell?.cell.candidate.candidateId,
                      selected: matrixCell.evidence.cell.candidate.selected,
                      admitted:
                        matrixCell.evidence.cell.candidate.admitted === true,
                    },
                  ]"
                  :data-intensity="matrixCell.evidence.intensity"
                  :aria-label="matrixCell.evidence.ariaLabel"
                  :title="matrixCell.evidence.ariaLabel"
                  @click="selectCell(matrixCell.evidence.cell)"
                >
                  <span class="cellValue">
                    {{ matrixCell.evidence.displayValue }}
                  </span>
                  <span
                    v-if="matrixCell.evidence.cell.candidate.selected"
                    class="marker markerSelected"
                    aria-hidden="true"
                    >S</span
                  >
                  <span
                    v-if="matrixCell.evidence.cell.candidate.admitted === true"
                    class="marker markerAdmitted"
                    aria-hidden="true"
                    >A</span
                  >
                </button>
                <span
                  v-else
                  class="vcetMetricCell unavailable"
                  :aria-label="`Fast ${matrixCell.fastLen}, Slow ${row.slowLen}: candidate unavailable.`"
                  >—</span
                >
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <section
        v-else-if="model.designMode === 'halton'"
        class="vcetScatterPanel"
        aria-labelledby="vcetScatterTitle"
      >
        <div class="vcetScatterHeader">
          <div>
            <strong id="vcetScatterTitle">Effective-control projection</strong>
            <p>
              Every point remains a distinct candidate identity. Change either
              axis to inspect the sealed design without collapsing it into a
              Fast×Slow cell.
            </p>
          </div>
          <div class="vcetScatterAxisControls">
            <label for="vcetScatterXAxis">
              <span>X axis</span>
              <select id="vcetScatterXAxis" v-model="scatterXAxis">
                <option
                  v-for="axis in VCET_CAMPAIGN_EVIDENCE_PARAMETER_AXES"
                  :key="`scatter_x_${axis.key}`"
                  :value="axis.key"
                  :disabled="axis.key === scatterYAxis"
                >
                  {{ axis.label }}
                </option>
              </select>
            </label>
            <label for="vcetScatterYAxis">
              <span>Y axis</span>
              <select id="vcetScatterYAxis" v-model="scatterYAxis">
                <option
                  v-for="axis in VCET_CAMPAIGN_EVIDENCE_PARAMETER_AXES"
                  :key="`scatter_y_${axis.key}`"
                  :value="axis.key"
                  :disabled="axis.key === scatterXAxis"
                >
                  {{ axis.label }}
                </option>
              </select>
            </label>
          </div>
        </div>

        <div
          v-if="scatterSample.points.length > 0"
          class="vcetScatterPlot"
          role="group"
          :aria-label="`${scatterView.yAxis.label} by ${scatterView.xAxis.label} candidate projection`"
        >
          <span class="scatterAxisTitle scatterAxisTitleY">{{ scatterView.yAxis.shortLabel }}</span>
          <span class="scatterAxisTitle scatterAxisTitleX">{{ scatterView.xAxis.shortLabel }}</span>
          <span class="scatterBound scatterBoundYMax">{{
            formatVcetCampaignEvidenceAxisValue(scatterView.yMaximum, scatterYAxis)
          }}</span>
          <span class="scatterBound scatterBoundYMin">{{
            formatVcetCampaignEvidenceAxisValue(scatterView.yMinimum, scatterYAxis)
          }}</span>
          <span class="scatterBound scatterBoundXMin">{{
            formatVcetCampaignEvidenceAxisValue(scatterView.xMinimum, scatterXAxis)
          }}</span>
          <span class="scatterBound scatterBoundXMax">{{
            formatVcetCampaignEvidenceAxisValue(scatterView.xMaximum, scatterXAxis)
          }}</span>
          <button
            v-for="point in scatterSample.points"
            :key="point.candidate.candidateId"
            type="button"
            class="vcetScatterPoint"
            :class="[
              `tone-${point.metricCell.tone}`,
              {
                active: point.candidate.candidateId === activeMetricCell?.cell.candidate.candidateId,
                selected: point.candidate.selected,
                admitted: point.candidate.admitted === true,
              },
            ]"
            :data-intensity="point.metricCell.intensity"
            :style="scatterPointStyle(point)"
            :aria-label="point.ariaLabel"
            :title="point.ariaLabel"
            @click="selectCell(point.metricCell.cell)"
          />
        </div>
        <div v-else class="vcetGridUnavailable" role="note">
          No candidates contain complete effective controls for these axes.
        </div>
        <p class="vcetScatterNote">
          {{ scatterAxisRange(scatterView.xMinimum, scatterView.xMaximum, scatterXAxis) }} on X ·
          {{ scatterAxisRange(scatterView.yMinimum, scatterView.yMaximum, scatterYAxis) }} on Y.
          Showing {{ scatterSample.points.length.toLocaleString() }} of
          {{ scatterSample.totalPointCount.toLocaleString() }} plottable candidates<span
            v-if="scatterView.omittedCandidateCount > 0"
            >; {{ scatterView.omittedCandidateCount.toLocaleString() }} lack complete axis controls</span
          >. Projected points can overlap; the paged table preserves every retained identity.
        </p>
      </section>

      <div v-else class="vcetGridUnavailable" role="note">
        {{ model.visualGridUnavailableReason }}
      </div>

      <div
        v-if="activeMetricCell"
        class="vcetActiveEvidence"
        aria-live="polite"
        aria-atomic="true"
      >
        <div class="vcetActiveHeading">
          <div>
            <span class="panelMetricKey">Focused candidate</span>
            <strong v-if="model.designMode === 'period_grid'" class="mono">
              Fast {{ activeMetricCell.cell.fastLen }} / Slow
              {{ activeMetricCell.cell.slowLen }}
            </strong>
            <strong
              v-else
              class="mono"
              :title="activeMetricCell.cell.candidate.searchCandidateId || activeMetricCell.cell.candidate.candidateId"
            >
              Candidate
              {{
                activeMetricCell.cell.candidate.searchOrdinal != null
                  ? `#${activeMetricCell.cell.candidate.searchOrdinal + 1}`
                  : activeMetricCell.cell.candidate.candidateId
              }}
            </strong>
          </div>
          <div class="activeMarkers">
            <span v-if="activeMetricCell.cell.candidate.selected">Backend selected</span>
            <span v-if="activeMetricCell.cell.candidate.admitted === true">Admitted</span>
            <span v-else-if="activeMetricCell.cell.candidate.admitted === false">Not admitted</span>
            <span v-else>Admission unavailable</span>
          </div>
        </div>
        <div class="vcetActiveMetrics">
          <span
            v-for="metric in VCET_CAMPAIGN_EVIDENCE_METRICS"
            :key="`active_${metric.key}`"
          >
            <b>{{ metric.shortLabel }}</b>
            {{ candidateMetric(activeMetricCell.cell.candidate.candidateId, metric.key) }}
          </span>
        </div>
        <div
          v-if="model.designMode === 'halton'"
          class="vcetActiveControls"
          aria-label="Focused candidate effective controls"
        >
          <span
            v-for="axis in VCET_CAMPAIGN_EVIDENCE_PARAMETER_AXES"
            :key="`active_parameter_${axis.key}`"
            :title="axis.description"
          >
            <b>{{ axis.shortLabel }}</b>
            {{ candidateParameter(activeMetricCell.cell.candidate.candidateId, axis.key) }}
          </span>
        </div>
        <p id="vcetCampaignEvidencePlateau" class="vcetPlateauSummary">
          {{ plateau.summary }}
        </p>
      </div>

      <details class="vcetAccessibleTable">
        <summary>
          Candidate evidence table · {{ model.candidates.length.toLocaleString() }} retained
        </summary>
        <div class="vcetTablePagination" aria-label="Candidate table pagination">
          <span>{{ tableRangeSummary }} · page {{ tablePage }}/{{ tablePageCount }}</span>
          <div>
            <button
              type="button"
              :disabled="tablePage <= 1"
              aria-label="Previous candidate table page"
              @click="moveTablePage(-1)"
            >
              Previous
            </button>
            <button
              type="button"
              :disabled="tablePage >= tablePageCount"
              aria-label="Next candidate table page"
              @click="moveTablePage(1)"
            >
              Next
            </button>
          </div>
        </div>
        <div class="vcetCandidateTableScroll">
          <table :class="{ multidimensionalTable: model.designMode === 'halton' }">
            <caption>
              Selection-safe VCET candidate evidence. This paged table preserves
              every retained candidate and its exact evaluation identity.
            </caption>
            <thead>
              <tr>
                <th v-if="model.designMode === 'halton'" scope="col">Ordinal</th>
                <th scope="col">Candidate</th>
                <th v-if="model.designMode === 'halton'" scope="col">Search identity</th>
                <th scope="col">Fast</th>
                <th scope="col">Slow</th>
                <th v-if="model.designMode === 'halton'" scope="col">Alpha</th>
                <th v-if="model.designMode === 'halton'" scope="col">CE period</th>
                <th v-if="model.designMode === 'halton'" scope="col">CE mult</th>
                <th v-if="model.designMode === 'halton'" scope="col">CE source</th>
                <th v-if="model.designMode === 'halton'" scope="col">Slope</th>
                <th v-if="model.designMode === 'halton'" scope="col">Max bars</th>
                <th scope="col">Profitable</th>
                <th scope="col">Hard gate</th>
                <th scope="col">PnL p10</th>
                <th scope="col">Sharpe p10</th>
                <th scope="col">Missing</th>
                <th scope="col">Evaluated</th>
                <th scope="col">Decision</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="candidate in tableCandidates"
                :key="candidate.candidateId"
                :class="{ selectedRow: candidate.selected }"
              >
                <td v-if="model.designMode === 'halton'">
                  {{ candidate.searchOrdinal != null ? candidate.searchOrdinal + 1 : "—" }}
                </td>
                <th scope="row" class="mono">{{ candidate.candidateId }}</th>
                <td
                  v-if="model.designMode === 'halton'"
                  class="mono"
                  :title="candidate.searchCandidateId || undefined"
                >
                  {{ candidate.searchCandidateId || "Unavailable" }}
                </td>
                <td>{{
                  model.designMode === "halton"
                    ? candidateParameter(candidate.candidateId, "vidya_short_period")
                    : candidate.fastLen
                }}</td>
                <td>{{
                  model.designMode === "halton"
                    ? candidateParameter(candidate.candidateId, "vidya_long_period")
                    : candidate.slowLen
                }}</td>
                <td v-if="model.designMode === 'halton'">{{
                  candidateParameter(candidate.candidateId, "vidya_alpha")
                }}</td>
                <td v-if="model.designMode === 'halton'">{{
                  candidateParameter(candidate.candidateId, "ce_period")
                }}</td>
                <td v-if="model.designMode === 'halton'">{{
                  candidateParameter(candidate.candidateId, "ce_mult")
                }}</td>
                <td v-if="model.designMode === 'halton'">{{
                  candidateParameter(candidate.candidateId, "ce_use_close")
                }}</td>
                <td v-if="model.designMode === 'halton'">{{
                  candidateParameter(candidate.candidateId, "require_vidya_slope")
                }}</td>
                <td v-if="model.designMode === 'halton'">{{
                  candidateParameter(candidate.candidateId, "max_trade_bars")
                }}</td>
                <td>{{ candidateMetric(candidate.candidateId, "profitable_share_requested") }}</td>
                <td>{{ candidateMetric(candidate.candidateId, "hard_gate_pass_share_requested") }}</td>
                <td>{{ candidateMetric(candidate.candidateId, "pnl_p10") }}</td>
                <td>{{ candidateMetric(candidate.candidateId, "sharpe_p10") }}</td>
                <td>{{ candidateMetric(candidate.candidateId, "missing_coverage") }}</td>
                <td>{{ candidate.evaluatedDatasets }}/{{ candidate.requestedDatasets }}</td>
                <td>
                  {{
                    [
                      candidate.selected ? "Selected" : "",
                      candidate.admitted === true
                        ? "Admitted"
                        : candidate.admitted === false
                          ? "Not admitted"
                          : "Admission unavailable",
                    ]
                      .filter(Boolean)
                      .join(" · ")
                  }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </details>
    </template>
  </section>
</template>

<style scoped>
.vcetEvidence {
  margin-top: 0.8rem;
  border: 1px solid color-mix(in oklab, var(--accent) 22%, var(--border));
  border-radius: 14px;
  background:
    radial-gradient(
      circle at 92% 0%,
      color-mix(in oklab, var(--accent-soft) 78%, transparent),
      transparent 36%
    ),
    var(--surface-sunken);
  padding: 0.85rem;
  box-shadow: var(--shadow-inline);
}

.vcetEvidenceHeader {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
}

.vcetEvidenceHeader h3 {
  margin: 0.12rem 0 0;
  font-size: 1rem;
}

.vcetEvidenceLead {
  max-width: 64rem;
  margin: 0.28rem 0 0;
  color: var(--muted);
  font-size: 0.8rem;
  line-height: 1.4;
}

.vcetMetricControl {
  flex: 0 0 min(15rem, 40%);
  display: grid;
  gap: 0.28rem;
  color: var(--muted-strong);
  font-size: 0.72rem;
  font-weight: 700;
}

.vcetMetricControl select {
  width: 100%;
}

.vcetEvidenceFacts,
.vcetEvidenceLegend,
.vcetActiveMetrics,
.vcetActiveControls,
.activeMarkers {
  display: flex;
  flex-wrap: wrap;
  gap: 0.38rem;
}

.vcetEvidenceFacts {
  margin-top: 0.7rem;
}

.vcetEvidenceFacts span,
.vcetEvidenceLegend span,
.vcetActiveMetrics span,
.vcetActiveControls span,
.activeMarkers span {
  border: 1px solid var(--border);
  border-radius: 999px;
  background: color-mix(in oklab, var(--surface) 88%, transparent);
  padding: 0.22rem 0.48rem;
  color: var(--muted-strong);
  font-size: 0.7rem;
  line-height: 1.25;
}

.vcetMetricExplanation {
  margin-top: 0.55rem;
  color: var(--muted);
  font-size: 0.76rem;
}

.vcetEvidenceWarning,
.vcetGridUnavailable {
  margin-top: 0.65rem;
  border: 1px solid color-mix(in oklab, var(--warning) 34%, var(--border));
  border-radius: 10px;
  background: var(--surface-warning);
  color: var(--warning-ink);
  padding: 0.58rem 0.66rem;
  font-size: 0.76rem;
  line-height: 1.38;
}

.vcetEvidenceWarning ul {
  margin: 0.35rem 0 0;
  padding-left: 1.15rem;
}

.vcetEvidenceLegend {
  align-items: center;
  margin-top: 0.7rem;
}

.marker {
  display: inline-grid;
  width: 1.15rem;
  height: 1.15rem;
  place-items: center;
  border-radius: 999px;
  font-size: 0.61rem;
  line-height: 1;
}

.markerSelected {
  background: var(--accent-strong);
  color: var(--on-accent);
}

.markerAdmitted {
  border: 1px solid color-mix(in oklab, var(--ok) 58%, var(--border));
  background: var(--ok-soft);
  color: var(--ok-ink);
}

.vcetMatrixScroll,
.vcetCandidateTableScroll {
  margin-top: 0.58rem;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: 11px;
  background: var(--surface);
}

.vcetMatrixScroll:focus-visible {
  outline: 2px solid var(--focus-border);
  outline-offset: 2px;
}

.vcetMatrix {
  width: 100%;
  min-width: max-content;
  border-collapse: separate;
  border-spacing: 0.24rem;
  padding: 0.3rem;
}

.vcetMatrix th {
  min-width: 4rem;
  padding: 0.25rem;
  color: var(--muted-strong);
  font-size: 0.7rem;
  text-align: center;
}

.vcetMatrix .axisCorner {
  min-width: 5rem;
  text-align: right;
}

.vcetMatrix td {
  min-width: 5rem;
  height: 3.1rem;
  padding: 0;
}

.vcetMetricCell {
  position: relative;
  width: 100%;
  min-width: 5rem;
  min-height: 3.1rem;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.18rem;
  border: 1px solid color-mix(in oklab, var(--border-strong) 76%, transparent);
  border-radius: 8px;
  background: var(--surface2);
  color: var(--text);
  padding: 0.35rem;
  font-family: var(--mono);
  font-size: 0.68rem;
  font-weight: 700;
  transition:
    transform var(--motion-fast) var(--ease-standard),
    box-shadow var(--motion-fast) var(--ease-standard),
    border-color var(--motion-fast) var(--ease-standard);
}

button.vcetMetricCell {
  cursor: pointer;
}

button.vcetMetricCell:hover {
  transform: translateY(-1px);
  border-color: var(--focus-border);
  box-shadow: var(--shadow-sm);
}

button.vcetMetricCell:focus-visible,
.vcetMetricCell.active {
  outline: 2px solid var(--focus-border);
  outline-offset: 1px;
  box-shadow: var(--focus-ring);
}

.vcetMetricCell.selected {
  border-color: var(--accent);
}

.vcetMetricCell.admitted:not(.selected) {
  border-color: color-mix(in oklab, var(--ok) 58%, var(--border));
}

.vcetMetricCell.unavailable,
.vcetMetricCell.tone-unavailable {
  background: color-mix(in oklab, var(--surface2) 84%, var(--muted));
  color: var(--muted);
}

.vcetMetricCell.tone-neutral {
  background: var(--surface2);
}

.vcetMetricCell.tone-favorable[data-intensity="1"] {
  background: color-mix(in oklab, var(--surface) 82%, var(--ok));
}

.vcetMetricCell.tone-favorable[data-intensity="2"] {
  background: color-mix(in oklab, var(--surface) 70%, var(--ok));
}

.vcetMetricCell.tone-favorable[data-intensity="3"] {
  background: color-mix(in oklab, var(--surface) 56%, var(--ok));
}

.vcetMetricCell.tone-favorable[data-intensity="4"] {
  background: color-mix(in oklab, var(--surface) 42%, var(--ok));
}

.vcetMetricCell.tone-risk[data-intensity="1"] {
  background: color-mix(in oklab, var(--surface) 84%, var(--danger));
}

.vcetMetricCell.tone-risk[data-intensity="2"] {
  background: color-mix(in oklab, var(--surface) 73%, var(--danger));
}

.vcetMetricCell.tone-risk[data-intensity="3"] {
  background: color-mix(in oklab, var(--surface) 62%, var(--danger));
}

.vcetMetricCell.tone-risk[data-intensity="4"] {
  background: color-mix(in oklab, var(--surface) 50%, var(--danger));
}

.cellValue {
  white-space: nowrap;
}

.vcetScatterPanel {
  margin-top: 0.65rem;
  border: 1px solid var(--border);
  border-radius: 11px;
  background: color-mix(in oklab, var(--surface) 92%, transparent);
  padding: 0.7rem;
}

.vcetScatterHeader {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
}

.vcetScatterHeader p,
.vcetScatterNote {
  margin: 0.24rem 0 0;
  color: var(--muted);
  font-size: 0.73rem;
  line-height: 1.4;
}

.vcetScatterAxisControls {
  display: grid;
  grid-template-columns: repeat(2, minmax(9rem, 1fr));
  gap: 0.55rem;
}

.vcetScatterAxisControls label {
  display: grid;
  gap: 0.22rem;
  color: var(--muted-strong);
  font-size: 0.69rem;
  font-weight: 700;
}

.vcetScatterPlot {
  position: relative;
  min-height: 23rem;
  margin: 0.75rem 1.8rem 1.65rem 2.8rem;
  border-left: 1px solid var(--border-strong);
  border-bottom: 1px solid var(--border-strong);
  border-radius: 8px;
  background:
    linear-gradient(to right, color-mix(in oklab, var(--border) 45%, transparent) 1px, transparent 1px),
    linear-gradient(to top, color-mix(in oklab, var(--border) 45%, transparent) 1px, transparent 1px),
    var(--surface-sunken);
  background-size: 25% 100%, 100% 25%;
}

.vcetScatterPoint {
  position: absolute;
  z-index: 1;
  width: 0.72rem;
  height: 0.72rem;
  transform: translate(-50%, 50%);
  border: 1px solid color-mix(in oklab, var(--border-strong) 82%, transparent);
  border-radius: 999px;
  background: var(--surface2);
  padding: 0;
  box-shadow: var(--shadow-sm);
  cursor: pointer;
  transition:
    transform var(--motion-fast) var(--ease-standard),
    box-shadow var(--motion-fast) var(--ease-standard);
}

.vcetScatterPoint:hover,
.vcetScatterPoint:focus-visible,
.vcetScatterPoint.active {
  z-index: 3;
  transform: translate(-50%, 50%) scale(1.55);
  outline: 2px solid var(--focus-border);
  outline-offset: 1px;
  box-shadow: var(--focus-ring);
}

.vcetScatterPoint.selected {
  z-index: 4;
  border: 2px solid var(--accent-strong);
}

.vcetScatterPoint.admitted:not(.selected) {
  border-color: var(--ok-ink);
}

.vcetScatterPoint.tone-favorable[data-intensity="1"] {
  background: color-mix(in oklab, var(--surface) 68%, var(--ok));
}

.vcetScatterPoint.tone-favorable[data-intensity="2"] {
  background: color-mix(in oklab, var(--surface) 52%, var(--ok));
}

.vcetScatterPoint.tone-favorable[data-intensity="3"] {
  background: color-mix(in oklab, var(--surface) 34%, var(--ok));
}

.vcetScatterPoint.tone-favorable[data-intensity="4"] {
  background: var(--ok);
}

.vcetScatterPoint.tone-risk[data-intensity="1"] {
  background: color-mix(in oklab, var(--surface) 72%, var(--danger));
}

.vcetScatterPoint.tone-risk[data-intensity="2"] {
  background: color-mix(in oklab, var(--surface) 56%, var(--danger));
}

.vcetScatterPoint.tone-risk[data-intensity="3"] {
  background: color-mix(in oklab, var(--surface) 38%, var(--danger));
}

.vcetScatterPoint.tone-risk[data-intensity="4"] {
  background: var(--danger);
}

.scatterAxisTitle,
.scatterBound {
  position: absolute;
  color: var(--muted-strong);
  font-size: 0.66rem;
  pointer-events: none;
}

.scatterAxisTitleX {
  left: 50%;
  bottom: -1.45rem;
  transform: translateX(-50%);
  font-weight: 700;
}

.scatterAxisTitleY {
  top: 50%;
  left: -2.3rem;
  transform: translate(-50%, -50%) rotate(-90deg);
  font-weight: 700;
  white-space: nowrap;
}

.scatterBoundYMax {
  top: 0.2rem;
  left: -0.45rem;
  transform: translateX(-100%);
}

.scatterBoundYMin {
  bottom: 0.2rem;
  left: -0.45rem;
  transform: translateX(-100%);
}

.scatterBoundXMin {
  left: 0.2rem;
  bottom: -1rem;
}

.scatterBoundXMax {
  right: 0.2rem;
  bottom: -1rem;
}

.vcetMetricCell .marker {
  position: absolute;
  top: 0.2rem;
  width: 0.9rem;
  height: 0.9rem;
  font-size: 0.52rem;
}

.vcetMetricCell .markerSelected {
  left: 0.2rem;
}

.vcetMetricCell .markerAdmitted {
  right: 0.2rem;
}

.vcetActiveEvidence {
  margin-top: 0.65rem;
  border: 1px solid var(--border);
  border-radius: 11px;
  background: color-mix(in oklab, var(--surface) 90%, transparent);
  padding: 0.65rem;
}

.vcetActiveHeading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.8rem;
}

.vcetActiveHeading > div:first-child {
  display: grid;
  gap: 0.16rem;
}

.vcetActiveMetrics {
  margin-top: 0.5rem;
}

.vcetActiveControls {
  margin-top: 0.42rem;
}

.vcetPlateauSummary {
  margin: 0.5rem 0 0;
  color: var(--muted-strong);
  font-size: 0.76rem;
  line-height: 1.4;
}

.vcetAccessibleTable {
  margin-top: 0.65rem;
  border-top: 1px solid var(--border);
  padding-top: 0.55rem;
}

.vcetAccessibleTable summary {
  cursor: pointer;
  color: var(--muted-strong);
  font-size: 0.76rem;
  font-weight: 700;
}

.vcetTablePagination {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.65rem;
  margin-top: 0.45rem;
  color: var(--muted);
  font-size: 0.72rem;
}

.vcetTablePagination > div {
  display: flex;
  gap: 0.35rem;
}

.vcetTablePagination button {
  border: 1px solid var(--border);
  border-radius: 7px;
  background: var(--surface);
  color: var(--text);
  padding: 0.3rem 0.5rem;
  font-size: 0.7rem;
  cursor: pointer;
}

.vcetTablePagination button:disabled {
  color: var(--muted);
  cursor: default;
  opacity: 0.58;
}

.vcetAccessibleTable table {
  width: 100%;
  min-width: 58rem;
  border-collapse: collapse;
  font-size: 0.72rem;
}

.vcetAccessibleTable table.multidimensionalTable {
  min-width: 98rem;
}

.vcetAccessibleTable caption {
  padding: 0.45rem;
  color: var(--muted);
  text-align: left;
}

.vcetAccessibleTable th,
.vcetAccessibleTable td {
  border-bottom: 1px solid var(--border);
  padding: 0.38rem 0.45rem;
  text-align: right;
  white-space: nowrap;
}

.vcetAccessibleTable th:first-child,
.vcetAccessibleTable td:first-child,
.vcetAccessibleTable th:last-child,
.vcetAccessibleTable td:last-child {
  text-align: left;
}

.vcetAccessibleTable .selectedRow {
  background: var(--selection-fill);
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 740px) {
  .vcetEvidenceHeader,
  .vcetActiveHeading,
  .vcetScatterHeader,
  .vcetTablePagination {
    display: grid;
  }

  .vcetMetricControl {
    width: 100%;
    max-width: none;
  }

  .vcetScatterAxisControls {
    grid-template-columns: 1fr;
    width: 100%;
  }

  .vcetScatterPlot {
    min-height: 18rem;
  }
}
</style>
