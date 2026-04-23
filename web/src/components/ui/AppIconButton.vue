<script setup lang="ts">
import { computed, useAttrs } from 'vue'
import AppVisualIcon from '../AppVisualIcon.vue'
import type { AppIcon } from '../../utils/iconography'

defineOptions({ inheritAttrs: false })

const props = withDefaults(
  defineProps<{
    icon: AppIcon
    label: string
    type?: 'button' | 'submit' | 'reset'
    variant?: 'secondary' | 'subtle' | 'danger'
    size?: 'sm' | 'md'
    disabled?: boolean
    disabledReason?: string
    title?: string
  }>(),
  {
    type: 'button',
    variant: 'secondary',
    size: 'md',
    disabled: false,
    disabledReason: '',
    title: '',
  },
)

const attrs = useAttrs()
const resolvedLabel = computed(() => String(props.label || '').trim())
const resolvedTitle = computed(() => {
  if (props.disabled && String(props.disabledReason || '').trim()) return String(props.disabledReason || '').trim()
  return String(props.title || '').trim() || resolvedLabel.value
})
</script>

<template>
  <button
    v-bind="attrs"
    :type="type"
    class="appIconButton"
    :class="[`appIconButton-${variant}`, `appIconButton-${size}`, { 'danger-btn': variant === 'danger' }]"
    :disabled="disabled"
    :aria-label="resolvedLabel"
    :title="resolvedTitle || undefined"
  >
    <AppVisualIcon :icon="icon" class="appIconButtonIcon" />
  </button>
</template>

<style scoped>
.appIconButton {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: 0 0 auto;
  border-radius: 10px;
}

.appIconButton-sm {
  inline-size: 2rem;
  min-height: 2rem;
  padding: 0;
}

.appIconButton-md {
  inline-size: 2.25rem;
  min-height: 2.25rem;
  padding: 0;
}

.appIconButton-subtle {
  background: color-mix(in oklab, var(--surface2) 84%, transparent);
  color: var(--muted-strong);
}

.appIconButtonIcon {
  width: 0.95rem;
  height: 0.95rem;
}
</style>
