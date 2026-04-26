<script setup lang="ts">
import { computed } from 'vue'
import AppEmptyState from './AppEmptyState.vue'
import { normalizeTableDensity, type TableDensity } from '../../composables/usePersistedTableDensity'

const props = withDefaults(
  defineProps<{
    title: string
    summary?: string
    ariaLabel?: string
    rowCount?: number
    busy?: boolean
    emptyTitle?: string
    emptyBody?: string
    keyboardHint?: string
    viewport?: boolean
    toolbarLabel?: string
    density?: TableDensity
  }>(),
  {
    summary: '',
    ariaLabel: '',
    rowCount: 0,
    busy: false,
    emptyTitle: '',
    emptyBody: '',
    keyboardHint: '',
    viewport: true,
    toolbarLabel: '',
    density: 'comfortable',
  },
)

const tableDensity = computed(() => normalizeTableDensity(props.density))
const controlsLabel = computed(() => props.toolbarLabel || `${props.title} controls`)
</script>

<template>
  <section
    class="appDataTableShell"
    :class="`density-${tableDensity}`"
    :aria-label="ariaLabel || title"
    :aria-busy="busy ? 'true' : undefined"
    :data-table-density="tableDensity"
  >
    <header class="appDataTableHeader">
      <div class="appDataTableTitleBlock">
        <h3 class="appDataTableTitle">{{ title }}</h3>
        <div v-if="summary" class="appDataTableSummary mono">{{ summary }}</div>
      </div>
      <div v-if="$slots.actions" class="appDataTableActions">
        <slot name="actions" />
      </div>
    </header>

    <div
      v-if="$slots.controls"
      class="appDataTableControls"
      role="toolbar"
      :aria-label="controlsLabel"
    >
      <slot name="controls" />
    </div>

    <AppEmptyState
      v-if="rowCount <= 0 && emptyTitle"
      class="appDataTableEmpty"
      :title="emptyTitle"
      :body="emptyBody"
    >
      <template v-if="$slots.emptyActions" #actions>
        <slot name="emptyActions" />
      </template>
    </AppEmptyState>
    <div
      v-else
      :class="viewport ? 'appDataTableViewport' : 'appDataTableContent'"
      :tabindex="viewport ? 0 : undefined"
    >
      <slot />
    </div>

    <footer v-if="keyboardHint || $slots.footer" class="appDataTableFooter">
      <div v-if="keyboardHint" class="appDataTableHint mono">{{ keyboardHint }}</div>
      <slot name="footer" />
    </footer>
  </section>
</template>

<style scoped>
.appDataTableShell {
  min-width: 0;
  display: grid;
  gap: 0.65rem;
}

.appDataTableHeader {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.75rem;
  min-width: 0;
}

.appDataTableTitleBlock {
  min-width: 0;
  display: grid;
  gap: 0.12rem;
}

.appDataTableTitle {
  margin: 0;
  font-size: 0.88rem;
  line-height: 1.2;
}

.appDataTableSummary {
  color: var(--muted);
  font-size: 0.73rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.appDataTableActions,
.appDataTableControls,
.appDataTableFooter {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.appDataTableActions {
  justify-content: flex-end;
}

.appDataTableControls {
  align-items: center;
  justify-content: space-between;
  position: sticky;
  top: 0;
  z-index: 3;
  padding: 0.35rem;
  border: 1px solid color-mix(in oklab, var(--border) 72%, transparent);
  border-radius: 10px;
  background: color-mix(in oklab, var(--panel) 96%, var(--surface));
  box-shadow: var(--shadow-inline);
}

.appDataTableViewport {
  min-width: 0;
  min-height: 0;
  overflow: auto;
  border: 1px solid color-mix(in oklab, var(--border) 90%, transparent);
  border-radius: 10px;
  background: color-mix(in oklab, var(--panel) 96%, var(--surface));
}

.appDataTableContent {
  min-width: 0;
  min-height: 0;
}

.appDataTableEmpty {
  border-radius: 10px;
}

.appDataTableFooter {
  align-items: center;
  justify-content: space-between;
  color: var(--muted);
  font-size: 0.73rem;
}

.mono {
  font-family: var(--mono);
}

.appDataTableShell.density-compact {
  gap: 0.45rem;
}

.appDataTableShell.density-spacious {
  gap: 0.85rem;
}

.appDataTableShell.density-compact :deep(th),
.appDataTableShell.density-compact :deep(td) {
  padding-top: 0.34rem;
  padding-bottom: 0.34rem;
}

.appDataTableShell.density-spacious :deep(th),
.appDataTableShell.density-spacious :deep(td) {
  padding-top: 0.68rem;
  padding-bottom: 0.68rem;
}

.appDataTableShell.density-compact :deep(.runListButton) {
  padding-top: 0.48rem;
  padding-bottom: 0.48rem;
}

.appDataTableShell.density-spacious :deep(.runListButton) {
  padding-top: 0.78rem;
  padding-bottom: 0.78rem;
}

.appDataTableShell.density-compact :deep(.pillRow),
.appDataTableShell.density-compact :deep(.runListSecondary) {
  margin-top: 0.2rem;
}

.appDataTableShell.density-spacious :deep(.pillRow),
.appDataTableShell.density-spacious :deep(.runListSecondary) {
  margin-top: 0.42rem;
}

@media (max-width: 760px) {
  .appDataTableHeader,
  .appDataTableFooter {
    display: grid;
    justify-content: stretch;
  }

  .appDataTableActions,
  .appDataTableControls {
    justify-content: flex-start;
  }
}
</style>
