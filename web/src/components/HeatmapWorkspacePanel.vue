<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { refreshHeatmapFromWorkflowRun } from '../services/analysisService'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'

const analysisStore = useAnalysisStore()
const workflowStore = useWorkflowStore()
const {
  heatmapMeta,
  canHeatmapExportPng,
  heatmapStatusLine,
  updatedAtUnixMs,
} = storeToRefs(analysisStore)
const { resultsTab } = storeToRefs(workflowStore)

const mountReady = ref(false)
const panelRoot = ref<HTMLElement | null>(null)

const visible = computed(() => String(resultsTab.value || '').trim() === 'heatmap')

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
})

function resolveMount(): void {
  const panel = document.querySelector('[data-vue-panel="heatmap"]') as HTMLElement | null
  const mount = document.querySelector('#heatmapVueShellMount') as HTMLElement | null
  if (!panel || !mount) return
  panelRoot.value = panel
  panel.dataset.vueHeatmap = '1'
  mountReady.value = true
}

onMounted(async () => {
  await nextTick()
  resolveMount()
  await nextTick()
  refreshHeatmapFromWorkflowRun()
})

watch(visible, async (nextVisible) => {
  if (!nextVisible) return
  await nextTick()
  refreshHeatmapFromWorkflowRun()
})

onBeforeUnmount(() => {
  if (panelRoot.value) {
    delete panelRoot.value.dataset.vueHeatmap
    panelRoot.value = null
  }
})
</script>

<template>
  <Teleport v-if="mountReady" to="#heatmapVueShellMount">
    <section class="heatmapPanelVueRoot heatmapVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="title">Heatmap</div>
        <div class="meta mono">{{ stamp }}</div>
      </header>

      <section class="pane">
        <div class="metaLine mono">{{ heatmapMeta }}</div>
        <div class="metaLine mono">Zero-centered scale: red negative, blue near zero, green positive.</div>
        <div class="actions">
          <button type="button" :disabled="!canHeatmapExportPng" @click="analysisStore.heatmapExportPng()">Export PNG</button>
        </div>
      </section>

      <section class="pane">
        <h3>Heatmap Chart</h3>
        <div id="heatmapEmptyHintVue" class="muted" style="font-size: 0.8rem;">Run an optimization with Heatmap bins > 0 to generate a heatmap.</div>
        <div id="heatmapWrapVue" style="margin-top: 0.75rem; display: none;">
          <div class="muted" style="margin-bottom: 0.5rem; font-size: 0.8rem;" id="heatmapMetaVue">-</div>
          <div class="heatmapCanvasStack" id="heatmapCanvasStackVue">
            <canvas id="heatmapCanvasVue" width="512" height="512"></canvas>
            <canvas id="heatmapOverlayVue" width="512" height="512"></canvas>
            <div id="heatmapTooltipVue" class="heatmapTooltip" style="display: none;"></div>
          </div>
          <div class="row" style="margin-top: 0.75rem;">
            <div>
              <label>Legend</label>
              <canvas id="heatmapLegendVue" class="heatmapLegendCanvas" width="512" height="18"></canvas>
              <div class="heatmapLegendLabels">
                <span id="heatmapLegendMinVue" class="mono">-</span>
                <span id="heatmapLegendMidVue" class="mono">-</span>
                <span id="heatmapLegendMaxVue" class="mono">-</span>
              </div>
            </div>
            <div>
              <label>&nbsp;</label>
              <div class="muted" style="margin-top: 0.5rem; font-size: 0.8rem;">
                Click a cell to drill down. Markers and crosshair are always on.
              </div>
            </div>
          </div>
        </div>
      </section>

      <div class="status mono">{{ heatmapStatusLine }}</div>
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

.title {
  font-size: 0.95rem;
  font-weight: 650;
}

.meta {
  font-size: 0.78rem;
  color: var(--muted);
}

.pane {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel-footer);
  padding: 0.75rem 0.8rem;
  min-width: 0;
}

.pane h3 {
  margin: 0 0 0.55rem;
  font-size: 0.86rem;
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

.metaLine {
  font-size: 0.78rem;
  color: var(--muted);
}

.metaLine + .metaLine {
  margin-top: 0.3rem;
}

.status {
  font-size: 0.78rem;
  color: var(--muted);
}

.muted {
  color: var(--muted);
  font-size: 0.8rem;
}

.mono {
  font-family: var(--mono);
}

</style>

<style>
[data-vue-panel='heatmap'][data-vue-heatmap='1'] > :not(#heatmapVueShellMount) {
  display: none !important;
}
</style>



