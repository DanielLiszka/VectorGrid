<script setup lang="ts">
import type { TableViewPresetOption } from './tableViewPresets'

withDefaults(
  defineProps<{
    label?: string
    presets: readonly TableViewPresetOption[]
    activePresetId?: string
    qaPrefix?: string
  }>(),
  {
    label: 'View',
    activePresetId: '',
    qaPrefix: '',
  },
)

const emit = defineEmits<{
  apply: [presetId: string]
}>()
</script>

<template>
  <div class="appTableViewPresetControl" role="group" :aria-label="label">
    <span class="presetLabel">{{ label }}</span>
    <div class="presetSegments" role="presentation">
      <button
        v-for="preset in presets"
        :key="preset.id"
        type="button"
        class="presetSegment"
        :class="{ active: preset.id === activePresetId }"
        :aria-pressed="preset.id === activePresetId ? 'true' : 'false'"
        :title="preset.summary || preset.label"
        :data-qa="qaPrefix ? `${qaPrefix}.${preset.id}` : undefined"
        @click="emit('apply', preset.id)"
      >
        {{ preset.label }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.appTableViewPresetControl {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 0.45rem;
  color: var(--muted);
  font-size: 0.74rem;
}

.presetLabel {
  white-space: nowrap;
}

.presetSegments {
  min-width: 0;
  display: inline-flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.18rem;
  padding: 0.12rem;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-sunken) 70%, transparent);
}

.presetSegment {
  min-height: 1.55rem;
  padding: 0 0.48rem;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--muted);
  font: inherit;
}

.presetSegment:hover:not(:disabled),
.presetSegment:focus-visible {
  color: var(--text);
  background: color-mix(in oklab, var(--panel-focus-edge) 14%, transparent);
}

.presetSegment.active {
  color: var(--text);
  background: color-mix(in oklab, var(--panel-focus-edge) 20%, var(--panel));
  box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--panel-focus-edge) 42%, transparent);
}

@media (max-width: 760px) {
  .appTableViewPresetControl {
    align-items: flex-start;
    flex-direction: column;
  }

  .presetSegments {
    width: 100%;
  }

  .presetSegment {
    flex: 1 1 auto;
  }
}
</style>
