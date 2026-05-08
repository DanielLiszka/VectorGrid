<script setup lang="ts">
import { computed, type CSSProperties } from 'vue'

export interface AppMetricItem {
  key: string
  label: string
  value: string
  tone?: 'neutral' | 'success' | 'warning' | 'danger' | 'accent'
  trend?: string
  detail?: string
  title?: string
}

const props = withDefaults(
  defineProps<{
    items: AppMetricItem[]
    ariaLabel?: string
    minItemWidth?: string
    size?: 'default' | 'compact'
    surface?: 'default' | 'sunken'
  }>(),
  {
    items: () => [],
    ariaLabel: '',
    minItemWidth: '8.5rem',
    size: 'default',
    surface: 'default',
  },
)

const stripStyle = computed<CSSProperties>(() => ({
  '--app-metric-min-width': props.minItemWidth,
}))
</script>

<template>
  <dl
    class="appMetricStrip"
    :class="[`appMetricStrip-${size}`, `appMetricStrip-surface-${surface}`]"
    :style="stripStyle"
    :aria-label="ariaLabel || undefined"
  >
    <div
      v-for="item in items"
      :key="item.key"
      class="appMetricItem"
      :class="item.tone ? `appMetricItem-${item.tone}` : undefined"
    >
      <dt class="appMetricLabel">{{ item.label }}</dt>
      <dd class="appMetricValue mono" :title="item.title || item.value">
        <span class="appMetricValueRow">
          <span class="appMetricPrimaryValue">{{ item.value || '—' }}</span>
          <span v-if="item.trend" class="appMetricTrend">{{ item.trend }}</span>
        </span>
        <small v-if="item.detail" class="appMetricDetail" :title="item.detail">{{ item.detail }}</small>
      </dd>
    </div>
  </dl>
</template>

<style scoped>
.appMetricStrip {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(var(--app-metric-min-width, 8.5rem), 1fr));
  gap: 0.45rem;
  margin: 0;
}

.appMetricItem {
  min-width: 0;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 10px;
  background: color-mix(in oklab, var(--surface-working) 94%, var(--panel));
  padding: 0.45rem 0.55rem;
}

.appMetricStrip-compact {
  gap: 0.4rem;
}

.appMetricStrip-compact .appMetricItem {
  border-radius: 8px;
  padding: 0.4rem 0.48rem;
}

.appMetricStrip-surface-sunken .appMetricItem {
  background: color-mix(in oklab, var(--surface-sunken) 72%, transparent);
  border-color: color-mix(in oklab, var(--border) 78%, transparent);
}

.appMetricLabel {
  margin: 0;
  font-size: 0.66rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted);
}

.appMetricValue {
  margin: 0.12rem 0 0;
  min-width: 0;
  display: grid;
  gap: 0.16rem;
  overflow: hidden;
  font-size: 0.78rem;
  color: var(--text);
}

.appMetricValueRow {
  min-width: 0;
  display: flex;
  align-items: baseline;
  gap: 0.35rem;
}

.appMetricPrimaryValue,
.appMetricTrend {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.appMetricPrimaryValue {
  flex: 1 1 auto;
}

.appMetricTrend {
  flex: 0 1 auto;
  color: var(--muted);
  font-size: 0.72rem;
}

.appMetricDetail {
  display: block;
  color: var(--muted-strong);
  font-size: 0.72rem;
  line-height: 1.35;
  overflow-wrap: anywhere;
}

.appMetricItem-success {
  border-color: color-mix(in oklab, var(--ok) 30%, var(--border));
}

.appMetricItem-warning {
  border-color: color-mix(in oklab, var(--warning) 32%, var(--border));
}

.appMetricItem-danger {
  border-color: color-mix(in oklab, var(--danger) 32%, var(--border));
}

.appMetricItem-accent {
  border-color: color-mix(in oklab, var(--accent) 34%, var(--border));
}
</style>
