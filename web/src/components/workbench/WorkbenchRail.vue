<script setup lang="ts">
import { computed, nextTick, type Component } from 'vue'
import {
  BriefcaseBusiness,
  CircleAlert,
  CircleCheckBig,
  CircleDot,
  FolderOpenDot,
  Play,
  Settings2,
} from '@lucide/vue'
import type { WorkbenchNavigationItem } from '../../utils/workbenchNavigationModel'

const props = defineProps<{
  items: WorkbenchNavigationItem[]
}>()

const emit = defineEmits<{
  select: [id: string]
}>()

const iconMap: Record<string, Component> = {
  chart: CircleDot,
  library: FolderOpenDot,
  runs: CircleCheckBig,
  portfolio: BriefcaseBusiness,
  live: Play,
  settings: Settings2,
  diagnostics: CircleAlert,
}

const primaryItems = computed(() => props.items.filter((item) => !item.secondary))
const secondaryItems = computed(() => props.items.filter((item) => item.secondary))

function selectItem(item: WorkbenchNavigationItem): void {
  emit('select', item.id)
}

function itemIcon(item: WorkbenchNavigationItem): Component {
  return iconMap[item.icon] ?? CircleDot
}

function focusWorkspace(id: string): void {
  nextTick(() => {
    document.querySelector<HTMLElement>(`#workspaceBar [data-workspace="${id}"]`)?.focus()
  })
}

function onItemKeydown(event: KeyboardEvent, item: WorkbenchNavigationItem): void {
  const key = event.key
  if (!['ArrowDown', 'ArrowRight', 'ArrowUp', 'ArrowLeft', 'Home', 'End'].includes(key)) return
  event.preventDefault()
  const items = props.items
  const currentIndex = Math.max(0, items.findIndex((entry) => entry.id === item.id))
  let nextIndex = currentIndex
  if (key === 'ArrowDown' || key === 'ArrowRight') nextIndex = (currentIndex + 1) % items.length
  else if (key === 'ArrowUp' || key === 'ArrowLeft') nextIndex = (currentIndex - 1 + items.length) % items.length
  else if (key === 'Home') nextIndex = 0
  else if (key === 'End') nextIndex = items.length - 1
  const nextItem = items[nextIndex]
  if (!nextItem) return
  emit('select', nextItem.id)
  focusWorkspace(nextItem.id)
}
</script>

<template>
  <nav
    id="workspaceBar"
    class="workbenchRail"
    aria-label="Workspace navigation"
    data-qa-panel="workbench.rail"
  >
    <div class="railTabList" role="tablist" aria-label="Workspace navigation" aria-orientation="vertical">
      <div class="railGroup primary">
        <button
          v-for="item in primaryItems"
          :id="item.tabId"
          :key="item.id"
          type="button"
          class="railButton"
          :class="{ active: item.selected }"
          role="tab"
          :aria-selected="item.selected ? 'true' : 'false'"
          :aria-controls="item.ariaControls"
          :tabindex="item.selected ? 0 : -1"
          :title="`${item.label}. ${item.description}`"
          :data-workspace="item.id"
          :data-qa-action="item.actionId"
          @click="selectItem(item)"
          @keydown="onItemKeydown($event, item)"
        >
          <component :is="itemIcon(item)" aria-hidden="true" class="railIcon" />
          <span>{{ item.shortLabel }}</span>
        </button>
      </div>

      <div class="railGroup secondary">
        <button
          v-for="item in secondaryItems"
          :id="item.tabId"
          :key="item.id"
          type="button"
          class="railButton compact"
          :class="{ active: item.selected }"
          role="tab"
          :aria-selected="item.selected ? 'true' : 'false'"
          :aria-controls="item.ariaControls"
          :tabindex="item.selected ? 0 : -1"
          :title="`${item.label}. ${item.description}`"
          :data-workspace="item.id"
          :data-qa-action="item.actionId"
          @click="selectItem(item)"
          @keydown="onItemKeydown($event, item)"
        >
          <component :is="itemIcon(item)" aria-hidden="true" class="railIcon" />
          <span>{{ item.shortLabel }}</span>
        </button>
      </div>
    </div>
  </nav>
</template>

<style scoped>
.workbenchRail {
  min-width: 0;
  min-height: 0;
  padding: 0.6rem 0.38rem;
  border-right: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  background:
    linear-gradient(180deg, color-mix(in oklab, var(--surface-shell) 88%, var(--panel)), var(--surface-sunken));
  overflow: hidden auto;
}

.railTabList {
  min-width: 0;
  min-height: 100%;
  display: grid;
  grid-template-rows: 1fr auto;
  gap: 0.5rem;
}

.railGroup {
  display: grid;
  align-content: start;
  gap: 0.32rem;
}

.railGroup.secondary {
  align-self: end;
}

.railButton {
  width: 100%;
  min-width: 0;
  min-height: 52px;
  padding: 0.42rem 0.22rem;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: var(--muted);
  display: grid;
  justify-items: center;
  gap: 0.2rem;
  font-size: 0.66rem;
  font-weight: 650;
  line-height: 1.12;
}

.railButton.compact {
  min-height: 48px;
}

.railButton:hover:not(:disabled) {
  background: color-mix(in oklab, var(--surface-working) 88%, var(--panel));
  border-color: color-mix(in oklab, var(--border) 82%, transparent);
  transform: none;
}

.railButton.active {
  color: var(--text);
  border-color: color-mix(in oklab, var(--panel-focus-edge) 76%, var(--accent));
  background: color-mix(in oklab, var(--selection-fill) 86%, var(--surface-working));
  box-shadow: inset 3px 0 0 var(--selection-rail);
}

.railIcon {
  width: 1.05rem;
  height: 1.05rem;
}

@media (max-width: 780px) {
  .workbenchRail {
    padding: 0.35rem;
    border-right: 0;
    border-bottom: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
    grid-template-columns: 1fr auto;
    grid-template-rows: auto;
    overflow-x: auto;
  }

  .railGroup {
    display: flex;
    min-width: max-content;
  }

  .railButton {
    width: 72px;
    min-height: 44px;
  }
}
</style>
