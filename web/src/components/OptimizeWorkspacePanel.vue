<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useOptimizeStore, type OptimizeOption } from '../stores/optimizeStore'
import { useWorkflowStore } from '../stores/workflowStore'

const optimizeStore = useOptimizeStore()
const workflowStore = useWorkflowStore()
const {
  strategyKind,
  strategyHint,
  strategyOptions,
  presetProfile,
  presetOptions,
  backend,
  backendOptions,
  deviceId,
  deviceOptions,
  objective,
  mode,
  fastStart,
  fastEnd,
  fastStep,
  slowStart,
  slowEnd,
  slowStep,
  fastMaType,
  fastMaTypeOptions,
  slowMaType,
  slowMaTypeOptions,
  pairsPreview,
  gpuStatus,
  gpuParityMeta,
  progressMeta,
  statusLine,
  runDisabled,
  validateDisabled,
  canApplyPreset,
  canPresetKernel,
  canRefreshGpuStatus,
  canParityCheck,
  updatedAtUnixMs,
} = storeToRefs(optimizeStore)
const { controlsTab } = storeToRefs(workflowStore)

const mountReady = ref(false)
const panelRoot = ref<HTMLElement | null>(null)

const strategyDraft = ref('')
const presetDraft = ref('')
const backendDraft = ref('')
const deviceDraft = ref('0')
const objectiveDraft = ref('Sharpe')
const modeDraft = ref('Grid')
const fastStartDraft = ref('')
const fastEndDraft = ref('')
const fastStepDraft = ref('')
const slowStartDraft = ref('')
const slowEndDraft = ref('')
const slowStepDraft = ref('')
const fastTypeDraft = ref('')
const slowTypeDraft = ref('')

const visible = computed(() => String(controlsTab.value || '').trim() === 'opt')

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
})

function resolvedOptions(options: OptimizeOption[], fallbackValue: string, fallbackLabel: string): OptimizeOption[] {
  if (options.length > 0) return options
  const value = String(fallbackValue || '').trim()
  if (!value) return []
  return [{ value, label: String(fallbackLabel || value).trim() || value, disabled: false }]
}

const strategyOptionItems = computed(() => resolvedOptions(strategyOptions.value, strategyKind.value, strategyKind.value || 'Strategy'))
const presetOptionItems = computed(() => resolvedOptions(presetOptions.value, presetProfile.value, presetProfile.value || 'Preset'))
const backendOptionItems = computed(() => resolvedOptions(backendOptions.value, backend.value, backend.value || 'Backend'))
const deviceOptionItems = computed(() => resolvedOptions(deviceOptions.value, deviceId.value || '0', deviceId.value || 'Auto'))
const fastTypeOptionItems = computed(() => resolvedOptions(fastMaTypeOptions.value, fastMaType.value, fastMaType.value || 'MA type'))
const slowTypeOptionItems = computed(() => resolvedOptions(slowMaTypeOptions.value, slowMaType.value, slowMaType.value || 'MA type'))

watch(strategyKind, (v) => { const next = String(v || ''); if (strategyDraft.value !== next) strategyDraft.value = next }, { immediate: true })
watch(presetProfile, (v) => { const next = String(v || ''); if (presetDraft.value !== next) presetDraft.value = next }, { immediate: true })
watch(backend, (v) => { const next = String(v || ''); if (backendDraft.value !== next) backendDraft.value = next }, { immediate: true })
watch(deviceId, (v) => { const next = String(v || '0'); if (deviceDraft.value !== next) deviceDraft.value = next }, { immediate: true })
watch(objective, (v) => { const next = String(v || 'Sharpe'); if (objectiveDraft.value !== next) objectiveDraft.value = next }, { immediate: true })
watch(mode, (v) => { const next = String(v || 'Grid'); if (modeDraft.value !== next) modeDraft.value = next }, { immediate: true })
watch(fastStart, (v) => { const next = String(v || ''); if (fastStartDraft.value !== next) fastStartDraft.value = next }, { immediate: true })
watch(fastEnd, (v) => { const next = String(v || ''); if (fastEndDraft.value !== next) fastEndDraft.value = next }, { immediate: true })
watch(fastStep, (v) => { const next = String(v || ''); if (fastStepDraft.value !== next) fastStepDraft.value = next }, { immediate: true })
watch(slowStart, (v) => { const next = String(v || ''); if (slowStartDraft.value !== next) slowStartDraft.value = next }, { immediate: true })
watch(slowEnd, (v) => { const next = String(v || ''); if (slowEndDraft.value !== next) slowEndDraft.value = next }, { immediate: true })
watch(slowStep, (v) => { const next = String(v || ''); if (slowStepDraft.value !== next) slowStepDraft.value = next }, { immediate: true })
watch(fastMaType, (v) => { const next = String(v || ''); if (fastTypeDraft.value !== next) fastTypeDraft.value = next }, { immediate: true })
watch(slowMaType, (v) => { const next = String(v || ''); if (slowTypeDraft.value !== next) slowTypeDraft.value = next }, { immediate: true })

