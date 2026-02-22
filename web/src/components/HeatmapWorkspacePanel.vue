<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'

interface OptionItem {
  value: string
  label: string
  disabled: boolean
}

const analysisStore = useAnalysisStore()
const workflowStore = useWorkflowStore()
const {
  heatmapMeta,
  heatmapWrapVisible,
  heatmapColor,
  heatmapColorOptions,
  heatmapRange,
  heatmapRangeOptions,
  heatmapShowTop,
  heatmapShowTopOptions,
  heatmapShowCrosshair,
  heatmapShowCrosshairOptions,
  canHeatmapExportPng,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(analysisStore)
const { resultsTab } = storeToRefs(workflowStore)

const mountReady = ref(false)
const panelRoot = ref<HTMLElement | null>(null)

const colorDraft = ref('cividis')
const rangeDraft = ref('q2_98')
const showTopDraft = ref('true')
const showCrosshairDraft = ref('true')

const visible = computed(() => String(resultsTab.value || '').trim() === 'heatmap')

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
})

function resolvedOptions(options: OptionItem[], fallbackValue: string, fallbackLabel: string): OptionItem[] {
  if (options.length > 0) return options
  const value = String(fallbackValue || '').trim()
  if (!value) return []
  return [{ value, label: String(fallbackLabel || value).trim() || value, disabled: false }]
}

const colorOptions = computed(() => resolvedOptions(heatmapColorOptions.value as OptionItem[], heatmapColor.value, heatmapColor.value || 'cividis'))
const rangeOptions = computed(() => resolvedOptions(heatmapRangeOptions.value as OptionItem[], heatmapRange.value, heatmapRange.value || 'q2_98'))
const showTopOptions = computed(() => resolvedOptions(heatmapShowTopOptions.value as OptionItem[], heatmapShowTop.value, heatmapShowTop.value || 'true'))
const showCrosshairOptions = computed(() => resolvedOptions(heatmapShowCrosshairOptions.value as OptionItem[], heatmapShowCrosshair.value, heatmapShowCrosshair.value || 'true'))

watch(heatmapColor, (v) => { const next = String(v || 'cividis'); if (colorDraft.value !== next) colorDraft.value = next }, { immediate: true })
watch(heatmapRange, (v) => { const next = String(v || 'q2_98'); if (rangeDraft.value !== next) rangeDraft.value = next }, { immediate: true })
watch(heatmapShowTop, (v) => { const next = String(v || 'true'); if (showTopDraft.value !== next) showTopDraft.value = next }, { immediate: true })
watch(heatmapShowCrosshair, (v) => { const next = String(v || 'true'); if (showCrosshairDraft.value !== next) showCrosshairDraft.value = next }, { immediate: true })

function setSelectValue(id: string, nextValue: string): void {
  analysisStore.setSelectField(id, nextValue)
}

function resolveMount(): void {
  const panel = document.querySelector('[data-vue-panel="heatmap"]') as HTMLElement | null
  const mount = document.getElementById('heatmapVueShellMount')
  if (!panel || !mount) return
  panelRoot.value = panel
  panel.dataset.vueHeatmap = '1'
  mountReady.value = true
}

onMounted(async () => {
  await nextTick()
  resolveMount()
  // Ask legacy render path to repaint once Vue chart host exists.
  window.setTimeout(() => {
    setSelectValue('heatmapColor', colorDraft.value || 'cividis')
  }, 0)
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
        <div class="metaLine mono">Heatmap visible: {{ heatmapWrapVisible ? 'yes' : 'no' }}</div>
        <div class="gridFour">
          <div>
            <label for="vueHeatmapColor">Colormap</label>
            <select
              id="vueHeatmapColor"
              :value="colorDraft"
              @change="colorDraft = String(($event.target as HTMLSelectElement).value || 'cividis'); setSelectValue('heatmapColor', colorDraft)"
            >
              <option
                v-for="option in colorOptions"
                :key="`color_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
          <div>
            <label for="vueHeatmapRange">Range</label>
            <select
              id="vueHeatmapRange"
              :value="rangeDraft"
              @change="rangeDraft = String(($event.target as HTMLSelectElement).value || 'q2_98'); setSelectValue('heatmapRange', rangeDraft)"
            >
              <option
                v-for="option in rangeOptions"
                :key="`range_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
          <div>
            <label for="vueHeatmapShowTop">Top markers</label>
            <select
              id="vueHeatmapShowTop"
              :value="showTopDraft"
              @change="showTopDraft = String(($event.target as HTMLSelectElement).value || 'true'); setSelectValue('heatmapShowTop', showTopDraft)"
            >
              <option
                v-for="option in showTopOptions"
                :key="`showtop_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
          <div>
            <label for="vueHeatmapShowCrosshair">Crosshair</label>
            <select
              id="vueHeatmapShowCrosshair"
              :value="showCrosshairDraft"
              @change="showCrosshairDraft = String(($event.target as HTMLSelectElement).value || 'true'); setSelectValue('heatmapShowCrosshair', showCrosshairDraft)"
            >
              <option
                v-for="option in showCrosshairOptions"
                :key="`cross_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
        </div>
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
                Tip: click a cell to drill down. Markers = Top rows (rank).
              </div>
            </div>
          </div>
        </div>
      </section>

      <div class="status mono">{{ statusLine }}</div>
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

.gridFour {
  margin-top: 0.55rem;
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.6rem;
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

@media (max-width: 1100px) {
  .gridFour {
    grid-template-columns: 1fr 1fr;
  }
}

@media (max-width: 680px) {
  .gridFour {
    grid-template-columns: 1fr;
  }
}
</style>

<style>
[data-vue-panel='heatmap'][data-vue-heatmap='1'] > :not(#heatmapVueShellMount) {
  display: none !important;
}
</style>


