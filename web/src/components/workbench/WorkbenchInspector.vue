<script setup lang="ts">
import { nextTick } from 'vue'
import { ChevronLeft, ChevronRight } from '@lucide/vue'
import type { WorkbenchInspectorMode, WorkbenchInspectorState } from '../../utils/inspectorModel'

const props = defineProps<{
  state: WorkbenchInspectorState
  collapsed: boolean
  compact?: boolean
  compactOpen?: boolean
}>()

const emit = defineEmits<{
  toggle: []
  closeCompact: []
  selectMode: [mode: WorkbenchInspectorMode]
  runAction: [actionId: string]
}>()

function focusMode(mode: WorkbenchInspectorMode): void {
  nextTick(() => document.querySelector<HTMLElement>(`#workbench_inspector_mode_${mode}`)?.focus())
}

function onModeKeydown(event: KeyboardEvent, mode: WorkbenchInspectorMode): void {
  const key = event.key
  if (!['ArrowRight', 'ArrowDown', 'ArrowLeft', 'ArrowUp', 'Home', 'End'].includes(key)) return
  event.preventDefault()
  const modes = props.state.modes
  if (modes.length === 0) return
  const currentIndex = Math.max(0, modes.findIndex((item) => item.id === mode))
  let nextIndex = currentIndex
  if (key === 'ArrowRight' || key === 'ArrowDown') nextIndex = (currentIndex + 1) % modes.length
  else if (key === 'ArrowLeft' || key === 'ArrowUp') nextIndex = (currentIndex - 1 + modes.length) % modes.length
  else if (key === 'Home') nextIndex = 0
  else if (key === 'End') nextIndex = modes.length - 1
  const nextMode = modes[nextIndex]?.id
  if (!nextMode) return
  emit('selectMode', nextMode)
  focusMode(nextMode)
}

function closeInspector(): void {
  if (props.compact) {
    emit('closeCompact')
    return
  }
  emit('toggle')
}
</script>

<template>
  <aside
    class="workbenchInspector"
    :class="{ collapsed, compact, compactOpen }"
    aria-label="Workbench inspector"
    data-qa-panel="workbench.inspector"
    :data-collapsed="collapsed ? 'true' : 'false'"
    :aria-hidden="compact && !compactOpen ? 'true' : undefined"
    :inert="compact && !compactOpen"
  >
    <header class="inspectorHeader">
      <div v-if="!collapsed" class="headerCopy">
        <span class="eyebrow">Inspector</span>
        <strong data-qa-state="workbench.inspector.activeMode">{{ state.modes.find((item) => item.current)?.label }}</strong>
      </div>
      <button
        type="button"
        class="toggleButton"
        data-qa-action="workbench.inspector.toggle"
        :aria-label="compact ? 'Close inspector' : collapsed ? 'Expand inspector' : 'Collapse inspector'"
        @click="closeInspector"
      >
        <ChevronLeft v-if="!collapsed || compact" aria-hidden="true" />
        <ChevronRight v-else aria-hidden="true" />
      </button>
    </header>

    <div v-if="!collapsed" class="modeTabs" role="tablist" aria-label="Inspector modes">
      <button
        v-for="mode in state.modes"
        :id="`workbench_inspector_mode_${mode.id}`"
        :key="mode.id"
        type="button"
        class="modeButton"
        :class="[{ active: mode.current }, mode.tone]"
        role="tab"
        :aria-selected="mode.current ? 'true' : 'false'"
        :tabindex="mode.current ? 0 : -1"
        :data-qa-action="mode.actionId"
        @click="emit('selectMode', mode.id)"
        @keydown="onModeKeydown($event, mode.id)"
      >
        {{ mode.label }}
      </button>
    </div>

    <section
      v-if="!collapsed"
      class="inspectorBody"
      aria-label="Inspector details"
      aria-live="polite"
      tabindex="0"
    >
      <article
        v-for="section in state.sections"
        :key="section.id"
        class="inspectorCard"
        :class="section.tone"
        :data-qa-state="`workbench.inspector.section.${section.id}`"
      >
        <span class="cardTitle">{{ section.title }}</span>
        <strong>{{ section.value }}</strong>
        <span v-if="section.detail" class="cardDetail">{{ section.detail }}</span>
      </article>
    </section>

    <footer v-if="!collapsed && state.actions.length > 0" class="inspectorActions" aria-label="Inspector actions">
      <button
        v-for="action in state.actions"
        :key="action.id"
        type="button"
        class="inspectorAction"
        :class="action.tone"
        :disabled="action.disabled"
        :data-qa-action="action.actionId"
        @click="emit('runAction', action.id)"
      >
        {{ action.label }}
      </button>
    </footer>
  </aside>
</template>

