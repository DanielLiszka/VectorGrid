<script setup lang="ts">
import type { MetricCellVisual } from '../../visualizations/metricCellVisualModel'

defineProps<{
  text: string
  visual?: MetricCellVisual | null
  align?: 'left' | 'right'
}>()
</script>

<template>
  <div
    class="appMetricCell"
    :class="[visual ? `tone-${visual.tone}` : 'tone-none', align === 'left' ? 'align-left' : 'align-right']"
    :data-visual-kind="visual?.kind || 'none'"
    :style="visual?.kind === 'bar' ? { '--metric-fill': `${visual.fillPct}%` } : undefined"
    :aria-label="visual?.label || undefined"
  >
    <span v-if="visual?.kind === 'bar'" class="metricCellFill" aria-hidden="true"></span>
    <span class="metricCellText">{{ text }}</span>
    <slot />
  </div>
</template>

<style scoped>
.appMetricCell {
  --metric-fill: 0%;
  position: relative;
  isolation: isolate;
  min-width: 0;
  min-height: 1.35rem;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.45rem;
}

.appMetricCell.align-left {
  justify-content: flex-start;
}

.metricCellFill {
  position: absolute;
  z-index: 0;
  inset: 0.08rem 0.02rem 0.08rem auto;
  width: var(--metric-fill);
  min-width: 2px;
  border-radius: 4px;
  opacity: 0.42;
}

.metricCellText {
  position: relative;
  z-index: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}

.appMetricCell > :not(.metricCellFill) {
  position: relative;
  z-index: 1;
}

.appMetricCell[data-visual-kind="badge"] {
  justify-content: flex-start;
}

.appMetricCell[data-visual-kind="badge"] .metricCellText {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 1.25rem;
  padding: 0.08rem 0.42rem;
  border-radius: 999px;
  border: 1px solid color-mix(in oklab, var(--border) 84%, transparent);
  background: color-mix(in oklab, var(--surface-inline) 88%, var(--surface-working));
}

.tone-good .metricCellFill,
.tone-good[data-visual-kind="badge"] .metricCellText {
  border-color: color-mix(in oklab, #16a34a 42%, var(--border));
  background: color-mix(in oklab, var(--surface-success) 84%, transparent);
  color: color-mix(in oklab, #15803d 76%, var(--muted-strong));
}

.tone-risk .metricCellFill,
.tone-risk[data-visual-kind="badge"] .metricCellText {
  border-color: color-mix(in oklab, var(--danger) 42%, var(--border));
  background: color-mix(in oklab, var(--surface-danger) 84%, transparent);
  color: color-mix(in oklab, var(--danger-ink) 78%, var(--muted-strong));
}

.tone-warn .metricCellFill,
.tone-warn[data-visual-kind="badge"] .metricCellText {
  border-color: color-mix(in oklab, var(--warning) 42%, var(--border));
  background: color-mix(in oklab, var(--surface-warning) 84%, transparent);
  color: color-mix(in oklab, var(--warning-ink) 78%, var(--muted-strong));
}

.tone-neutral .metricCellFill,
.tone-neutral[data-visual-kind="badge"] .metricCellText {
  border-color: color-mix(in oklab, var(--muted-soft) 34%, var(--border));
  background: color-mix(in oklab, var(--surface-inline) 82%, transparent);
  color: var(--muted-strong);
}

.tone-accent .metricCellFill,
.tone-accent[data-visual-kind="badge"] .metricCellText {
  border-color: color-mix(in oklab, var(--accent) 38%, var(--border));
  background: color-mix(in oklab, var(--accent-soft) 82%, transparent);
  color: color-mix(in oklab, var(--accent-strong) 72%, var(--muted-strong));
}
</style>
