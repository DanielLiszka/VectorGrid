<script setup lang="ts">
import AppEmptyState from './AppEmptyState.vue'

withDefaults(
  defineProps<{
    title: string
    summary?: string
    ariaLabel?: string
    rowCount?: number
    busy?: boolean
    emptyTitle?: string
    emptyBody?: string
    keyboardHint?: string
  }>(),
  {
    summary: '',
    ariaLabel: '',
    rowCount: 0,
    busy: false,
    emptyTitle: '',
    emptyBody: '',
    keyboardHint: '',
  },
)
</script>

<template>
  <section class="appDataTableShell" :aria-label="ariaLabel || title" :aria-busy="busy ? 'true' : undefined">
    <header class="appDataTableHeader">
      <div class="appDataTableTitleBlock">
        <h3 class="appDataTableTitle">{{ title }}</h3>
        <div v-if="summary" class="appDataTableSummary mono">{{ summary }}</div>
      </div>
      <div v-if="$slots.actions" class="appDataTableActions">
        <slot name="actions" />
      </div>
    </header>

    <div v-if="$slots.controls" class="appDataTableControls">
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
    <div v-else class="appDataTableViewport" tabindex="0">
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
}

.appDataTableViewport {
  min-width: 0;
  min-height: 0;
  overflow: auto;
  border: 1px solid color-mix(in oklab, var(--border) 90%, transparent);
  border-radius: 10px;
  background: color-mix(in oklab, var(--panel) 96%, var(--surface));
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
