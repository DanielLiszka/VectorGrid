<script setup lang="ts">
export interface AppMetricItem {
  key: string
  label: string
  value: string
  tone?: 'neutral' | 'success' | 'warning' | 'danger' | 'accent'
  title?: string
}

withDefaults(
  defineProps<{
    items: AppMetricItem[]
    ariaLabel?: string
  }>(),
  {
    items: () => [],
    ariaLabel: '',
  },
)
</script>

<template>
  <dl class="appMetricStrip" :aria-label="ariaLabel || undefined">
    <div
      v-for="item in items"
      :key="item.key"
      class="appMetricItem"
      :class="item.tone ? `appMetricItem-${item.tone}` : undefined"
    >
      <dt class="appMetricLabel">{{ item.label }}</dt>
      <dd class="appMetricValue mono" :title="item.title || item.value">{{ item.value || '—' }}</dd>
    </div>
  </dl>
</template>

<style scoped>
.appMetricStrip {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(8.5rem, 1fr));
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
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.78rem;
  color: var(--text);
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
