<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useQueuedInputWriter } from '../composables/useQueuedInputWriter'
import { useDistributedStore } from '../stores/distributedStore'
import { useWorkflowStore } from '../stores/workflowStore'

const distStore = useDistributedStore()
const workflowStore = useWorkflowStore()
const {
  leaderMode,
  leaderModeHint,
  workerStatus,
  workerBindAddr,
  workerToken,
  leaderToken,
  workersText,
  assetsCsvText,
  pathFrom,
  pathTo,
  workersCount,
  assetPathCount,
  tileCsvPath,
  tilesPerWorker,
  outputPreview,
  statusLine,
  isRunning,
  canCancel,
  canStartWorker,
  canStopWorker,
  canWorkerStatus,
  canFillFromPolygon,
  canUseCurrentCsv,
  canRunAssets,
  canRunTiles,
  updatedAtUnixMs,
} = storeToRefs(distStore)
const { controlsTab, workspace, workflow } = storeToRefs(workflowStore)

const mountReady = ref(false)
const panelRoot = ref<HTMLElement | null>(null)

const workerBindDraft = ref('')
const workerTokenDraft = ref('')
const leaderTokenDraft = ref('')
const workersDraft = ref('')
const assetsDraft = ref('')
const pathFromDraft = ref('')
const pathToDraft = ref('')
const tileCsvDraft = ref('')
const tilesPerWorkerDraft = ref('')
const modeDraft = ref('assets')

const visible = computed(() => {
  const ws = String(workspace.value || '').trim()
  const wf = String(workflow.value || '').trim()
  const controls = String(controlsTab.value || '').trim()
  return ws === 'workflow' && wf === 'optimize' && (controls === 'distributed' || isRunning.value)
})

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
})

watch(
  () => [
    String(workerBindAddr.value || ''),
    String(workerToken.value || ''),
    String(leaderToken.value || ''),
    String(workersText.value || ''),
    String(assetsCsvText.value || ''),
    String(pathFrom.value || ''),
    String(pathTo.value || ''),
    String(tileCsvPath.value || ''),
    String(tilesPerWorker.value || ''),
    String(leaderMode.value || 'assets'),
  ] as const,
  ([
    nextWorkerBindAddr,
    nextWorkerToken,
    nextLeaderToken,
    nextWorkersText,
    nextAssetsCsvText,
    nextPathFrom,
    nextPathTo,
    nextTileCsvPath,
    nextTilesPerWorker,
    nextLeaderMode,
  ]) => {
    if (workerBindDraft.value !== nextWorkerBindAddr) workerBindDraft.value = nextWorkerBindAddr
    if (workerTokenDraft.value !== nextWorkerToken) workerTokenDraft.value = nextWorkerToken
    if (leaderTokenDraft.value !== nextLeaderToken) leaderTokenDraft.value = nextLeaderToken
    if (workersDraft.value !== nextWorkersText) workersDraft.value = nextWorkersText
    if (assetsDraft.value !== nextAssetsCsvText) assetsDraft.value = nextAssetsCsvText
    if (pathFromDraft.value !== nextPathFrom) pathFromDraft.value = nextPathFrom
    if (pathToDraft.value !== nextPathTo) pathToDraft.value = nextPathTo
    if (tileCsvDraft.value !== nextTileCsvPath) tileCsvDraft.value = nextTileCsvPath
    if (tilesPerWorkerDraft.value !== nextTilesPerWorker) tilesPerWorkerDraft.value = nextTilesPerWorker
    if (modeDraft.value !== nextLeaderMode) modeDraft.value = nextLeaderMode
  },
  { immediate: true },
)

function setInputValue(id: string, nextValue: string): void {
  distStore.setInputField(id, nextValue)
}

const { queueInputValue, commitInputValue, flushQueuedInputValuesNow } = useQueuedInputWriter(setInputValue)

function setSelectValue(id: string, nextValue: string): void {
  distStore.setSelectField(id, nextValue)
}

function generateTokenAction(): void {
  flushQueuedInputValuesNow()
  distStore.generateToken()
}

function startWorkerAction(): void {
  flushQueuedInputValuesNow()
  distStore.startWorker()
}

function stopWorkerAction(): void {
  flushQueuedInputValuesNow()
  distStore.stopWorker()
}

