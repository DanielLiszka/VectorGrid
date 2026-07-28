<script setup lang="ts">
import { computed } from "vue";
import OptionsExpressionFinalistDashboardPanel from "./OptionsExpressionFinalistDashboardPanel.vue";
import type {
  AnalyzeOptionsExpressionValidationFeasibilitySummary,
  OptionsExpressionValidationAcceptanceSummary,
  OptionsExpressionValidationPlanSummary,
  ValidateOptionsExpressionArtifactSummary,
} from "../services/optionsExpressionService";
import type { OptionsFinalistDashboardModel } from "../utils/optionsFinalistDashboardModel";

interface KeyValueRow {
  key: string;
  label: string;
  value: string;
}

const props = defineProps<{
  feasibility: AnalyzeOptionsExpressionValidationFeasibilitySummary | null;
  acceptance: OptionsExpressionValidationAcceptanceSummary | null;
  validation: ValidateOptionsExpressionArtifactSummary | null;
  finalistDashboard: OptionsFinalistDashboardModel | null;
}>();

const quoteQualityRows = computed<KeyValueRow[]>(() => {
  const quality = props.validation?.result.datasetProvenance?.quoteQuality;
  if (!quality) return [];
  return [
    {
      key: "quote-rows",
      label: "Quote rows",
      value: `${quality.anyQuoteRowCount} / ${quality.rowCount}`,
    },
    {
      key: "bid-ask-coverage",
      label: "Bid/ask coverage",
      value: formatPercent(quality.bidAskCoverageRate, 1),
    },
    {
      key: "median-spread",
      label: "Median spread",
      value: `${formatOptionalDecimal(quality.medianSpreadBps, 1)} bps`,
    },
    {
      key: "stale-quotes",
      label: "Stale quotes",
      value: formatPercent(quality.staleQuoteRate, 1),
    },
    {
      key: "quote-warnings",
      label: "Quote warnings",
      value: String(quality.warnings.length),
    },
    {
      key: "invalid-bid-ask",
      label: "Invalid bid/ask",
      value: String(quality.invalidBidAskRowCount),
    },
  ];
});

const quoteQualityWarnings = computed(
  () =>
    props.validation?.result.datasetProvenance?.quoteQuality?.warnings.join(
      " ",
    ) ?? "",
);

function formatOptionalDecimal(
  value: number | null | undefined,
  digits = 4,
): string {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed.toFixed(digits) : "—";
}

function formatPercent(value: number | null | undefined, digits = 1): string {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? `${(parsed * 100).toFixed(digits)}%` : "—";
}

function formatTimestamp(value: number | null | undefined): string {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed <= 0) return "—";
  try {
    return new Date(parsed).toISOString();
  } catch {
    return "—";
  }
}

function formatValidationPlanMode(value: string | null | undefined): string {
  switch (String(value ?? "").trim()) {
    case "session_chronological_replay":
      return "Session replay";
    case "date_range_holdout":
      return "Date-range holdout";
    case "anchored_walk_forward":
      return "Anchored walk-forward";
    case "rolling_session_walk_forward":
      return "Rolling walk-forward";
    case "original_embedded_splits":
    default:
      return "Original splits";
  }
}

function formatValidationPlanDetail(
  plan: OptionsExpressionValidationPlanSummary | null | undefined,
): string {
  if (!plan) return "";
  if (plan.mode === "date_range_holdout") {
    return `${formatTimestamp(plan.startTimestampMs)} to ${formatTimestamp(
      plan.endTimestampMs,
    )}`;
  }
  if (
    plan.mode === "anchored_walk_forward" ||
    plan.mode === "rolling_session_walk_forward"
  ) {
    return `train ${plan.trainSessionCount ?? "—"} · validate ${
      plan.validationSessionCount ?? "—"
    } · step ${plan.stepSessionCount ?? "—"}`;
  }
  return "";
}
</script>