<style scoped>
.workbenchInspector {
  min-width: 0;
  min-height: 0;
  border-left: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  background: color-mix(in oklab, var(--surface-shell) 80%, var(--panel));
  display: grid;
  grid-template-rows: auto auto minmax(0, 1fr) auto;
  gap: 0.55rem;
  padding: 0.68rem;
  overflow: hidden;
}

.workbenchInspector.collapsed {
  padding: 0.5rem 0.36rem;
  justify-items: center;
}

.inspectorHeader {
  min-width: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.headerCopy {
  min-width: 0;
  display: grid;
  gap: 0.08rem;
}

.headerCopy strong {
  min-width: 0;
  font-size: 0.96rem;
  line-height: 1.2;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.eyebrow,
.cardTitle {
  font-size: var(--text-xs);
  font-weight: 700;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--muted);
}

.toggleButton {
  width: 32px;
  height: 32px;
  padding: 0;
  border-radius: 8px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: 0 0 auto;
}

.toggleButton svg {
  width: 1rem;
  height: 1rem;
}

.modeTabs {
  display: flex;
  flex-wrap: wrap;
  gap: 0.28rem;
}

.modeButton {
  flex: 1 1 auto;
  min-width: min(6.4rem, 100%);
  padding: 0.34rem 0.42rem;
  border-radius: 8px;
  font-size: var(--text-xs);
  font-weight: 650;
  color: var(--muted);
  background: color-mix(in oklab, var(--surface-inline) 90%, transparent);
  overflow: hidden;
  text-overflow: ellipsis;
}

.modeButton.active {
  color: var(--text);
  border-color: color-mix(in oklab, var(--panel-focus-edge) 72%, var(--border));
  background: color-mix(in oklab, var(--selection-fill) 86%, var(--surface-working));
  box-shadow: inset 0 -2px 0 var(--selection-rail);
}

.modeButton.warning:not(.active) {
  color: var(--warning-ink);
}

.modeButton.danger:not(.active) {
  color: var(--danger-ink);
}

.inspectorBody {
  min-height: 0;
  overflow: auto;
  display: grid;
  align-content: start;
  gap: 0.5rem;
  padding-right: 0.08rem;
}

.inspectorCard {
  min-width: 0;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-working) 94%, var(--panel));
  padding: 0.62rem 0.66rem;
  display: grid;
  gap: 0.16rem;
}

.inspectorCard strong {
  min-width: 0;
  font-size: 0.88rem;
  line-height: 1.2;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.cardDetail {
  min-width: 0;
  color: var(--muted-strong);
  font-size: 0.76rem;
  line-height: 1.35;
  overflow-wrap: anywhere;
}

.inspectorCard.accent {
  border-color: color-mix(in oklab, var(--panel-focus-edge) 58%, var(--border));
}

.inspectorCard.success {
  background: color-mix(in oklab, var(--surface-success) 68%, var(--surface-working));
}

.inspectorCard.warning {
  background: color-mix(in oklab, var(--surface-warning) 68%, var(--surface-working));
}

.inspectorCard.danger {
  background: color-mix(in oklab, var(--surface-danger) 68%, var(--surface-working));
}

.inspectorActions {
  min-width: 0;
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
  border-top: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  padding-top: 0.5rem;
}

.inspectorAction {
  min-height: 30px;
  border-radius: 8px;
  padding: 0.3rem 0.48rem;
  font-size: var(--text-xs);
  font-weight: 650;
  color: var(--muted-strong);
  background: color-mix(in oklab, var(--surface-inline) 90%, transparent);
}

.inspectorAction.accent {
  color: var(--text);
  border-color: color-mix(in oklab, var(--panel-focus-edge) 72%, var(--border));
  background: color-mix(in oklab, var(--selection-fill) 86%, var(--surface-working));
}

.inspectorAction:disabled {
  opacity: 0.55;
}

@media (max-width: 1180px) {
  .workbenchInspector {
    position: fixed;
    top: calc(var(--desktop-titlebar-h, 0px) + 0.5rem);
    right: 0.5rem;
    bottom: 4.2rem;
    z-index: 1260;
    width: min(330px, calc(100vw - 1rem));
    border: 1px solid color-mix(in oklab, var(--border) 92%, transparent);
    border-radius: 10px;
    box-shadow: 0 18px 44px rgb(var(--shadow-overflow-rgb) / 0.28);
    transform: translateX(calc(100% + 1rem));
    visibility: hidden;
    pointer-events: none;
    transition: transform 160ms ease, visibility 0s linear 160ms;
  }

  .workbenchInspector.compactOpen {
    transform: translateX(0);
    visibility: visible;
    pointer-events: auto;
    transition: transform 160ms ease;
  }
}

@media (prefers-reduced-motion: reduce) {
  .workbenchInspector {
    transition: none;
  }
}
</style>