function setInputValue(id: string, nextValue: string): void {
  optimizeStore.setInputField(id, nextValue)
}

function setSelectValue(id: string, nextValue: string): void {
  optimizeStore.setSelectField(id, nextValue)
}

function runOptimizationAction(): void {
  optimizeStore.runOptimization()
}

function runValidationAction(): void {
  optimizeStore.runValidation()
}

function resolveMount(): void {
  const panel = document.querySelector('[data-vue-panel="opt"]') as HTMLElement | null
  const mount = document.getElementById('optVueShellMount')
  if (!panel || !mount) return
  panelRoot.value = panel
  panel.dataset.vueOpt = '1'
  mountReady.value = true
}

onMounted(async () => {
  await nextTick()
  resolveMount()
  // Ask legacy MA-params renderer to repaint once Vue hosts are mounted.
  window.setTimeout(() => {
    const currentType = String(fastTypeDraft.value || fastMaType.value || '').trim()
    if (currentType) setSelectValue('fastMaType', currentType)
  }, 0)
})

onBeforeUnmount(() => {
  if (panelRoot.value) {
    delete panelRoot.value.dataset.vueOpt
    panelRoot.value = null
  }
})
</script>

<template>
  <Teleport v-if="mountReady" to="#optVueShellMount">
    <section class="optPanelVueRoot optVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="title">Optimize</div>
        <div class="meta mono">{{ stamp }}</div>
      </header>

      <section class="pane">
        <div class="gridTwo">
          <div>
            <label for="vueStrategyKind">Strategy</label>
            <select
              id="vueStrategyKind"
              :value="strategyDraft"
              @change="strategyDraft = String(($event.target as HTMLSelectElement).value || ''); setSelectValue('strategyKind', strategyDraft)"
            >
              <option
                v-for="option in strategyOptionItems"
                :key="`strategy_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
            <div class="muted">{{ strategyHint }}</div>
          </div>

          <div>
            <label for="vuePresetProfile">Preset</label>
            <div class="inputWithAction">
              <select
                id="vuePresetProfile"
                :value="presetDraft"
                @change="presetDraft = String(($event.target as HTMLSelectElement).value || ''); setSelectValue('presetProfile', presetDraft)"
              >
                <option
                  v-for="option in presetOptionItems"
                  :key="`preset_${option.value}_${option.label}`"
                  :value="option.value"
                  :disabled="option.disabled"
                >
                  {{ option.label }}
                </option>
              </select>
              <button type="button" :disabled="!canApplyPreset" @click="optimizeStore.applyPreset()">Apply preset</button>
            </div>
          </div>
        </div>

        <div class="gridTwo">
          <div>
            <label for="vueObjective">Objective</label>
            <select
              id="vueObjective"
              :value="objectiveDraft"
              @change="objectiveDraft = String(($event.target as HTMLSelectElement).value || 'Sharpe'); setSelectValue('objective', objectiveDraft)"
            >
              <option value="Sharpe">Sharpe</option>
              <option value="Sortino">Sortino</option>
              <option value="Calmar">Calmar</option>
              <option value="Omega">Omega</option>
              <option value="Pnl">PnL</option>
              <option value="MaxDrawdown">Min drawdown</option>
            </select>
          </div>
          <div>
            <label for="vueMode">Mode</label>
            <select
              id="vueMode"
              :value="modeDraft"
              @change="modeDraft = String(($event.target as HTMLSelectElement).value || 'Grid'); setSelectValue('mode', modeDraft)"
            >
              <option value="Grid">Grid</option>
              <option value="CoarseToFine">Coarse-to-Fine</option>
              <option value="GpuVdps">GPU VDPS</option>
              <option value="Auto">Auto</option>
            </select>
          </div>
        </div>
      </section>

      <section class="pane">
        <div class="gridTwo">
          <div>
            <label for="vueBackend">Backend</label>
            <select
              id="vueBackend"
              :value="backendDraft"
              @change="backendDraft = String(($event.target as HTMLSelectElement).value || ''); setSelectValue('backend', backendDraft)"
            >
              <option
                v-for="option in backendOptionItems"
                :key="`backend_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
          <div>
            <label for="vueDevice">GPU device</label>
            <select
              id="vueDevice"
              :value="deviceDraft"
              @change="deviceDraft = String(($event.target as HTMLSelectElement).value || '0'); setSelectValue('deviceId', deviceDraft)"
            >
              <option
                v-for="option in deviceOptionItems"
                :key="`device_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
        </div>
        <div class="actions">
          <button type="button" :disabled="!canPresetKernel" @click="optimizeStore.presetKernel()">Preset GPU kernel</button>
          <button type="button" :disabled="!canRefreshGpuStatus" @click="optimizeStore.refreshGpuStatus()">Refresh GPU status</button>
          <button type="button" :disabled="!canParityCheck" @click="optimizeStore.runGpuParityCheck()">Parity check</button>
        </div>
        <div class="muted mono">{{ gpuStatus }}</div>
        <div class="muted mono">{{ gpuParityMeta }}</div>
      </section>

      <section class="pane">
        <div class="split">
          <div>
            <h3>Fast range</h3>
            <div class="rowThree">
              <div>
                <label for="vueFastStart">Start</label>
                <input
                  id="vueFastStart"
                  type="number"
                  min="1"
                  step="1"
                  :value="fastStartDraft"
                  @input="fastStartDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('fastStart', fastStartDraft)"
                />
              </div>
              <div>
                <label for="vueFastEnd">End</label>
                <input
                  id="vueFastEnd"
                  type="number"
                  min="1"
                  step="1"
                  :value="fastEndDraft"
                  @input="fastEndDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('fastEnd', fastEndDraft)"
                />
              </div>
              <div>
                <label for="vueFastStep">Step</label>
                <input
                  id="vueFastStep"
                  type="number"
                  min="0"
                  step="1"
                  :value="fastStepDraft"
                  @input="fastStepDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('fastStep', fastStepDraft)"
                />
              </div>
            </div>
            <label for="vueFastType">Fast MA type</label>
            <select
              id="vueFastType"
              :value="fastTypeDraft"
              @change="fastTypeDraft = String(($event.target as HTMLSelectElement).value || ''); setSelectValue('fastMaType', fastTypeDraft)"
            >
              <option
                v-for="option in fastTypeOptionItems"
                :key="`fasttype_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>

          <div>
            <h3>Slow range</h3>
            <div class="rowThree">
              <div>
                <label for="vueSlowStart">Start</label>
                <input
                  id="vueSlowStart"
                  type="number"
                  min="1"
                  step="1"
                  :value="slowStartDraft"
                  @input="slowStartDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('slowStart', slowStartDraft)"
                />
              </div>
              <div>
                <label for="vueSlowEnd">End</label>
                <input
                  id="vueSlowEnd"
                  type="number"
                  min="1"
                  step="1"
                  :value="slowEndDraft"
                  @input="slowEndDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('slowEnd', slowEndDraft)"
                />
              </div>
              <div>
                <label for="vueSlowStep">Step</label>
                <input
                  id="vueSlowStep"
                  type="number"
                  min="0"
                  step="1"
                  :value="slowStepDraft"
                  @input="slowStepDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('slowStep', slowStepDraft)"
                />
              </div>
            </div>
            <label for="vueSlowType">Slow MA type</label>
            <select
              id="vueSlowType"
              :value="slowTypeDraft"
              @change="slowTypeDraft = String(($event.target as HTMLSelectElement).value || ''); setSelectValue('slowMaType', slowTypeDraft)"
            >
              <option
                v-for="option in slowTypeOptionItems"
                :key="`slowtype_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
        </div>
        <div class="muted mono">{{ pairsPreview }}</div>
      </section>

      <section class="pane">
        <h3>Run controls</h3>
        <div class="actions">
          <button type="button" class="primary" :disabled="runDisabled" @click="runOptimizationAction">Run optimization</button>
          <button type="button" :disabled="validateDisabled" @click="runValidationAction">Run validation</button>
        </div>
        <div class="muted mono">{{ progressMeta || '—' }}</div>
        <div class="muted mono">{{ statusLine }}</div>
      </section>

      <section class="pane">
        <h3>MA parameters</h3>
        <div class="split">
          <div id="fastMaParamsWrapVue" style="display: none;">
            <div class="muted" style="margin-bottom: 0.35rem;">Fast MA parameters</div>
            <div id="fastMaParamsBodyVue"></div>
          </div>
          <div id="slowMaParamsWrapVue" style="display: none;">
            <div class="muted" style="margin-bottom: 0.35rem;">Slow MA parameters</div>
            <div id="slowMaParamsBodyVue"></div>
          </div>
        </div>
      </section>
    </section>
  </Teleport>
</template>

<style scoped>
.optPanelVueRoot {
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

.gridTwo {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.7rem;
}

.gridTwo + .gridTwo {
  margin-top: 0.7rem;
}

.split {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.8rem;
}

.rowThree {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  gap: 0.55rem;
}

.inputWithAction {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.45rem;
}

.actions {
  margin-top: 0.6rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.actions button {
  padding: 0.42rem 0.6rem;
  font-size: 0.76rem;
}

.actions .primary {
  border-color: color-mix(in oklab, var(--accent) 70%, var(--border));
}

.muted {
  color: var(--muted);
  font-size: 0.8rem;
  margin-top: 0.4rem;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 980px) {
  .gridTwo,
  .split,
  .rowThree,
  .inputWithAction {
    grid-template-columns: 1fr;
  }
}
</style>

<style>
[data-vue-panel='opt'][data-vue-opt='1'] > :not(#optVueShellMount) {
  display: none !important;
}
</style>



