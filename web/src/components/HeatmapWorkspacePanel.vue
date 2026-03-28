<script setup lang="ts">
import { computed, nextTick, onMounted, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useShellMount } from '../composables/useShellMount'
import { refreshHeatmapFromWorkflowRun } from '../services/analysisService'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

const analysisStore = useAnalysisStore()
const workflowStore = useWorkflowStore()
const {
  heatmapMeta,
  canHeatmapExportPng,
  heatmapStatusLine,
  updatedAtUnixMs,
} = storeToRefs(analysisStore)
const { resultsTab } = storeToRefs(workflowStore)

const { mountReady } = useShellMount('#heatmapVueShellMount')

const visible = computed(() => String(resultsTab.value || '').trim() === 'heatmap')

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))

onMounted(async () => {
  await nextTick()
  await nextTick()
  refreshHeatmapFromWorkflowRun()
})

watch(visible, async (nextVisible) => {
  if (!nextVisible) return
  await nextTick()
  refreshHeatmapFromWorkflowRun()
})

function reviewTopResults(): void {
  workflowStore.selectResultsTab('top')
}
</script>

<template>
  <Teleport v-if="mountReady" to="#heatmapVueShellMount">
    <section class="heatmapPanelVueRoot heatmapVueShellMountHost" v-show="visible">
      <header class="head">
        <h2 class="panelTitle">Heatmap</h2>
        <div class="panelMeta mono panelNumeric">{{ stamp }}</div>
      </header>

      <section class="pane">
        <div class="metaLine mono" role="status" aria-live="polite">{{ heatmapMeta }}</div>
        <div class="metaLine mono">Zero-centered scale: red negative, blue near zero, green positive.</div>
        <div class="actions">
          <button type="button" :disabled="!canHeatmapExportPng" @click="analysisStore.heatmapExportPng()">Export PNG</button>
        </div>
      </section>

      <section class="pane">
        <h3 class="panelPaneTitle">Heatmap Chart</h3>
        <div id="heatmapEmptyHintVue" class="panelStatus" role="status" aria-live="polite">Run an optimization with Heatmap bins > 0 to generate a heatmap.</div>
        <div class="chartAssist">
          <div id="heatmapKeyboardHint" class="panelMuted">
            Use the Top results table to choose another candidate, or click a heatmap cell if you want direct chart drilldown.
          </div>
          <div class="actions" style="margin-top: 0;">
            <button type="button" @click="reviewTopResults">Review Top results</button>
          </div>
        </div>
        <div class="panelAdvisory">
          Heatmaps require a scored optimization run with Heatmap bins above 0. If the renderer stays empty, verify those settings before rerunning.
        </div>
        <div id="heatmapWrapVue" style="margin-top: 0.75rem; display: none;">
          <div class="panelStatus" style="margin-bottom: 0.5rem;" id="heatmapMetaVue" role="status" aria-live="polite">-</div>
          <div
            class="heatmapCanvasStack chartSurface"
            id="heatmapCanvasStackVue"
            aria-label="Optimization heatmap for the selected objective. Use the Top results tab for a non-pointer way to move between candidates."
            aria-describedby="heatmapKeyboardHint"
          >
            <canvas id="heatmapCanvasVue" width="512" height="512" aria-hidden="true"></canvas>
            <canvas id="heatmapOverlayVue" width="512" height="512" aria-hidden="true"></canvas>
            <div id="heatmapTooltipVue" class="heatmapTooltip" style="display: none;"></div>
          </div>
          <div class="row" style="margin-top: 0.75rem;">
            <div>
              <label>Legend</label>
              <canvas id="heatmapLegendVue" class="heatmapLegendCanvas" width="512" height="18" aria-hidden="true"></canvas>
              <div class="heatmapLegendLabels">
                <span id="heatmapLegendMinVue" class="mono">-</span>
                <span id="heatmapLegendMidVue" class="mono">-</span>
                <span id="heatmapLegendMaxVue" class="mono">-</span>
              </div>
            </div>
            <div>
              <label>&nbsp;</label>
              <div class="panelMuted" style="margin-top: 0.5rem;">
                Select a Top result or click a cell to drill down. Markers and crosshair stay on for both paths.
              </div>
            </div>
          </div>
        </div>
      </section>

      <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ heatmapStatusLine }}</div>
    </section>
  </Teleport>
</template>

<style scoped>
.heatmapPanelVueRoot {
  display: grid;
  gap: 0.85rem;
}

.head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.75rem;
}

.pane {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel-footer);
  padding: 0.75rem 0.8rem;
  min-width: 0;
}

.actions {
  margin-top: 0.65rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.actions button {
  padding: 0.42rem 0.6rem;
  font-size: 0.76rem;
}

.chartAssist {
  margin-top: 0.75rem;
  display: grid;
  gap: 0.35rem;
}

.chartSurface:focus-visible {
  outline: 2px solid var(--focus-border);
  outline-offset: 2px;
  box-shadow: var(--focus-ring);
}

.metaLine {
  font-size: 0.78rem;
  color: var(--muted);
}

.metaLine + .metaLine {
  margin-top: 0.3rem;
}

.mono {
  font-family: var(--mono);
}

</style>




