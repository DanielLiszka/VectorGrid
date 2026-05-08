<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'

withDefaults(
  defineProps<{
    chartId: string
    metaId: string
    title: string
    metaText: string
    advisory?: string
    frameAriaLabel?: string
    hostAriaLabel?: string
    describedBy?: string
  }>(),
  {
    advisory: '',
    frameAriaLabel: '',
    hostAriaLabel: 'Price chart',
    describedBy: '',
  },
)

const emit = defineEmits<{
  'host-ready': [element: HTMLElement | null]
}>()

const chartHost = ref<HTMLElement | null>(null)

onMounted(() => {
  emit('host-ready', chartHost.value)
})

onBeforeUnmount(() => {
  emit('host-ready', null)
})
</script>

<template>
  <div class="priceChartPanel">
    <div class="panelMuted chartTitle">{{ title }}</div>
    <div v-if="advisory" class="panelAdvisory">{{ advisory }}</div>
    <div
      class="tvChartFrame chartSurface"
      role="group"
      :aria-label="frameAriaLabel || hostAriaLabel"
      :aria-describedby="describedBy || undefined"
    >
      <div
        :id="chartId"
        ref="chartHost"
        class="tvChartHost"
        role="img"
        :aria-label="hostAriaLabel"
      ></div>
    </div>
    <div :id="metaId" class="panelStatus mono panelNumeric chartMeta" role="status" aria-live="polite">
      {{ metaText }}
    </div>
  </div>
</template>

<style scoped>
.priceChartPanel {
  display: grid;
  gap: 0.35rem;
}

.chartTitle {
  margin-bottom: 0;
}

.chartMeta {
  margin-top: 0;
}

.chartSurface:focus-visible {
  outline: 2px solid var(--focus-border);
  outline-offset: 2px;
  box-shadow: var(--focus-ring);
}

.mono {
  font-family: var(--mono);
}
</style>
