<script setup lang="ts">
interface KeyValueRow {
  key: string;
  label: string;
  value: string;
}

interface FrontierLeaderRow {
  key: string;
  label: string;
  artifactLabel: string;
  metricLabel: string;
  value: number | null;
  validation: string;
  drift: number | null;
  stressDrop: number | null;
}

interface ArtifactMetaGroupRow {
  key: string;
  label: string;
  artifactCount: number;
  validatedCount: number;
  fragileCount: number;
  avgRobust: number | null;
  avgSelectionEv: number | null;
  avgValidationEv: number | null;
  avgStressDrop: number | null;
}

interface ArtifactMetaSection {
  key: string;
  label: string;
  rows: readonly ArtifactMetaGroupRow[];
}

interface ArtifactLaneDisagreementRow {
  key: string;
  label: string;
  laneCount: number;
  selectionWinnerLane: string;
  validationWinnerLane: string;
  driftLabel: string;
  validationSpread: number | null;
  laneSummary: string;
}

defineProps<{
  frontierLeaderRows: readonly FrontierLeaderRow[];
  showMetaAnalysis: boolean;
  metaOverviewRows: readonly KeyValueRow[];
  metaSections: readonly ArtifactMetaSection[];
  laneDisagreementRows: readonly ArtifactLaneDisagreementRow[];
}>();

function formatOptionalDecimal(
  value: number | null | undefined,
  digits = 4,
): string {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed.toFixed(digits) : "—";
}
</script>

<template>
  <details v-if="frontierLeaderRows.length > 0" class="panelDetails">
    <summary>Artifact frontier leaders</summary>
    <div class="panelDetailsBody tableWrap" tabindex="0">
      <table class="resultTable" aria-label="Options artifact frontier leaders">
        <thead>
          <tr>
            <th>Stage</th>
            <th>Artifact</th>
            <th>Metric</th>
            <th>Validation</th>
            <th>Val Δ</th>
            <th>Stress Δ</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in frontierLeaderRows" :key="row.key">
            <td class="mono">{{ row.label }}</td>
            <td class="mono">{{ row.artifactLabel }}</td>
            <td class="mono">
              {{ row.metricLabel }} ·
              {{ formatOptionalDecimal(row.value, 3) }}
            </td>
            <td class="mono">{{ row.validation }}</td>
            <td class="mono">{{ formatOptionalDecimal(row.drift, 3) }}</td>
            <td class="mono">
              {{ formatOptionalDecimal(row.stressDrop, 3) }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </details>

  <details v-if="showMetaAnalysis" class="panelDetails">
    <summary>Options artifact meta-analysis</summary>
    <div class="panelDetailsBody">
      <div class="panelMuted">
        Cross-artifact summary across all saved options-expression artifacts
        currently indexed in the app data store.
      </div>
      <div class="detailGrid detailGridTight">
        <div v-for="row in metaOverviewRows" :key="row.key">
          <span class="panelMetricKey">{{ row.label }}</span>
          <span class="panelMetricValue mono panelNumeric">{{
            row.value
          }}</span>
        </div>
      </div>
      <div
        v-for="section in metaSections"
        :key="section.key"
        class="coverageSection"
      >
        <div class="panelMuted">{{ section.label }}</div>
        <div class="tableWrap" tabindex="0">
          <table
            class="resultTable"
            :aria-label="`${section.label} artifact meta-analysis`"
          >
            <thead>
              <tr>
                <th>Group</th>
                <th>Artifacts</th>
                <th>Validated</th>
                <th>Fragile</th>
                <th>Avg robust</th>
                <th>Avg sel EV</th>
                <th>Avg validated EV</th>
                <th>Avg stress drop</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="row in section.rows" :key="row.key">
                <td class="mono">{{ row.label }}</td>
                <td class="mono">{{ row.artifactCount }}</td>
                <td class="mono">{{ row.validatedCount }}</td>
                <td class="mono">{{ row.fragileCount }}</td>
                <td class="mono">
                  {{ formatOptionalDecimal(row.avgRobust, 3) }}
                </td>
                <td class="mono">
                  {{ formatOptionalDecimal(row.avgSelectionEv, 3) }}
                </td>
                <td class="mono">
                  {{ formatOptionalDecimal(row.avgValidationEv, 3) }}
                </td>
                <td class="mono">
                  {{ formatOptionalDecimal(row.avgStressDrop, 3) }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
      <div v-if="laneDisagreementRows.length > 0" class="coverageSection">
        <div class="panelMuted">Realism lane disagreement</div>
        <div class="tableWrap" tabindex="0">
          <table class="resultTable" aria-label="Realism lane disagreement">
            <thead>
              <tr>
                <th>Group</th>
                <th>Lanes</th>
                <th>Selection winner</th>
                <th>Validation winner</th>
                <th>Drift</th>
                <th>Validation spread</th>
                <th>Lane summary</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="row in laneDisagreementRows" :key="row.key">
                <td class="mono">{{ row.label }}</td>
                <td class="mono">{{ row.laneCount }}</td>
                <td class="mono">{{ row.selectionWinnerLane }}</td>
                <td class="mono">{{ row.validationWinnerLane }}</td>
                <td class="mono">{{ row.driftLabel }}</td>
                <td class="mono">
                  {{ formatOptionalDecimal(row.validationSpread, 3) }}
                </td>
                <td class="mono">{{ row.laneSummary }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  </details>
</template>

<style scoped>
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

@media (max-width: 980px) {
  .detailGrid {
    grid-template-columns: 1fr;
  }
}
</style>
