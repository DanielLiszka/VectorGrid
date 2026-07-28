<script setup lang="ts">
import { computed } from "vue";

interface ValidationScopeRow {
  key: string;
  label: string;
  value: string;
}

interface ValidationValueRow extends ValidationScopeRow {
  scopeLabel: string;
}

interface ValidationStateRow {
  key: string;
  label: string;
  statusLabel: string;
  scopeLabel: string;
  detail: string;
}

const props = defineProps<{
  scopeRows: readonly ValidationScopeRow[];
  driftRows: readonly ValidationValueRow[];
  monotonicityRows: readonly ValidationStateRow[];
  regretRows: readonly ValidationValueRow[];
}>();

const validationTables = computed(() =>
  [
    {
      key: "drift",
      ariaLabel: "Selected tuple validation drift",
      headers: ["Transition", "Delta EV / Premium", "Scope"],
      rows: props.driftRows.map((row) => ({
        key: row.key,
        cells: [row.label, row.value, row.scopeLabel],
      })),
    },
    {
      key: "monotonicity",
      ariaLabel: "Selected tuple validation monotonicity",
      headers: ["Family", "Status", "Scope", "Detail"],
      rows: props.monotonicityRows.map((row) => ({
        key: row.key,
        cells: [row.label, row.statusLabel, row.scopeLabel, row.detail],
      })),
    },
    {
      key: "regret",
      ariaLabel: "Selected tuple validation regret",
      headers: ["Lane", "Regret", "Scope"],
      rows: props.regretRows.map((row) => ({
        key: row.key,
        cells: [row.label, row.value, row.scopeLabel],
      })),
    },
  ].filter((table) => table.rows.length > 0),
);
</script>

<template>
  <details
    v-if="
      props.scopeRows.length > 0 ||
      props.driftRows.length > 0 ||
      props.monotonicityRows.length > 0 ||
      props.regretRows.length > 0
    "
    class="panelDetails"
    open
  >
    <summary>Validation interpretation</summary>
    <div class="panelDetailsBody">
      <div
        v-if="props.scopeRows.length > 0"
        class="detailGrid detailGridTight"
      >
        <div v-for="row in props.scopeRows" :key="row.key">
          <span class="panelMetricKey">{{ row.label }}</span>
          <span class="panelMetricValue mono">{{ row.value }}</span>
        </div>
      </div>
      <div class="panelMuted">
        Support rows above reflect valid trades over total events for this
        retained tuple. They describe retained evidence depth, not a confidence
        interval. Regret rows below compare only within the stated scope.
      </div>
      <div
        v-for="table in validationTables"
        :key="table.key"
        class="tableWrap"
        tabindex="0"
      >
        <table class="resultTable" :aria-label="table.ariaLabel">
          <thead>
            <tr>
              <th v-for="header in table.headers" :key="header">
                {{ header }}
              </th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="row in table.rows" :key="row.key">
              <td
                v-for="(cell, cellIndex) in row.cells"
                :key="cellIndex"
                :class="{ mono: cellIndex < 2 }"
              >
                {{ cell }}
              </td>
            </tr>
          </tbody>
        </table>
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
