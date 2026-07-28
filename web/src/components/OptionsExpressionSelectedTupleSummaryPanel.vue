<script setup lang="ts">
import { computed } from "vue";
import OptionsExpressionValidationInterpretationPanel from "./OptionsExpressionValidationInterpretationPanel.vue";
import type { OptionsExpressionTupleSummary } from "../services/optionsExpressionService";

interface KeyValueRow {
  key: string;
  label: string;
  value: string;
}

interface DetailRow extends KeyValueRow {
  spanTwo?: boolean;
  prose?: boolean;
}

interface ValidationSummary {
  label: string;
  detail: string;
}

interface ValidationFrontierRow {
  key: string;
  label: string;
  status: string;
  support: string;
  expectancy: number;
  delta: number;
}

interface ValidationScopeRow {
  key: string;
  label: string;
  value: string;
}

interface ValidationValueRow extends ValidationScopeRow {
  detail: string;
  scopeLabel: string;
}

interface ValidationStateRow {
  key: string;
  label: string;
  statusLabel: string;
  detail: string;
  scopeLabel: string;
}

const props = defineProps<{
  tuple: OptionsExpressionTupleSummary;
  validation: ValidationSummary | null;
  supportRows: readonly KeyValueRow[];
  validationFrontierRows: readonly ValidationFrontierRow[];
  validationScopeRows: readonly ValidationScopeRow[];
  validationDriftRows: readonly ValidationValueRow[];
  validationMonotonicityRows: readonly ValidationStateRow[];
  validationRegretRows: readonly ValidationValueRow[];
}>();

