<script setup lang="ts">
import { computed, useAttrs } from 'vue'
import AppVisualIcon from '../AppVisualIcon.vue'
import type { AppIcon } from '../../utils/iconography'

defineOptions({ inheritAttrs: false })

const props = withDefaults(
  defineProps<{
    type?: 'button' | 'submit' | 'reset'
    variant?: 'primary' | 'secondary' | 'subtle' | 'danger'
    size?: 'sm' | 'md'
    disabled?: boolean
    disabledReason?: string
    loading?: boolean
    loadingLabel?: string
    icon?: AppIcon | null
    iconOnly?: boolean
    ariaLabel?: string
    title?: string
  }>(),
  {
    type: 'button',
    variant: 'secondary',
    size: 'md',
    disabled: false,
    disabledReason: '',
    loading: false,
    loadingLabel: '',
    icon: null,
    iconOnly: false,
    ariaLabel: '',
    title: '',
  },
)

const attrs = useAttrs()
const buttonDisabled = computed(() => props.disabled || props.loading)
const resolvedTitle = computed(() => {
  const disabledReason = String(props.disabledReason || '').trim()
  if (buttonDisabled.value && disabledReason) return disabledReason
  return String(props.title || '').trim()
})
const resolvedAriaLabel = computed(() => {
  const label = String(props.ariaLabel || '').trim()
  if (label) return label
  const title = String(props.title || '').trim()
  if (props.iconOnly && title) return title
  return props.iconOnly ? String(props.disabledReason || '').trim() || undefined : undefined
})
const resolvedLoadingLabel = computed(() => String(props.loadingLabel || '').trim() || 'Loading')
</script>

<template>
  <button
    v-bind="attrs"
    :type="type"
    class="appButton"
    :class="[
      `appButton-${variant}`,
      `appButton-${size}`,
      { primary: variant === 'primary', dangerBtn: variant === 'danger', 'danger-btn': variant === 'danger', iconOnly, loading },
    ]"
    :disabled="buttonDisabled"
    :title="resolvedTitle || undefined"
    :aria-label="resolvedAriaLabel"
    :aria-busy="loading ? 'true' : undefined"
  >
    <span v-if="loading" class="appButtonSpinner" aria-hidden="true"></span>
    <AppVisualIcon v-if="icon" :icon="icon" class="appButtonIcon" />
    <span v-if="!iconOnly" class="appButtonLabel">
      <template v-if="loading">{{ resolvedLoadingLabel }}</template>
      <slot v-else />
    </span>
  </button>
</template>

<style scoped>
.appButton {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.42rem;
  min-width: 0;
}

.appButton-sm {
  min-height: 2rem;
  padding: 0.38rem 0.58rem;
  font-size: 0.74rem;
}

.appButton-md {
  min-height: 2.25rem;
}

.appButton-subtle {
  background: color-mix(in oklab, var(--surface2) 84%, transparent);
  color: var(--muted-strong);
}

.appButtonIcon {
  width: 0.95rem;
  height: 0.95rem;
  flex: 0 0 auto;
}

.appButtonLabel {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}

.iconOnly {
  inline-size: 2.25rem;
  padding-inline: 0;
}

.appButtonSpinner {
  inline-size: 0.86rem;
  block-size: 0.86rem;
  border-radius: 999px;
  border: 2px solid currentColor;
  border-right-color: transparent;
  animation: appButtonSpin 760ms linear infinite;
}

@keyframes appButtonSpin {
  to {
    transform: rotate(360deg);
  }
}
</style>
