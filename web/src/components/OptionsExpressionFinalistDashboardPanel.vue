<script setup lang="ts">
import type { OptionsFinalistDashboardModel } from "../utils/optionsFinalistDashboardModel";
import { formatOptionsFinalistNumber } from "../utils/optionsFinalistDashboardModel";

defineProps<{
  dashboard: OptionsFinalistDashboardModel;
}>();
</script>

<template>
  <div
    class="optionsFinalistDashboard"
    data-qa-panel="options.finalistDashboard"
  >
    <div class="panelSectionHeader">
      <div>
        <div class="panelEyebrow">Finalist acceptance</div>
        <div class="panelSectionLead">
          {{ dashboard.acceptedTupleCount }} /
          {{ dashboard.validatedTupleCount }} accepted ·
          {{ dashboard.verdict }}
        </div>
      </div>
    </div>
    <div class="gateBadgeRow finalistGateGrid">
      <span
        v-for="gate in dashboard.gates"
        :key="gate.key"
        class="gateBadge finalistGate"
        :class="{
          isPass: gate.tone === 'pass',
          isPending: gate.tone === 'pending',
          isMixed: gate.tone === 'mixed',
        }"
        :title="gate.detail"
      >
        {{ gate.label }} · {{ gate.status }}
      </span>
    </div>
    <div v-if="dashboard.rows.length" class="tableWrap" tabindex="0">
      <table class="resultTable" aria-label="Options finalist dashboard">
        <thead>
          <tr>
            <th>Tuple</th>
            <th>Verdict</th>
            <th>Support</th>
            <th>Robust</th>
            <th>Search EV</th>
            <th>Holdout EV</th>
            <th>Stress EV</th>
            <th>Harsher EV</th>
            <th>Extreme EV</th>
            <th>Strongest stress</th>
            <th>Trades</th>
            <th>Drift</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in dashboard.rows" :key="row.key">
            <td class="mono">{{ row.tupleLabel }}</td>
            <td>
              <span
                class="gateBadge miniBadge"
                :class="{
                  isPass: row.tone === 'pass',
                  isPending: row.tone === 'pending',
                  isMixed: row.tone === 'mixed',
                }"
              >
                {{ row.verdict }}
              </span>
            </td>
            <td class="mono">{{ row.support }}</td>
            <td class="mono">
              {{ formatOptionsFinalistNumber(row.validatedRobustScore) }}
            </td>
            <td class="mono">
              {{ formatOptionsFinalistNumber(row.searchEv) }}
            </td>
            <td class="mono">
              {{ formatOptionsFinalistNumber(row.holdoutEv) }}
            </td>
            <td class="mono">
              {{ formatOptionsFinalistNumber(row.stressEv) }}
            </td>
            <td class="mono">
              {{ formatOptionsFinalistNumber(row.harsherStressEv) }}
            </td>
            <td class="mono">
              {{ formatOptionsFinalistNumber(row.extremeStressEv) }}
            </td>
            <td class="mono">
              {{ row.strongestStressLabel }} ·
              {{ formatOptionsFinalistNumber(row.strongestStressEv) }}
            </td>
            <td class="mono">
              {{ formatOptionsFinalistNumber(row.validTrades, 0) }}
            </td>
            <td class="mono">{{ row.drift || "—" }}</td>
          </tr>
        </tbody>
      </table>
    </div>
    <div v-else class="panelMuted">
      No tuple-level validation rows were included in this artifact.
    </div>
  </div>
</template>

<style scoped>
.optionsFinalistDashboard {
  display: grid;
  gap: 0.65rem;
  border-top: 1px solid var(--border);
  padding-top: 0.75rem;
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

.finalistGateGrid {
  align-items: flex-start;
}

.finalistGate {
  max-width: 100%;
}

.miniBadge {
  padding: 0.18rem 0.4rem;
  font-size: 0.7rem;
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
</style>