function workerStatusAction(): void {
  flushQueuedInputValuesNow()
  distStore.refreshWorkerStatus()
}

function fillFromPolygonAction(): void {
  flushQueuedInputValuesNow()
  distStore.fillFromPolygon()
}

function runAssetsAction(): void {
  flushQueuedInputValuesNow()
  distStore.runAssets()
}

function useCurrentCsvAction(): void {
  flushQueuedInputValuesNow()
  distStore.useCurrentCsv()
}

function runTilesAction(): void {
  flushQueuedInputValuesNow()
  distStore.runTiles()
}

function cancelDistributedAction(): void {
  flushQueuedInputValuesNow()
  distStore.cancel()
}

function resolveMount(): void {
  const panel = document.querySelector('[data-vue-panel="distributed"]') as HTMLElement | null
  const mount = document.querySelector('#distributedVueShellMount') as HTMLElement | null
  if (!panel || !mount) return
  panelRoot.value = panel
  panel.dataset.vueDistributed = '1'
  mountReady.value = true
}

onMounted(async () => {
  await nextTick()
  resolveMount()
})

onBeforeUnmount(() => {
  flushQueuedInputValuesNow()
  if (panelRoot.value) {
    delete panelRoot.value.dataset.vueDistributed
    panelRoot.value = null
  }
})
</script>

