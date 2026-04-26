<script setup lang="ts">
import {
  TABLE_DENSITY_OPTIONS,
  normalizeTableDensity,
  type TableDensity,
} from '../../composables/usePersistedTableDensity'

const props = withDefaults(
  defineProps<{
    density: TableDensity
    label?: string
    qaPrefix?: string
  }>(),
  {
    label: 'Density',
    qaPrefix: '',
  },
)

const emit = defineEmits<{
  'update:density': [value: TableDensity]
}>()

function selectDensity(value: string): void {
  emit('update:density', normalizeTableDensity(value, props.density))
}
</script>

<template>
  <div class="appTableDensityControl" role="group" :aria-label="label">
    <span class="densityLabel">{{ label }}</span>
    <div class="densitySegments" role="presentation">
      <button
        v-for="option in TABLE_DENSITY_OPTIONS"
        :key="option"
        type="button"
        class="densitySegment"
        :class="{ active: option === density }"
        :aria-pressed="option === density"
        :data-qa="qaPrefix ? `${qaPrefix}.${option}` : undefined"
        @click="selectDensity(option)"
      >
        {{ option }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.appTableDensityControl {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 0.45rem;
  color: var(--muted);
  font-size: 0.74rem;
}

.densityLabel {
  white-space: nowrap;
}

.densitySegments {
  display: inline-flex;
  align-items: center;
  padding: 0.12rem;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-sunken) 70%, transparent);
}

.densitySegment {
  min-height: 1.55rem;
  padding: 0 0.48rem;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--muted);
  font: inherit;
  text-transform: capitalize;
}

.densitySegment:hover:not(:disabled),
.densitySegment:focus-visible {
  color: var(--text);
  background: color-mix(in oklab, var(--panel-focus-edge) 14%, transparent);
}

.densitySegment.active {
  color: var(--text);
  background: color-mix(in oklab, var(--panel-focus-edge) 20%, var(--panel));
  box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--panel-focus-edge) 42%, transparent);
}

@media (max-width: 760px) {
  .appTableDensityControl {
    align-items: flex-start;
    flex-direction: column;
  }

  .densitySegments {
    width: 100%;
  }

  .densitySegment {
    flex: 1 1 0;
  }
}
</style>
