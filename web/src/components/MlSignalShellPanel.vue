<script setup lang="ts">
import { useShellMount } from '../composables/useShellMount'
import MlSignalWorkspacePanel from './MlSignalWorkspacePanel.vue'

const { mountReady } = useShellMount('#optVueShellMount', {
  paneSelector: '[data-vue-panel="opt"]',
  paneDatasetKey: 'vueMlSignal',
})
</script>

<template>
  <Teleport v-if="mountReady" to="#optVueShellMount">
    <MlSignalWorkspacePanel />
  </Teleport>
</template>

<style>
[data-vue-panel="opt"][data-vue-ml-signal="1"] > :not(#optVueShellMount) {
  display: none !important;
}
</style>
