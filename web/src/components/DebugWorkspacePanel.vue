<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useShellMount } from '../composables/useShellMount'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

const analysisStore = useAnalysisStore()
const workflowStore = useWorkflowStore()
const {
  debugOutputPreview,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(analysisStore)
const { resultsTab } = storeToRefs(workflowStore)

const { mountReady } = useShellMount('#debugVueShellMount')

const visible = computed(() => String(resultsTab.value || '').trim() === 'debug')

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))

</script>

<template>
  <Teleport v-if="mountReady" to="#debugVueShellMount">
    <section class="debugPanelVueRoot debugVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="panelHeadCopy">
          <div class="panelEyebrow">Diagnostics</div>
          <h2 class="panelTitle">Debug JSON</h2>
          <div class="panelSubtitle">
            Raw analysis payloads stay available here for troubleshooting, but they are intentionally separated from the main review flow.
          </div>
        </div>
        <div class="panelMeta mono panelNumeric">{{ stamp }}</div>
      </header>

      <section class="pane">
        <details class="panelDetails" open>
          <summary>Raw analysis payload</summary>
          <div class="panelDetailsBody">
            <div class="panelMuted">
              Use this view when you need the exact JSON backing a result surface or a troubleshooting reproduction.
            </div>
            <pre class="mono output">{{ debugOutputPreview }}</pre>
          </div>
        </details>
      </section>

      <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ statusLine }}</div>
    </section>
  </Teleport>
</template>

<style scoped>
.debugPanelVueRoot {
  display: grid;
  gap: 0.85rem;
}

.head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
}

.pane {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel-footer);
  padding: 0.75rem 0.8rem;
  min-width: 0;
}

.output {
  margin: 0;
  max-height: min(58vh, 520px);
  overflow: auto;
  white-space: pre-wrap;
}

.mono {
  font-family: var(--mono);
}
</style>



