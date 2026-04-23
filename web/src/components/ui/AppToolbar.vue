<script setup lang="ts">
withDefaults(
  defineProps<{
    ariaLabel?: string
    align?: 'start' | 'end' | 'between'
  }>(),
  {
    ariaLabel: '',
    align: 'start',
  },
)
</script>

<template>
  <div
    class="appToolbar actions"
    :class="`appToolbar-${align}`"
    role="toolbar"
    :aria-label="ariaLabel || undefined"
  >
    <slot />
  </div>
</template>

<style scoped>
.appToolbar {
  margin-top: 0;
  align-items: center;
}

.appToolbar-end {
  justify-content: flex-end;
}

.appToolbar-between {
  justify-content: space-between;
}
</style>
