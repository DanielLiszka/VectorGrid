<script setup lang="ts">
import { computed, useAttrs } from 'vue'
import type { AppIcon } from '../utils/iconography'
import { isAssetIcon } from '../utils/iconography'

defineOptions({ inheritAttrs: false })

const props = withDefaults(
  defineProps<{
    icon?: AppIcon | null
    decorative?: boolean
    alt?: string
  }>(),
  {
    icon: null,
    decorative: true,
    alt: '',
  },
)

const attrs = useAttrs()
const assetIcon = computed(() => (isAssetIcon(props.icon) ? props.icon : null))
const resolvedAlt = computed(() => {
  if (props.decorative) return ''
  return String(props.alt || assetIcon.value?.label || '').trim()
})
</script>

<template>
  <img
    v-if="assetIcon"
    v-bind="attrs"
    :src="assetIcon.src"
    :alt="resolvedAlt"
    :aria-hidden="decorative ? 'true' : undefined"
    draggable="false"
    decoding="async"
  />
  <component
    :is="icon"
    v-else-if="icon"
    v-bind="attrs"
    :aria-hidden="decorative ? 'true' : undefined"
    :aria-label="!decorative && resolvedAlt ? resolvedAlt : undefined"
    :role="!decorative && resolvedAlt ? 'img' : undefined"
  />
</template>
