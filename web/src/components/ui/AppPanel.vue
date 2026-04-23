<script setup lang="ts">
defineOptions({ inheritAttrs: false })

withDefaults(
  defineProps<{
    eyebrow?: string
    title?: string
    subtitle?: string
    variant?: 'default' | 'prominent' | 'subtle'
  }>(),
  {
    eyebrow: '',
    title: '',
    subtitle: '',
    variant: 'default',
  },
)
</script>

<template>
  <section
    v-bind="$attrs"
    class="appPanel pane"
    :class="{
      paneProminent: variant === 'prominent',
      subtlePane: variant === 'subtle',
    }"
  >
    <div v-if="eyebrow || title || subtitle || $slots.actions" class="panelSectionHeader">
      <div>
        <div v-if="eyebrow" class="panelEyebrow">{{ eyebrow }}</div>
        <h3 v-if="title" class="panelPaneTitle">{{ title }}</h3>
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
