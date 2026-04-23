<script setup lang="ts">
import { ChevronDown } from 'lucide-vue-next'

export interface RunContextItem {
  key: string
  value: string
  current: boolean
  defaultVisible: boolean
}

withDefaults(
  defineProps<{
    items: RunContextItem[]
    secondaryItems: RunContextItem[]
    secondarySummary: string
  }>(),
  {
    items: () => [],
    secondaryItems: () => [],
    secondarySummary: '',
  },
)
</script>

<template>
  <section class="runContextBar contextStrip surfaceSunken" aria-label="Run context">
    <article
      v-for="item in items"
      :key="`ctx_${item.key}`"
      class="contextCard"
      :class="{ current: item.current }"
    >
      <div class="contextKey">{{ item.key }}</div>
      <div class="contextValue mono">{{ item.value }}</div>
    </article>

    <details v-if="secondaryItems.length > 0" class="contextOverflow">
      <summary class="contextOverflowSummary">
        <span class="contextOverflowBody">
          <span class="contextKey">More</span>
          <span class="contextValue mono">{{ secondarySummary }}</span>
        </span>
        <ChevronDown class="contextOverflowIcon" :size="14" aria-hidden="true" />
      </summary>
      <div class="contextOverflowPanel" role="group" aria-label="Additional run context">
        <article
          v-for="item in secondaryItems"
          :key="`ctx_more_${item.key}`"
          class="contextCard"
        >
          <div class="contextKey">{{ item.key }}</div>
          <div class="contextValue mono">{{ item.value }}</div>
        </article>
      </div>
    </details>
  </section>
</template>
