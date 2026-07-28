<script setup lang="ts">
import AppVisualIcon from "./AppVisualIcon.vue";
import { optionsFeasibilityPreviewIcon } from "../utils/appSvgAssets";

interface KeyValueRow {
  key: string;
  label: string;
  value: string;
}

interface FeasibilityGateRow extends KeyValueRow {
  passed: boolean;
  pending: boolean;
}

interface CoverageAdequacy {
  label: string;
  detail: string;
  tone: "pass" | "pending" | "mixed";
}

interface InvalidReasonRow {
  reason: string;
  count: number;
}

interface SplitInvalidSection {
  key: string;
  label: string;
  rows: readonly InvalidReasonRow[];
}

interface LaneRow {
  key: string;
  label: string;
  summaries: string;
  density: string;
  compileMs: string;
  bytes: string;
}

interface InvalidReasonShareRow extends InvalidReasonRow {
  share: number;
}

interface AxisCoverageRow {
  key: string;
  bucket: string;
  validSlots: string;
  validRate: string;
  invalidReasons: string;
}

interface AxisCoverageSection {
  key: string;
  label: string;
  bucketLabel: string;
  rows: readonly AxisCoverageRow[];
}

defineProps<{
  overallStatus: "pass" | "mixed" | "pending";
  gateRows: readonly FeasibilityGateRow[];
  coverageAdequacy: CoverageAdequacy | null;
  summaryRows: readonly KeyValueRow[];
  overlapRows: readonly KeyValueRow[];
  splitInvalidSections: readonly SplitInvalidSection[];
  structuralCacheBytes: string;
  laneRows: readonly LaneRow[];
  invalidReasonRows: readonly InvalidReasonShareRow[];
  axisCoverageSections: readonly AxisCoverageSection[];
  invalidReasons: string;
  recommendations: readonly string[];
}>();

function formatPercent(value: number): string {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? `${(parsed * 100).toFixed(1)}%` : "—";
}
</script>