<template>
  <div v-if="feasibility" class="panelMuted mono">
    {{ feasibility.selectedTupleCount }} selected tuple{{
      feasibility.selectedTupleCount === 1 ? "" : "s"
    }}
    · {{ feasibility.datasetPolicy }} · {{ feasibility.optionsDatasetId }} ·
    {{ feasibility.priceSemantics }} ·
    {{ formatValidationPlanMode(feasibility.validationPlan.mode) }}
    <template v-if="formatValidationPlanDetail(feasibility.validationPlan)">
      ·
      {{ formatValidationPlanDetail(feasibility.validationPlan) }}
    </template>
  </div>
  <div v-if="feasibility?.notes.length" class="panelMuted">
    {{ feasibility.notes.join(" ") }}
  </div>

  <div v-if="acceptance" class="gateBadgeRow">
    <span
      class="gateBadge"
      :class="{
        isPass: acceptance.verdict === 'accepted',
        isPending: acceptance.verdict === 'support_deficient',
        isMixed:
          acceptance.verdict === 'warning' || acceptance.verdict === 'rejected',
      }"
    >
      Subsequent validation ·
      {{ acceptance.verdict.replace(/_/g, " ") }}
    </span>
    <span v-if="validation" class="gateBadge isPending">
      {{ formatValidationPlanMode(validation.result.validationPlan.mode) }}
      <template
        v-if="formatValidationPlanDetail(validation.result.validationPlan)"
      >
        ·
        {{ formatValidationPlanDetail(validation.result.validationPlan) }}
      </template>
    </span>
  </div>
  <div v-if="acceptance" class="detailGrid detailGridTight">
    <div>
      <span class="panelMetricKey">Accepted</span>
      <span class="panelMetricValue mono panelNumeric">
        {{ acceptance.acceptedTupleCount }} /
        {{ acceptance.validatedTupleCount }}
      </span>
    </div>
    <div>
      <span class="panelMetricKey">Support deficient</span>
      <span class="panelMetricValue mono panelNumeric">
        {{ acceptance.supportDeficientTupleCount }}
      </span>
    </div>
    <div>
      <span class="panelMetricKey">Stress fragile</span>
      <span class="panelMetricValue mono panelNumeric">
        {{ acceptance.stressFragileTupleCount }}
      </span>
    </div>
    <div>
      <span class="panelMetricKey">Best validation EV</span>
      <span class="panelMetricValue mono panelNumeric">
        {{
          formatOptionalDecimal(acceptance.bestValidatedExpectancyPerPremium, 3)
        }}
      </span>
    </div>
  </div>
  <div v-if="acceptance?.notes.length" class="panelMuted">
    {{ acceptance.notes.join(" ") }}
  </div>
  <div v-if="quoteQualityRows.length > 0" class="detailGrid detailGridTight">
    <div v-for="row in quoteQualityRows" :key="row.key">
      <span class="panelMetricKey">{{ row.label }}</span>
      <span class="panelMetricValue mono panelNumeric">{{ row.value }}</span>
    </div>
  </div>
  <div v-if="quoteQualityWarnings" class="panelMuted">
    {{ quoteQualityWarnings }}
  </div>

  <div
    v-if="validation?.result.laneSummaries.length"
    class="tableWrap"
    tabindex="0"
  >
    <table class="resultTable" aria-label="Options validation lane summaries">
      <thead>
        <tr>
          <th>Lane</th>
          <th>Supported</th>
          <th>Best EV</th>
          <th>Worst EV</th>
          <th>Median EV</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="row in validation.result.laneSummaries" :key="row.lane">
          <td class="mono">{{ row.lane }}</td>
          <td class="mono">
            {{ row.supportedTupleCount }} / {{ row.tupleCount }}
          </td>
          <td class="mono">
            {{ formatOptionalDecimal(row.bestExpectancyPerPremium, 3) }}
          </td>
          <td class="mono">
            {{ formatOptionalDecimal(row.worstExpectancyPerPremium, 3) }}
          </td>
          <td class="mono">
            {{ formatOptionalDecimal(row.medianExpectancyPerPremium, 3) }}
          </td>
        </tr>
      </tbody>
    </table>
  </div>

  <OptionsExpressionFinalistDashboardPanel
    v-if="finalistDashboard"
    :dashboard="finalistDashboard"
  />
</template>

<style scoped>
.gateBadgeRow {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.gateBadge {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  border-radius: 999px;
  border: 1px solid var(--border);
  padding: 0.3rem 0.55rem;
  font-size: 0.76rem;
  background: color-mix(in srgb, var(--panel) 82%, transparent);
}

.gateBadge.isPass {
  border-color: color-mix(in srgb, var(--success, #4caf50) 45%, var(--border));
  color: color-mix(in srgb, var(--success, #4caf50) 78%, white);
}

.gateBadge.isPending {
  border-color: color-mix(in srgb, var(--accent, #4da3ff) 35%, var(--border));
  color: color-mix(in srgb, var(--accent, #4da3ff) 80%, white);
}

.gateBadge.isMixed {
  border-color: color-mix(in srgb, var(--warning, #f0b429) 45%, var(--border));
  color: color-mix(in srgb, var(--warning, #f0b429) 85%, white);
}

.detailGrid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.7rem;
}

.detailGridTight {
  gap: 0.55rem;
}

.tableWrap {
  overflow: auto;
}

.resultTable {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.78rem;
}

.resultTable th,
.resultTable td {
  border-bottom: 1px solid var(--border);
  padding: 0.45rem 0.4rem;
  text-align: left;
}

.resultTable tbody tr {
  cursor: pointer;
}

@media (max-width: 980px) {
  .detailGrid {
    grid-template-columns: 1fr;
  }
}
</style>
