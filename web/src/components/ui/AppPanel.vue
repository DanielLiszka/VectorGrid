<script setup lang="ts">
import { computed, useId } from 'vue'

defineOptions({ inheritAttrs: false })

const props = withDefaults(
  defineProps<{
    eyebrow?: string
    title?: string
    subtitle?: string
    variant?: 'default' | 'prominent' | 'subtle'
    headingId?: string
  }>(),
  {
    eyebrow: '',
    title: '',
    subtitle: '',
    variant: 'default',
    headingId: '',
  },
)

const generatedId = useId()
const resolvedHeadingId = computed(() => {
  if (!props.title) return undefined
  return props.headingId || `${generatedId}-heading`
})
</script>

<template>
  <section
    v-bind="$attrs"
    class="appPanel pane"
    :aria-labelledby="resolvedHeadingId"
    :class="{
      paneProminent: variant === 'prominent',
      subtlePane: variant === 'subtle',
    }"
  >
    <div v-if="eyebrow || title || subtitle || $slots.actions" class="panelSectionHeader">
      <div>
        <div v-if="eyebrow" class="panelEyebrow">{{ eyebrow }}</div>
        <h3 v-if="title" :id="resolvedHeadingId" class="panelPaneTitle">{{ title }}</h3>
        <div v-if="subtitle" class="panelMuted">{{ subtitle }}</div>
      </div>
      <slot name="actions" />
    </div>
    <slot />
  </section>
</template>

<style scoped>
.appPanel {
  min-width: 0;
}
</style>
