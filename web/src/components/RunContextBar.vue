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

<style scoped>
.contextCard {
  min-width: 0;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-working) 96%, var(--panel));
  padding: 0.22rem 0.42rem;
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 0.02rem 0.34rem;
  align-items: baseline;
  transition:
    background-color var(--motion-fast) var(--ease-standard),
    border-color var(--motion-fast) var(--ease-standard),
    box-shadow var(--motion-fast) var(--ease-standard),
    transform var(--motion-fast) var(--ease-standard);
}

.contextCard.current {
  border-color: color-mix(in oklab, var(--panel-focus-edge) 74%, var(--border));
  background: linear-gradient(
    145deg,
    color-mix(in oklab, var(--surface-focus) 90%, var(--panel-footer)),
    color-mix(in oklab, var(--panel-footer) 92%, var(--surface-working))
  );
  box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--panel-focus-edge) 16%, transparent);
}

.contextKey {
  font-size: 0.57rem;
  letter-spacing: 0.07em;
  color: var(--muted);
  text-transform: uppercase;
}

.contextValue {
  min-width: 0;
  font-size: 0.72rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.contextOverflow {
  position: relative;
  min-width: 0;
}

.contextOverflowSummary {
  min-height: 100%;
  list-style: none;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-working) 94%, var(--panel));
  padding: 0.22rem 0.42rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  cursor: pointer;
}

.contextOverflowSummary::-webkit-details-marker {
  display: none;
}

.contextOverflowBody {
  min-width: 0;
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 0.02rem 0.34rem;
  align-items: baseline;
}

.contextOverflowIcon {
  flex: 0 0 auto;
  color: var(--muted);
  transition: transform var(--motion-fast) var(--ease-standard);
}

.contextOverflow[open] .contextOverflowSummary {
  border-color: color-mix(in oklab, var(--panel-focus-edge) 70%, var(--border));
  background: color-mix(in oklab, var(--surface-focus) 88%, var(--panel));
}

.contextOverflow[open] .contextOverflowIcon {
  transform: rotate(180deg);
}

.contextOverflowPanel {
  position: absolute;
  top: calc(100% + 0.35rem);
  right: 0;
  z-index: 6;
  min-width: min(22rem, calc(100vw - 2rem));
  display: grid;
  gap: 0.32rem;
  padding: 0.35rem;
  border: 1px solid color-mix(in oklab, var(--border) 90%, transparent);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-overlay) 94%, var(--panel));
  box-shadow: var(--shadow-md);
}

@media (max-width: 760px) {
  .contextCard,
  .contextOverflow {
    flex: 0 0 min(13rem, 76vw);
  }

  .contextOverflowPanel {
    left: 0;
    right: auto;
    min-width: min(100%, 22rem);
  }
}
</style>