const detailRows = computed<DetailRow[]>(() => {
  const tuple = props.tuple;
  const optionalMetrics: Array<DetailRow | null> = [
    tuple.holdoutMetrics
      ? {
          key: "holdout-ev",
          label: "Holdout EV / premium",
          value: tuple.holdoutMetrics.expectancyPerPremium.toFixed(4),
        }
      : null,
    tuple.sealedEvaluationMetrics
      ? {
          key: "sealed-ev",
          label: "Sealed EV / premium",
          value: tuple.sealedEvaluationMetrics.expectancyPerPremium.toFixed(4),
        }
      : null,
    tuple.stressMetrics
      ? {
          key: "stress-ev",
          label: "Stress EV / premium",
          value: tuple.stressMetrics.expectancyPerPremium.toFixed(4),
        }
      : null,
    tuple.sealedStressMetrics
      ? {
          key: "sealed-stress-ev",
          label: "Sealed stress EV / premium",
          value: tuple.sealedStressMetrics.expectancyPerPremium.toFixed(4),
        }
      : null,
    tuple.harsherStressMetrics
      ? {
          key: "harsher-ev",
          label: "Harsher EV / premium",
          value: tuple.harsherStressMetrics.expectancyPerPremium.toFixed(4),
        }
      : null,
    tuple.sealedHarsherStressMetrics
      ? {
          key: "sealed-harsher-ev",
          label: "Sealed harsher EV / premium",
          value:
            tuple.sealedHarsherStressMetrics.expectancyPerPremium.toFixed(4),
        }
      : null,
    tuple.extremeStressMetrics
      ? {
          key: "extreme-ev",
          label: "Extreme EV / premium",
          value: tuple.extremeStressMetrics.expectancyPerPremium.toFixed(4),
        }
      : null,
    tuple.sealedExtremeStressMetrics
      ? {
          key: "sealed-extreme-ev",
          label: "Sealed extreme EV / premium",
          value:
            tuple.sealedExtremeStressMetrics.expectancyPerPremium.toFixed(4),
        }
      : null,
  ];

  return [
    {
      key: "validation",
      label: "Validation",
      value: props.validation?.label ?? "Search only",
    },
    {
      key: "shape",
      label: "Delay / DTE / Hold",
      value: `${tuple.delayMinutes} / ${tuple.dteDays} / ${tuple.holdMinutes}`,
    },
    {
      key: "moneyness",
      label: "Moneyness",
      value: tuple.moneynessBucket,
    },
    {
      key: "structure",
      label: "Structure",
      value: tuple.structureFamily || "long_single",
    },
    {
      key: "target-stop",
      label: "Target / Stop",
      value: `${tuple.targetReturn == null ? "—" : tuple.targetReturn} / ${
        tuple.stopReturn == null ? "—" : tuple.stopReturn
      }`,
    },
    {
      key: "exit-template",
      label: "Exit template",
      value: tuple.exitTemplate,
    },
    {
      key: "fill-model",
      label: "Fill model",
      value: tuple.fillModel,
    },
    {
      key: "robust-score",
      label: "Robust score",
      value: tuple.robustScore.toFixed(3),
    },
    {
      key: "pre-sealed-rank",
      label: "Pre-sealed rank",
      value: String(tuple.preSealedRank ?? "—"),
    },
    {
      key: "sealed-final-rank",
      label: "Sealed final rank",
      value: String(tuple.sealedFinalRank ?? "—"),
    },
    {
      key: "sealed-finalist",
      label: "Sealed finalist",
      value: tuple.sealedFinalist ? "yes" : "no",
    },
    {
      key: "validation-note",
      label: "Validation note",
      value:
        props.validation?.detail ??
        "No supported holdout or sealed trades are present yet.",
      spanTwo: true,
      prose: true,
    },
    ...props.supportRows,
    {
      key: "ev-trade",
      label: "EV / Trade",
      value: tuple.selectionMetrics.expectancyPerTrade.toFixed(4),
    },
    {
      key: "ev-premium",
      label: "EV / Premium",
      value: tuple.selectionMetrics.expectancyPerPremium.toFixed(4),
    },
    {
      key: "median-return",
      label: "Median return / premium",
      value: tuple.selectionMetrics.medianReturnOnPremium.toFixed(4),
    },
    {
      key: "win-rate",
      label: "Win rate",
      value: tuple.selectionMetrics.winRate.toFixed(3),
    },
    {
      key: "valid-invalid",
      label: "Valid / Invalid",
      value: `${tuple.selectionMetrics.validTrades} / ${tuple.selectionMetrics.invalidEvents}`,
    },
    {
      key: "pnl-contract",
      label: "PnL / contract",
      value: tuple.selectionMetrics.totalPnlPerContract.toFixed(4),
    },
    ...optionalMetrics.filter((row): row is DetailRow => row != null),
  ];
});
</script>

<template>
  <div class="detailGrid detailGridTight">
    <div
      v-for="row in detailRows"
      :key="row.key"
      :class="{ detailGridSpan2: row.spanTwo }"
    >
      <span class="panelMetricKey">{{ row.label }}</span>
      <span
        class="panelMetricValue"
        :class="{ mono: !row.prose, panelNumeric: !row.prose }"
      >
        {{ row.value }}
      </span>
    </div>
  </div>
  <details v-if="validationFrontierRows.length > 0" class="panelDetails" open>
    <summary>Validation frontier</summary>
    <div class="panelDetailsBody tableWrap" tabindex="0">
      <table
        class="resultTable"
        aria-label="Selected tuple validation frontier"
      >
        <thead>
          <tr>
            <th>Stage</th>
            <th>Status</th>
            <th>Support</th>
            <th>EV / Premium</th>
            <th>Delta vs selection</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in validationFrontierRows" :key="row.key">
            <td class="mono">{{ row.label }}</td>
            <td class="mono">{{ row.status }}</td>
            <td class="mono">{{ row.support }}</td>
            <td class="mono">{{ row.expectancy.toFixed(4) }}</td>
            <td class="mono">{{ row.delta.toFixed(4) }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </details>
  <OptionsExpressionValidationInterpretationPanel
    :scope-rows="validationScopeRows"
    :drift-rows="validationDriftRows"
    :monotonicity-rows="validationMonotonicityRows"
    :regret-rows="validationRegretRows"
  />
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

.detailGridSpan2 {
  grid-column: span 2;
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