<template>
  <section class="resultCard">
    <div class="panelSectionHeader">
      <div class="panelHeaderVisual">
        <AppVisualIcon
          :icon="optionsFeasibilityPreviewIcon"
          class="sectionGlyph"
        />
        <div>
          <div class="panelEyebrow">Phase 0.5 preview</div>
          <div class="panelSectionLead">
            Review candidate density, event coverage, and replay/compile timing
            before committing to a full optimization run or GPU work.
          </div>
        </div>
      </div>
    </div>
    <div class="gateBadgeRow">
      <span
        class="gateBadge"
        :class="{
          isPass: overallStatus === 'pass',
          isPending: overallStatus === 'pending',
          isMixed: overallStatus === 'mixed',
        }"
      >
        GPU viability
        {{
          overallStatus === "pass"
            ? "pass"
            : overallStatus === "pending"
              ? "pending"
              : "mixed"
        }}
      </span>
      <span
        v-for="row in gateRows"
        :key="row.key"
        class="gateBadge"
        :class="{
          isPass: row.passed,
          isPending: row.pending && !row.passed,
          isMixed: !row.passed && !row.pending,
        }"
      >
        {{ row.label }} · {{ row.value }}
      </span>
    </div>
    <div v-if="coverageAdequacy" class="gateBadgeRow">
      <span
        class="gateBadge"
        :class="{
          isPass: coverageAdequacy.tone === 'pass',
          isPending: coverageAdequacy.tone === 'pending',
          isMixed: coverageAdequacy.tone === 'mixed',
        }"
      >
        Coverage {{ coverageAdequacy.label }}
      </span>
    </div>
    <div v-if="coverageAdequacy" class="panelMuted">
      {{ coverageAdequacy.detail }}
    </div>
    <div class="detailGrid detailGridTight">
      <div v-for="row in summaryRows" :key="row.key">
        <span class="panelMetricKey">{{ row.label }}</span>
        <span class="panelMetricValue mono panelNumeric">{{ row.value }}</span>
      </div>
    </div>
    <details v-if="overlapRows.length > 0" class="panelDetails">
      <summary>Overlap adequacy</summary>
      <div class="panelDetailsBody">
        <div class="detailGrid detailGridTight">
          <div v-for="row in overlapRows" :key="row.key">
            <span class="panelMetricKey">{{ row.label }}</span>
            <span class="panelMetricValue mono panelNumeric">{{
              row.value
            }}</span>
          </div>
        </div>
      </div>
    </details>
    <details v-if="splitInvalidSections.length > 0" class="panelDetails">
      <summary>Split invalid reasons</summary>
      <div class="panelDetailsBody">
        <div
          v-for="section in splitInvalidSections"
          :key="section.key"
          class="coverageSection"
        >
          <div class="panelMuted">{{ section.label }}</div>
          <div class="tableWrap" tabindex="0">
            <table
              class="resultTable"
              :aria-label="`${section.label} split invalid reasons`"
            >
              <thead>
                <tr>
                  <th>Reason</th>
                  <th>Count</th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="row in section.rows"
                  :key="`${section.key}-${row.reason}`"
                >
                  <td class="mono">{{ row.reason }}</td>
                  <td class="mono">{{ row.count }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </details>
    <details class="panelDetails">
      <summary>Lane diagnostics</summary>
      <div class="panelDetailsBody">
        <div class="detailGrid detailGridTight">
          <div>
            <span class="panelMetricKey">Structural cache</span>
            <span class="panelMetricValue mono panelNumeric">
              {{ structuralCacheBytes }}
            </span>
          </div>
        </div>
        <div class="tableWrap" tabindex="0">
          <table
            class="resultTable"
            aria-label="Options feasibility lane diagnostics"
          >
            <thead>
              <tr>
                <th>Lane</th>
                <th>Compiled / Total summaries</th>
                <th>Density</th>
                <th>Compile time</th>
                <th>Bytes</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="row in laneRows" :key="row.key">
                <td class="mono">{{ row.label }}</td>
                <td class="mono">{{ row.summaries }}</td>
                <td class="mono">{{ row.density }}</td>
                <td class="mono">{{ row.compileMs }}</td>
                <td class="mono">{{ row.bytes }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </details>
    <details v-if="invalidReasonRows.length > 0" class="panelDetails">
      <summary>Invalid reasons</summary>
      <div class="panelDetailsBody tableWrap" tabindex="0">
        <table
          class="resultTable"
          aria-label="Options feasibility invalid reasons"
        >
          <thead>
            <tr>
              <th>Reason</th>
              <th>Count</th>
              <th>Share</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="row in invalidReasonRows" :key="row.reason">
              <td class="mono">{{ row.reason }}</td>
              <td class="mono">{{ row.count }}</td>
              <td class="mono">{{ formatPercent(row.share) }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </details>
    <details v-if="axisCoverageSections.length > 0" class="panelDetails">
      <summary>Bucket coverage</summary>
      <div class="panelDetailsBody">
        <div
          v-for="section in axisCoverageSections"
          :key="section.key"
          class="coverageSection"
        >
          <div class="panelMuted">{{ section.label }}</div>
          <div class="tableWrap" tabindex="0">
            <table
              class="resultTable"
              :aria-label="`${section.label} bucket coverage`"
            >
              <thead>
                <tr>
                  <th>{{ section.bucketLabel }}</th>
                  <th>Valid / Total slots</th>
                  <th>Valid rate</th>
                  <th>Top invalid reasons</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="row in section.rows" :key="row.key">
                  <td class="mono">{{ row.bucket }}</td>
                  <td class="mono">{{ row.validSlots }}</td>
                  <td class="mono">{{ row.validRate }}</td>
                  <td class="mono">{{ row.invalidReasons }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </details>
    <div v-if="invalidReasons" class="panelMuted mono">
      Invalid reasons: {{ invalidReasons }}
    </div>
    <details v-if="recommendations.length > 0" class="panelDetails">
      <summary>Recommendations</summary>
      <div class="panelDetailsBody">
        <ul class="recommendationList">
          <li v-for="recommendation in recommendations" :key="recommendation">
            {{ recommendation }}
          </li>
        </ul>
      </div>
    </details>
  </section>
</template>

<style scoped>
.resultCard {
  display: grid;
  gap: 0.75rem;
  border-top: 1px solid var(--border);
  padding-top: 0.75rem;
}

.panelHeaderVisual {
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
}

.sectionGlyph {
  width: 1.05rem;
  height: 1.05rem;
  flex: 0 0 auto;
  margin-top: 0.08rem;
}

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

.coverageSection {
  display: grid;
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

.recommendationList {
  display: grid;
  gap: 0.45rem;
  margin: 0;
  padding-left: 1.1rem;
}

@media (max-width: 980px) {
  .detailGrid {
    grid-template-columns: 1fr;
  }
}
</style>
