<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useShellMount } from '../composables/useShellMount'
import { refreshHeatmapFromWorkflowRun, restoreHeatmapVisible } from '../services/analysisService'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

interface OptionItem {
  value: string
  label: string
  disabled: boolean
}

const analysisStore = useAnalysisStore()
const workflowStore = useWorkflowStore()
const {
  heatmapMode,
  heatmapModeOptions,
  heatmapMeta,
  canHeatmapExportPng,
  heatmapStatusLine,
  updatedAtUnixMs,
} = storeToRefs(analysisStore)
const { resultsTab } = storeToRefs(workflowStore)

const { mountReady } = useShellMount('#heatmapVueShellMount')
const modeDraft = ref('objective')

const visible = computed(() => String(resultsTab.value || '').trim() === 'heatmap')

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))

const modeOptions = computed<OptionItem[]>(() => {
  const options = heatmapModeOptions.value as OptionItem[]
  if (options.length > 0) return options
  return [{ value: 'objective', label: 'Objective score', disabled: false }]
})

const chartAriaLabel = computed(() =>
  String(heatmapMode.value || '').trim() === 'plateau'
    ? 'Derived plateau heatmap for the selected objective. Use Top results for exact retained candidates, or click a populated heatmap cell for a representative parameter pair from that region.'
    : 'Optimization heatmap for the selected objective. Use Top results for exact retained candidates, or click a populated heatmap cell for a representative parameter pair from that region.',
)

const scaleNote = computed(() =>
  String(heatmapMode.value || '').trim() === 'plateau'
    ? 'Derived plateau scale: blue fragile, green stable. The score uses supportive 8-neighbor agreement, roughness, curvature, and bin-consistency when mean/count aggregates are available. Click drills a representative parameter pair for the selected heatmap region; Top results remains the exact retained-candidate path.'
    : 'Zero-centered scale: red negative, blue near zero, green positive. Click drills a representative parameter pair for the selected heatmap region.',
)

async function ensureVisibleHeatmap(): Promise<void> {
  await nextTick()
  if (!restoreHeatmapVisible()) {
    refreshHeatmapFromWorkflowRun()
  }
  const stack = document.getElementById('heatmapCanvasStackVue')
  const legend = document.getElementById('heatmapLegendVue')
  if (!stack || typeof stack.scrollIntoView !== 'function') return
  const stackRect = stack.getBoundingClientRect()
  const legendRect = legend?.getBoundingClientRect()
  const belowViewport =
    stackRect.top < 0 ||
    (legendRect ? legendRect.bottom > window.innerHeight : stackRect.bottom > window.innerHeight)
  if (belowViewport) {
    stack.scrollIntoView({ block: 'start', inline: 'nearest' })
  }
}

watch(
  () => String(heatmapMode.value || 'objective'),
  (nextMode) => {
    if (modeDraft.value !== nextMode) modeDraft.value = nextMode
  },
  { immediate: true },
)

watch([visible, mountReady], async ([nextVisible, nextReady]) => {
  if (!nextVisible || !nextReady) return
  await ensureVisibleHeatmap()
}, { immediate: true })

function setHeatmapMode(nextMode: string): void {
  analysisStore.setSelectField('heatmapMode', nextMode)
}

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
        <div class="heatmapControls">
          <div>
            <label for="vueHeatmapMode">Display mode</label>
            <select
              id="vueHeatmapMode"
              :value="modeDraft"
              @change="modeDraft = String(($event.target as HTMLSelectElement).value || 'objective'); setHeatmapMode(modeDraft)"
            >
              <option
                v-for="option in modeOptions"
                :key="`heatmap_mode_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
        </div>
        <div class="metaLine mono" role="status" aria-live="polite">{{ heatmapMeta }}</div>
        <div class="metaLine mono">{{ scaleNote }}</div>
        <div class="actions">
          <button type="button" :disabled="!canHeatmapExportPng" @click="analysisStore.heatmapExportPng()">Export PNG</button>
        </div>
      </section>

      <section class="pane">
        <h3 class="panelPaneTitle">Heatmap Chart</h3>
        <div id="heatmapEmptyHintVue" class="panelStatus" role="status" aria-live="polite">Run an optimization with Heatmap bins > 0 to generate a heatmap.</div>
        <div class="chartAssist">
          <div id="heatmapKeyboardHint" class="panelMuted">
            Use the Top results table to open an exact retained candidate, or click a populated heatmap cell to inspect a representative parameter pair for that region.
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
            :aria-label="chartAriaLabel"
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
                Select a Top result for exact retained candidates, or click a populated cell for a representative heatmap-region drilldown. Markers and crosshair stay on for both paths.
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

.heatmapControls {
  display: grid;
  gap: 0.65rem;
  grid-template-columns: minmax(0, 15rem);
  margin-bottom: 0.55rem;
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