<template>
  <Teleport v-if="mountReady" to="#distributedVueShellMount">
    <section class="distPanelVueRoot distVueMountHost" v-show="visible">
      <header class="head">
        <div class="title">Distributed Compute</div>
        <div class="meta mono">{{ workersCount }} workers · {{ assetPathCount }} assets · {{ stamp }}</div>
      </header>

      <div class="grid">
        <section class="pane">
          <h3>Worker</h3>
          <label for="vueDistWorkerBind">Bind address</label>
          <input
            id="vueDistWorkerBind"
            type="text"
            :value="workerBindDraft"
            placeholder="127.0.0.1:48991"
            @input="workerBindDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('distWorkerBindAddr', workerBindDraft)"
            @change="commitInputValue('distWorkerBindAddr', workerBindDraft)"
          />

          <label for="vueDistWorkerToken" style="margin-top: 0.55rem;">Worker token</label>
          <div class="inputWithAction">
            <input
              id="vueDistWorkerToken"
              type="text"
              :value="workerTokenDraft"
              placeholder="(required)"
              @input="workerTokenDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('distWorkerToken', workerTokenDraft)"
              @change="commitInputValue('distWorkerToken', workerTokenDraft)"
            />
            <button type="button" @click="generateTokenAction">Generate</button>
          </div>

          <div class="actions">
            <button type="button" :disabled="!canStartWorker" @click="startWorkerAction">Start worker</button>
            <button type="button" :disabled="!canStopWorker" @click="stopWorkerAction">Stop</button>
            <button type="button" :disabled="!canWorkerStatus" @click="workerStatusAction">Status</button>
          </div>
          <div class="muted mono">{{ workerStatus }}</div>
        </section>

        <section class="pane">
          <h3>Leader</h3>
          <label for="vueDistLeaderMode">Mode</label>
          <select
            id="vueDistLeaderMode"
            :value="modeDraft"
            @change="modeDraft = String(($event.target as HTMLSelectElement).value || 'assets'); setSelectValue('distLeaderMode', modeDraft)"
          >
            <option value="assets">assets</option>
            <option value="tiles">tiles</option>
          </select>

          <label for="vueDistLeaderToken" style="margin-top: 0.55rem;">Leader token</label>
          <input
            id="vueDistLeaderToken"
            type="text"
            :value="leaderTokenDraft"
            placeholder="(must match workers)"
            @input="leaderTokenDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('distLeaderToken', leaderTokenDraft)"
            @change="commitInputValue('distLeaderToken', leaderTokenDraft)"
          />

          <label for="vueDistWorkers" style="margin-top: 0.55rem;">Worker URLs (one per line)</label>
          <textarea
            id="vueDistWorkers"
            rows="4"
            :value="workersDraft"
            placeholder="http://192.168.1.50:48991"
            @input="workersDraft = String(($event.target as HTMLTextAreaElement).value || ''); queueInputValue('distLeaderWorkers', workersDraft)"
            @change="commitInputValue('distLeaderWorkers', workersDraft)"
          ></textarea>

          <div class="muted mono">{{ leaderModeHint }}</div>
        </section>
      </div>

      <section class="pane">
        <template v-if="modeDraft === 'assets'">
          <h3>Assets Mode</h3>
          <label for="vueDistAssets">Asset CSV paths (one per line)</label>
          <textarea
            id="vueDistAssets"
            rows="5"
            :value="assetsDraft"
            placeholder="\\\\SERVER\\share\\AAPL.csv"
            @input="assetsDraft = String(($event.target as HTMLTextAreaElement).value || ''); queueInputValue('distAssetsCsvs', assetsDraft)"
            @change="commitInputValue('distAssetsCsvs', assetsDraft)"
          ></textarea>

          <div class="gridTwo">
            <div>
              <label for="vueDistPathFrom">Rewrite from</label>
              <input
                id="vueDistPathFrom"
                type="text"
                :value="pathFromDraft"
                placeholder="C:\\Users\\...\\polygon_cache"
                @input="pathFromDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('distPathFrom', pathFromDraft)"
                @change="commitInputValue('distPathFrom', pathFromDraft)"
              />
            </div>
            <div>
              <label for="vueDistPathTo">Rewrite to</label>
              <input
                id="vueDistPathTo"
                type="text"
                :value="pathToDraft"
                placeholder="\\\\LEADERPC\\polygon_share"
                @input="pathToDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('distPathTo', pathToDraft)"
                @change="commitInputValue('distPathTo', pathToDraft)"
              />
            </div>
          </div>

          <div class="actions">
            <button type="button" :disabled="!canFillFromPolygon" @click="fillFromPolygonAction">Fill from Polygon</button>
            <button type="button" :disabled="!canRunAssets" @click="runAssetsAction">Run assets</button>
          </div>
        </template>

        <template v-else>
          <h3>Tiles Mode</h3>
          <div class="inputWithAction">
            <input
              id="vueDistTileCsv"
              type="text"
              :value="tileCsvDraft"
              placeholder="\\\\SERVER\\share\\dataset.csv"
              @input="tileCsvDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('distTileCsvPath', tileCsvDraft)"
              @change="commitInputValue('distTileCsvPath', tileCsvDraft)"
            />
            <button type="button" :disabled="!canUseCurrentCsv" @click="useCurrentCsvAction">Use current</button>
          </div>
          <label for="vueDistTilesPerWorker" style="margin-top: 0.55rem;">Tiles per worker</label>
          <input
            id="vueDistTilesPerWorker"
            type="number"
            min="1"
            step="1"
            :value="tilesPerWorkerDraft"
            @input="tilesPerWorkerDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('distTilesPerWorker', tilesPerWorkerDraft)"
            @change="commitInputValue('distTilesPerWorker', tilesPerWorkerDraft)"
          />
          <div class="actions">
            <button type="button" :disabled="!canRunTiles" @click="runTilesAction">Run tiles</button>
          </div>
        </template>
      </section>

      <section class="pane">
        <h3>Output</h3>
        <div class="actions">
          <button type="button" class="danger" :disabled="!canCancel" @click="cancelDistributedAction">Cancel distributed run</button>
        </div>
        <pre class="mono output">{{ outputPreview }}</pre>
        <div class="muted mono">{{ statusLine }}</div>
      </section>
    </section>
  </Teleport>
</template>

<style scoped>
.distPanelVueRoot {
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

.grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.8rem;
}

.gridTwo {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.7rem;
  margin-top: 0.55rem;
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

.inputWithAction {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 0.45rem;
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

.actions button.danger {
  border-color: var(--danger);
  color: var(--danger);
}

.output {
  margin-top: 0.55rem;
  max-height: 220px;
  overflow: auto;
}

.mono {
  font-family: var(--mono);
}

.muted {
  color: var(--muted);
  font-size: 0.8rem;
}

@media (max-width: 980px) {
  .grid,
  .gridTwo {
    grid-template-columns: 1fr;
  }
}
</style>

<style>
[data-vue-panel='distributed'][data-vue-distributed='1'] > :not(#distributedVueShellMount) {
  display: none !important;
}
</style>


