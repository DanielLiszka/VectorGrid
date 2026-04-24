<script setup lang="ts">
defineProps<{
  label: string
}>()
</script>

<template>
  <div class="visualizationToolbar" role="toolbar" :aria-label="label">
    <slot />
    <div v-if="$slots.actions" class="toolbarActions">
      <slot name="actions" />
    </div>
  </div>
</template>

<style scoped>
.visualizationToolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: end;
  justify-content: space-between;
  gap: 0.65rem;
}

.toolbarActions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.45rem;
}

.toolbarActions :deep(button) {
  padding: 0.42rem 0.6rem;
  font-size: 0.76rem;
}
</style>
