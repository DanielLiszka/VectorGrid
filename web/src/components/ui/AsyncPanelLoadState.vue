<script setup lang="ts">
import AppButton from './AppButton.vue'

defineProps<{
  label: string
  loading: boolean
}>()

defineEmits<{
  retry: []
}>()
</script>

<template>
  <section
    class="asyncPanelLoadState"
    :class="{ asyncPanelLoadStateError: !loading }"
    :role="loading ? 'status' : 'alert'"
    :aria-live="loading ? 'polite' : 'assertive'"
    aria-atomic="true"
    :aria-busy="loading ? 'true' : undefined"
  >
    <div class="asyncPanelLoadStateCopy">
      <strong class="asyncPanelLoadStateTitle">
        {{ loading ? `Loading ${label}…` : `${label} did not load` }}
      </strong>
      <p v-if="!loading" class="asyncPanelLoadStateBody">
        The frontend module could not be opened. Your current workflow is unchanged.
      </p>
    </div>
    <AppButton
      v-if="!loading"
      class="asyncPanelLoadStateRetry"
      size="sm"
      variant="secondary"
      :aria-label="`Retry loading ${label}`"
      @click="$emit('retry')"
    >
      Retry
    </AppButton>
  </section>
</template>

<style scoped>
.asyncPanelLoadState {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  margin: 0.65rem;
  border: 1px solid color-mix(in oklab, var(--accent) 30%, var(--border));
  border-radius: 10px;
  background: color-mix(in oklab, var(--surface2) 94%, var(--accent) 6%);
  color: var(--text);
  padding: 0.65rem 0.72rem;
}

.asyncPanelLoadStateError {
  border-color: color-mix(in oklab, var(--danger) 38%, var(--border));
  background: color-mix(in oklab, var(--surface2) 92%, var(--danger) 8%);
}

.asyncPanelLoadStateCopy {
  min-width: 0;
}

.asyncPanelLoadStateTitle {
  display: block;
  font-size: 0.82rem;
  line-height: 1.35;
}

.asyncPanelLoadStateBody {
  margin: 0.2rem 0 0;
  color: var(--muted);
  font-size: 0.76rem;
  line-height: 1.4;
}

.asyncPanelLoadStateRetry {
  flex: 0 0 auto;
}
</style>
