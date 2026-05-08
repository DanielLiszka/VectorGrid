<script setup lang="ts">
defineProps<{
  title: string
  body: string
  status?: string
  actionLabel?: string
}>()

defineEmits<{
  action: []
}>()
</script>

<template>
  <div class="chartEmptyState">
    <div class="chartEmptyMessage" role="status" aria-live="polite">
      <div class="chartEmptyTitle">{{ title }}</div>
      <div class="chartEmptyBody">{{ body }}</div>
      <div v-if="status" class="chartEmptyStatus mono panelNumeric">{{ status }}</div>
    </div>
    <button v-if="actionLabel" type="button" @click="$emit('action')">{{ actionLabel }}</button>
  </div>
</template>

<style scoped>
.chartEmptyState {
  display: grid;
  gap: 0.45rem;
  padding: 0.75rem;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: color-mix(in srgb, var(--panel-footer) 86%, transparent);
}

.chartEmptyMessage {
  display: grid;
  gap: 0.45rem;
}

.chartEmptyTitle {
  font-weight: 700;
  color: var(--text);
}

.chartEmptyBody,
.chartEmptyStatus {
  color: var(--muted);
  font-size: 0.82rem;
}

.chartEmptyState button {
  justify-self: start;
  padding: 0.42rem 0.6rem;
  font-size: 0.76rem;
}

.mono {
  font-family: var(--mono);
}
</style>
