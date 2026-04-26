<script setup lang="ts">
import { computed, ref } from 'vue'
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
    keyboardGridNavigation?: boolean
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
    keyboardGridNavigation: true,
  },
)

const tableDensity = computed(() => normalizeTableDensity(props.density))
const controlsLabel = computed(() => props.toolbarLabel || `${props.title} controls`)
const tableContentRef = ref<HTMLElement | null>(null)

const focusableSelector = [
  'button',
  '[href]',
  'input',
  'select',
  'textarea',
  '[tabindex]:not([tabindex="-1"])',
  '[contenteditable="true"]',
].join(',')
const cellSelector = [
  'td',
  'th',
  '[role="gridcell"]',
  '[role="cell"]',
  '[role="columnheader"]',
  '[role="rowheader"]',
].join(',')

function isEditableTarget(target: EventTarget | null): boolean {
  const element = target instanceof HTMLElement ? target : null
  if (!element) return false
  return !!element.closest('button, input, select, textarea, a[href], [contenteditable="true"]')
}

function dataRowsFor(table: HTMLElement): HTMLTableRowElement[] {
  return Array.from(table.querySelectorAll<HTMLTableRowElement>('tbody tr')).filter(
    (row) => row.getAttribute('aria-hidden') !== 'true' && !row.classList.contains('spacerRow'),
  )
}

function cellsFor(row: HTMLTableRowElement): HTMLElement[] {
  return Array.from(row.querySelectorAll<HTMLElement>(cellSelector)).filter(
    (cell) => cell.getAttribute('aria-hidden') !== 'true',
  )
}

function makeFocusable(element: HTMLElement): void {
  if (!element.matches(focusableSelector)) {
    element.setAttribute('tabindex', element.getAttribute('tabindex') || '-1')
  }
  element.focus({ preventScroll: true })
  element.scrollIntoView?.({ block: 'nearest', inline: 'nearest' })
}

function focusRowOrCell(row: HTMLTableRowElement, columnIndex: number, preferRow: boolean): void {
  if (preferRow && row.hasAttribute('tabindex')) {
    makeFocusable(row)
    return
  }
  const cells = cellsFor(row)
  makeFocusable(cells[Math.min(Math.max(columnIndex, 0), Math.max(0, cells.length - 1))] ?? row)
}

function focusInitialTableRow(table: HTMLElement): void {
  const rows = dataRowsFor(table)
  if (!rows.length) return
  const selectedRow =
    rows.find((row) => row.getAttribute('aria-selected') === 'true' || row.classList.contains('selected')) ??
    rows[0]
  if (selectedRow) focusRowOrCell(selectedRow, 0, true)
}

function onShellGridKeydown(event: KeyboardEvent): void {
  if (!props.keyboardGridNavigation || event.defaultPrevented || isEditableTarget(event.target)) return
  const root = tableContentRef.value
  const target = event.target instanceof HTMLElement ? event.target : null
  if (!root || !target || !root.contains(target)) return

  const key = event.key
  if (!['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Home', 'End', 'PageUp', 'PageDown'].includes(key)) return

  const table = (target.closest('table, [role="grid"], [role="table"]') ||
    root.querySelector('table, [role="grid"], [role="table"]')) as HTMLElement | null
  if (!table) return

  const rows = dataRowsFor(table)
  if (!rows.length) return

  const currentRow = target.closest('tbody tr') as HTMLTableRowElement | null
  if (!currentRow || !rows.includes(currentRow)) {
    if (key === 'ArrowDown' || key === 'ArrowRight' || key === 'Home' || key === 'PageDown') {
      event.preventDefault()
      focusInitialTableRow(table)
    }
    return
  }

  const currentCells = cellsFor(currentRow)
  const currentCell = target.closest(cellSelector) as HTMLElement | null
  const preferRow = target === currentRow || !currentCell
  const currentRowIndex = rows.indexOf(currentRow)
  const currentColumnIndex = currentCell ? Math.max(0, currentCells.indexOf(currentCell)) : 0
  let nextRowIndex = currentRowIndex
  let nextColumnIndex = currentColumnIndex
  let nextPreferRow = preferRow

  if (key === 'ArrowUp') nextRowIndex = Math.max(0, currentRowIndex - 1)
  else if (key === 'ArrowDown') nextRowIndex = Math.min(rows.length - 1, currentRowIndex + 1)
  else if (key === 'PageUp') nextRowIndex = Math.max(0, currentRowIndex - 10)
  else if (key === 'PageDown') nextRowIndex = Math.min(rows.length - 1, currentRowIndex + 10)
  else if (key === 'ArrowLeft') {
    nextPreferRow = false
    nextColumnIndex = Math.max(0, currentColumnIndex - 1)
  } else if (key === 'ArrowRight') {
    nextPreferRow = false
    nextColumnIndex = currentCell ? currentColumnIndex + 1 : 0
  } else if (key === 'Home') {
    if (event.ctrlKey) nextRowIndex = 0
    else {
      nextPreferRow = false
      nextColumnIndex = 0
    }
  } else if (key === 'End') {
    if (event.ctrlKey) nextRowIndex = rows.length - 1
    else {
      nextPreferRow = false
      nextColumnIndex = Math.max(0, currentCells.length - 1)
    }
  }

  const nextRow = rows[nextRowIndex]
  if (!nextRow) return
  const nextCells = cellsFor(nextRow)
  nextColumnIndex = Math.min(nextColumnIndex, Math.max(0, nextCells.length - 1))
  event.preventDefault()
  focusRowOrCell(nextRow, nextColumnIndex, nextPreferRow)
}
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
      ref="tableContentRef"
      :class="viewport ? 'appDataTableViewport' : 'appDataTableContent'"
      :tabindex="viewport ? 0 : undefined"
      :data-keyboard-grid-navigation="keyboardGridNavigation ? 'true' : undefined"
      @keydown="onShellGridKeydown"
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
