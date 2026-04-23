<script setup lang="ts">
defineOptions({ inheritAttrs: false })

withDefaults(
  defineProps<{
    title: string
    body?: string
    hint?: string
    blockedReason?: string
  }>(),
  {
    body: '',
    hint: '',
    blockedReason: '',
  },
)
</script>

<template>
  <section v-bind="$attrs" class="appEmptyState emptyState">
    <div class="emptyTitle">{{ title }}</div>
    <div v-if="body" class="emptyBody">{{ body }}</div>
    <div v-if="blockedReason" class="emptyBlockedReason" role="status">
      {{ blockedReason }}
    </div>
    <div v-if="$slots.actions" class="emptyActions">
      <slot name="actions" />
    </div>
    <div v-if="hint" class="panelMuted appEmptyHint">{{ hint }}</div>
  </section>
</template>

<style scoped>
.appEmptyState {
  min-height: 0;
}

.appEmptyHint {
  margin-top: 0.55rem;
}

.emptyBlockedReason {
  margin-top: 0.7rem;
  border: 1px solid color-mix(in oklab, var(--warning) 34%, var(--border));
  border-radius: 10px;
  background: var(--surface-warning);
  color: var(--warning-ink);
  padding: 0.54rem 0.64rem;
  font-size: 0.82rem;
  line-height: 1.35;
}
</style>
