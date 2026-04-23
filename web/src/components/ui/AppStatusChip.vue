<script setup lang="ts">
defineOptions({ inheritAttrs: false })

withDefaults(
  defineProps<{
    tone?: 'neutral' | 'accent' | 'success' | 'warning' | 'danger' | 'muted'
    label?: string
  }>(),
  {
    tone: 'neutral',
    label: '',
  },
)
</script>

<template>
  <span
    v-bind="$attrs"
    class="appStatusChip"
    :class="`appStatusChip-${tone}`"
    :aria-label="label || undefined"
  >
    <slot>{{ label }}</slot>
  </span>
</template>

<style scoped>
.appStatusChip {
  display: inline-flex;
  align-items: center;
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: color-mix(in oklab, var(--surface-working) 92%, var(--panel));
  color: var(--muted-strong);
  padding: var(--density-chip-y) var(--density-chip-x);
  font-size: 0.74rem;
  line-height: 1.2;
}

.appStatusChip-accent {
  border-color: color-mix(in oklab, var(--accent) 38%, var(--border));
  background: color-mix(in oklab, var(--selection-fill) 86%, var(--surface-working));
  color: var(--selection-text);
}

.appStatusChip-success {
  border-color: color-mix(in oklab, var(--ok) 36%, var(--border));
  background: var(--surface-success);
  color: var(--ok-ink);
}

.appStatusChip-warning {
  border-color: color-mix(in oklab, var(--warning) 36%, var(--border));
  background: var(--surface-warning);
  color: var(--warning-ink);
}

.appStatusChip-danger {
  border-color: color-mix(in oklab, var(--danger) 36%, var(--border));
  background: var(--surface-danger);
  color: var(--danger-ink);
}

.appStatusChip-muted {
  background: color-mix(in oklab, var(--surface2) 88%, var(--panel));
  color: var(--muted);
}
</style>
