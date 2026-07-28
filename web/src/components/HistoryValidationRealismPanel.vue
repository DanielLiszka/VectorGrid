<script setup lang="ts">
import { computed } from "vue";
import type { ValidationEvidenceSummary } from "../utils/validationEvidenceSummary";

const props = defineProps<{
  summary: ValidationEvidenceSummary;
}>();

const promotionEligible = computed(
  () => props.summary.promotionEligibility.eligible,
);

const promotionLabel = computed(() =>
  promotionEligible.value
    ? "Eligible for promotion review"
    : "Promotion blocked",
);

const usePostureLabel = computed(() =>
  promotionEligible.value ? "Review candidate" : "Research only",
);

function displayEvidenceValue(value: string): string {
  const next = String(value || "").trim();
  return next ? next.replace(/_/g, " ") : "unknown";
}
</script>

<template>
  <section
    class="historyRealismPanel surfaceSunken"
    aria-label="Saved validation data realism and promotion eligibility"
    data-qa-panel="history.validationRealism"
    :data-qa-promotion-state="summary.promotionEligibility.state"
    :data-qa-realism-state="summary.dataRealism.status"
  >
    <div class="historyRealismHeader">
      <div>
        <div class="panelEyebrow">Saved validation evidence</div>
        <h3 class="panelPaneTitle">Data realism &amp; promotion</h3>
      </div>
      <div class="historyRealismBadges" aria-label="Artifact use posture">
        <span
          class="historyRealismBadge"
          :class="{ good: promotionEligible, warn: !promotionEligible }"
          data-qa-field="history.validationRealism.usePosture"
        >
          {{ usePostureLabel }}
        </span>
        <span
          class="historyRealismBadge"
          :class="{ good: promotionEligible, bad: !promotionEligible }"
          data-qa-field="history.validationRealism.promotion"
        >
          {{ promotionLabel }}
        </span>
      </div>
    </div>

    <p class="historyRealismLead">
      This view reports only evidence persisted with the selected artifact.
      Missing and legacy fields remain unknown and block promotion review.
    </p>

    <div class="historyRealismSummaryGrid">
      <div>
        <span class="panelMetricKey">Data realism</span>
        <span
          class="panelMetricValue mono panelNumeric"
          data-qa-field="history.validationRealism.status"
        >
          {{ summary.dataRealism.status }}
        </span>
      </div>
      <div>
        <span class="panelMetricKey">Promotion state</span>
        <span
          class="panelMetricValue mono panelNumeric"
          data-qa-field="history.validationRealism.state"
        >
          {{ displayEvidenceValue(summary.promotionEligibility.state) }}
        </span>
      </div>
    </div>

    <div class="historyRealismEvidenceGrid">
      <div
        v-for="item in summary.dataRealism.items"
        :key="item.id"
        class="historyRealismEvidenceItem"
        :class="{ unknown: item.blocking }"
        :data-qa-field="`history.validationRealism.${item.id}`"
        :data-qa-state="item.blocking ? 'unknown' : 'reported'"
      >
        <span class="historyRealismEvidenceLabel">{{ item.label }}</span>
        <span class="historyRealismEvidenceValue mono">
          {{ displayEvidenceValue(item.value) }}
        </span>
        <span class="historyRealismEvidenceDetail">{{ item.detail }}</span>
      </div>
    </div>

    <div class="historyRealismPolicy">
      <p>{{ summary.dataRealism.summaryLine }}</p>
      <p>{{ summary.promotionEligibility.summaryLine }}</p>
      <p>
        Fee or slippage stress does not establish liquidity/capacity or
        market-impact evidence.
      </p>
    </div>
  </section>
</template>

<style scoped>
.historyRealismPanel {
  margin-top: 0.75rem;
  border: 1px solid color-mix(in oklab, var(--border) 86%, var(--accent));
  border-radius: 12px;
  padding: 0.75rem 0.8rem;
}

.historyRealismHeader {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.75rem;
}

.historyRealismBadges {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 0.4rem;
}

.historyRealismBadge {
  border: 1px solid var(--border);
  border-radius: 999px;
  padding: 0.22rem 0.55rem;
  background: var(--panel);
  font-size: 0.7rem;
  font-weight: 650;
  line-height: 1.2;
}

.historyRealismBadge.good {
  border-color: color-mix(in oklab, var(--accent) 66%, var(--border));
  background: color-mix(in oklab, var(--accent-soft) 62%, var(--surface));
}

.historyRealismBadge.warn {
  border-color: color-mix(in oklab, var(--warning) 62%, var(--border));
  background: color-mix(in oklab, var(--warning-soft) 54%, var(--surface));
}

.historyRealismBadge.bad {
  border-color: color-mix(in oklab, var(--danger) 62%, var(--border));
  background: color-mix(in oklab, var(--danger-soft) 50%, var(--surface));
}

.historyRealismLead {
  margin: 0.55rem 0 0;
  color: var(--muted);
  font-size: 0.78rem;
  line-height: 1.45;
}

.historyRealismSummaryGrid {
  margin-top: 0.6rem;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.45rem 0.75rem;
}

.historyRealismSummaryGrid > div {
  display: grid;
  gap: 0.18rem;
}

.historyRealismEvidenceGrid {
  margin-top: 0.65rem;
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.45rem;
}

.historyRealismEvidenceItem {
  min-width: 0;
  display: grid;
  gap: 0.16rem;
  border: 1px solid color-mix(in oklab, var(--border) 90%, transparent);
  border-radius: 10px;
  padding: 0.55rem;
  background: color-mix(in oklab, var(--panel) 92%, var(--surface-working));
}

.historyRealismEvidenceItem.unknown {
  border-color: color-mix(in oklab, var(--warning) 56%, var(--border));
  background: color-mix(in oklab, var(--warning-soft) 28%, var(--surface));
}

.historyRealismEvidenceLabel {
  color: var(--muted);
  font-size: 0.7rem;
  font-weight: 650;
  letter-spacing: 0.02em;
  text-transform: uppercase;
}

.historyRealismEvidenceValue {
  overflow-wrap: anywhere;
  font-size: 0.82rem;
}

.historyRealismEvidenceDetail {
  color: var(--muted);
  font-size: 0.7rem;
}

.historyRealismPolicy {
  margin-top: 0.65rem;
  border-top: 1px solid color-mix(in oklab, var(--border) 82%, transparent);
  padding-top: 0.55rem;
  color: var(--muted);
  font-size: 0.74rem;
  line-height: 1.45;
}

.historyRealismPolicy p {
  margin: 0;
}

.historyRealismPolicy p + p {
  margin-top: 0.3rem;
}

@media (max-width: 980px) {
  .historyRealismEvidenceGrid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 620px) {
  .historyRealismHeader {
    display: grid;
  }

  .historyRealismBadges {
    justify-content: flex-start;
  }

  .historyRealismSummaryGrid,
  .historyRealismEvidenceGrid {
    grid-template-columns: 1fr;
  }
}
</style>
