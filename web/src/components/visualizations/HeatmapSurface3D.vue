<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { RotateCcw } from '@lucide/vue'
import {
  BufferGeometry,
  Color,
  DirectionalLight,
  DoubleSide,
  Float32BufferAttribute,
  GridHelper,
  HemisphereLight,
  Mesh,
  MeshStandardMaterial,
  PerspectiveCamera,
  PlaneGeometry,
  Raycaster,
  Scene,
  SRGBColorSpace,
  Vector2,
  WebGLRenderer,
  type Material,
  type Texture,
} from 'three'
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js'
import {
  buildHeatmapCellParams,
  formatHeatmapDisplay,
  type HeatmapCanvasRenderModel,
  type HeatmapCellParams,
} from '../../visualizations/heatmapCanvasModel'
import {
  buildHeatmapSurface3dModel,
  type HeatmapSurface3dModel,
  type HeatmapSurface3dSample,
  type HeatmapSurface3dUnavailableBlock,
} from '../../visualizations/heatmapSurface3dModel'
import AppIconButton from '../ui/AppIconButton.vue'
import HeatmapLegend from './HeatmapLegend.vue'

const props = withDefaults(
  defineProps<{
    ariaLabel: string
    renderModel?: HeatmapCanvasRenderModel | null
    selectedSummary?: string
  }>(),
  {
    renderModel: null,
    selectedSummary: '',
  },
)

const emit = defineEmits<{
  'review-top-results': []
  'request-candidate-detail': [params: HeatmapCellParams]
}>()

const SURFACE_WIDTH = 10
const SURFACE_DEPTH = 10
const SURFACE_HEIGHT = 2.8
const SURFACE_BASE_Y = 0.08

const viewportRef = ref<HTMLElement | null>(null)
const tooltipRef = ref<HTMLElement | null>(null)
const legendRef = ref<InstanceType<typeof HeatmapLegend> | null>(null)
const surfaceReady = ref(false)
const renderError = ref('')
const hoverSample = ref<HeatmapSurface3dSample | null>(null)
const selectedSliceKey = ref('')

const sliceModel = computed(() => props.renderModel?.slice3d ?? null)
const activeSlice = computed(() => {
  const slices = sliceModel.value?.slices ?? []
  if (!slices.length) return null
  return slices.find((slice) => slice.key === selectedSliceKey.value) ?? slices[0] ?? null
})
const activeSliceIndex = computed(() => {
  const slices = sliceModel.value?.slices ?? []
  const idx = slices.findIndex((slice) => slice.key === activeSlice.value?.key)
  return idx >= 0 ? idx : 0
})
const surfaceModel = computed(() =>
  props.renderModel && activeSlice.value
    ? buildHeatmapSurface3dModel(props.renderModel, { slice: activeSlice.value })
    : null,
)
const unavailableDomainSummary = computed(() => {
  const surface = surfaceModel.value
  if (!surface || !surface.unavailableCells || !surface.totalCells) return ''
  const verb = surface.unavailableCells === 1 ? 'is' : 'are'
  return `${surface.unavailableCells} of ${surface.totalCells} bins ${verb} unavailable in this retained slice and shown as the muted base plane.`
})
const samplingSummary = computed(() => {
  const surface = surfaceModel.value
  if (!surface || (surface.colStride <= 1 && surface.rowStride <= 1)) return ''
  return `3D slice sampled to ${surface.sampledBinsFast}x${surface.sampledBinsSlow} blocks for interactive rendering.`
})
const hoverSummary = computed(() => {
  const renderModel = props.renderModel
  const sample = hoverSample.value
  if (!renderModel || !sample) return ''
  const fastLabel = renderModel.request.fastMaType.toUpperCase() || 'FAST'
  const slowLabel = renderModel.request.slowMaType.toUpperCase() || 'SLOW'
  return `${activeSlice.value?.label ?? 'Slice'} · ${fastLabel} ${sample.fast} · ${slowLabel} ${sample.slow} · ${sliceModel.value?.scoreLabel ?? renderModel.displayLabel} ${formatHeatmapDisplay(sample.displayValue, 4)}`
})
const visibleSelectedSummary = computed(() => hoverSummary.value || props.selectedSummary)
const sliceSummary = computed(() => {
  const model = sliceModel.value
  const slice = activeSlice.value
  if (!model || !slice) {
    return '3D slices require include_all=true and at least two retained MA type or parameter slices.'
  }
  return `${model.axisLabel}: ${slice.label} (${activeSliceIndex.value + 1} of ${model.slices.length}) · ${model.retainedSliceRowCount} retained rows across slices · ${model.scoreLabel} range ${formatHeatmapDisplay(model.range.min, 4)} to ${formatHeatmapDisplay(model.range.max, 4)}`
})

let renderer: WebGLRenderer | null = null
let scene: Scene | null = null
let camera: PerspectiveCamera | null = null
let controls: OrbitControls | null = null
let surfaceMesh: Mesh | null = null
let resizeObserver: ResizeObserver | null = null
let animationFrame: number | null = null
let pointerDownPoint: { x: number; y: number } | null = null
let pointerDragged = false
type SurfaceFaceSample = HeatmapSurface3dSample | null

const raycaster = new Raycaster()
const pointer = new Vector2()

function disposeMaterial(material: Material | Material[]): void {
  const materials = Array.isArray(material) ? material : [material]
  for (const entry of materials) {
    const maybeMap = (entry as Material & { map?: Texture | null }).map
    maybeMap?.dispose()
    entry.dispose()
  }
}

function disposeSceneObjects(nextScene: Scene | null): void {
  nextScene?.traverse((object) => {
    const mesh = object as Mesh
    mesh.geometry?.dispose?.()
    if (mesh.material) disposeMaterial(mesh.material)
  })
}

function stopAnimation(): void {
  if (animationFrame != null) {
    window.cancelAnimationFrame(animationFrame)
    animationFrame = null
  }
}

function renderFrame(): void {
  if (!renderer || !scene || !camera) return
  renderer.render(scene, camera)
}

function startAnimation(): void {
  if (animationFrame != null) return
  const tick = () => {
    controls?.update()
    renderFrame()
    animationFrame = window.requestAnimationFrame(tick)
  }
  animationFrame = window.requestAnimationFrame(tick)
}

function resizeRenderer(): void {
  const viewport = viewportRef.value
  if (!viewport || !renderer || !camera) return
  const rect = viewport.getBoundingClientRect()
  const width = Math.max(320, Math.round(rect.width || viewport.clientWidth || 720))
  const height = Math.max(320, Math.round(rect.height || viewport.clientHeight || 520))
  renderer.setSize(width, height, false)
  camera.aspect = width / height
  camera.updateProjectionMatrix()
  renderFrame()
}

function ensureRenderer(): boolean {
  const viewport = viewportRef.value
  if (!viewport) return false
  if (renderer && camera && controls) return true

  try {
    renderer = new WebGLRenderer({
      antialias: true,
      alpha: true,
      preserveDrawingBuffer: true,
    })
  } catch (error) {
    renderError.value = `3D slices unavailable: ${error instanceof Error ? error.message : 'WebGL failed to initialize.'}`
    return false
  }

  renderError.value = ''
  renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1))
  renderer.outputColorSpace = SRGBColorSpace
  renderer.setClearColor(0x000000, 0)
  renderer.domElement.id = 'heatmapSurface3DCanvasVue'
  renderer.domElement.className = 'heatmapSurface3dCanvas'
  renderer.domElement.setAttribute('aria-hidden', 'true')
  viewport.prepend(renderer.domElement)

  camera = new PerspectiveCamera(42, 1, 0.1, 100)
  controls = new OrbitControls(camera, renderer.domElement)
  controls.enableDamping = true
  controls.dampingFactor = 0.08
  controls.minDistance = 5
  controls.maxDistance = 18
  controls.maxPolarAngle = Math.PI * 0.48
  controls.target.set(0, 0.8, 0)

  resizeObserver = new ResizeObserver(resizeRenderer)
  resizeObserver.observe(viewport)
  resetCamera()
  return true
}

function surfaceX(col: number, binsFast: number): number {
  return (col / Math.max(1, binsFast) - 0.5) * SURFACE_WIDTH
}

function surfaceZ(logicalRow: number, binsSlow: number): number {
  return (logicalRow / Math.max(1, binsSlow) - 0.5) * SURFACE_DEPTH
}

function surfaceY(normalizedValue: number): number {
  return SURFACE_BASE_Y + normalizedValue * SURFACE_HEIGHT
}

function addVertex(
  positions: number[],
  colors: number[],
  x: number,
  y: number,
  z: number,
  color: Color,
): void {
  positions.push(x, y, z)
  colors.push(color.r, color.g, color.b)
}

function addQuad(
  positions: number[],
  colors: number[],
  points: Array<[number, number, number]>,
  color: Color,
  sample: SurfaceFaceSample,
  faceSamples: SurfaceFaceSample[],
): void {
  const [a, b, c, d] = points
  if (!a || !b || !c || !d) return
  addVertex(positions, colors, a[0], a[1], a[2], color)
  addVertex(positions, colors, b[0], b[1], b[2], color)
  addVertex(positions, colors, c[0], c[1], c[2], color)
  addVertex(positions, colors, a[0], a[1], a[2], color)
  addVertex(positions, colors, c[0], c[1], c[2], color)
  addVertex(positions, colors, d[0], d[1], d[2], color)
  faceSamples.push(sample, sample)
}

function addUnavailableBlock(
  positions: number[],
  colors: number[],
  block: HeatmapSurface3dUnavailableBlock,
  binsFast: number,
  binsSlow: number,
  faceSamples: SurfaceFaceSample[],
): void {
  const mutedColor = new Color(0xb7c4d3)
  const x0 = surfaceX(block.colStart, binsFast)
  const x1 = surfaceX(block.colEnd, binsFast)
  const z0 = surfaceZ(block.logicalRowStart, binsSlow)
  const z1 = surfaceZ(block.logicalRowEnd, binsSlow)
  const y = 0.026
  addQuad(positions, colors, [
    [x0, y, z0],
    [x1, y, z0],
    [x1, y, z1],
    [x0, y, z1],
  ], mutedColor, null, faceSamples)
}

function buildSurfaceMesh(renderModel: HeatmapCanvasRenderModel, surface: HeatmapSurface3dModel): Mesh {
  const positions: number[] = []
  const colors: number[] = []
  const faceSamples: SurfaceFaceSample[] = []
  const color = new Color()
  const sideColor = new Color()
  const binsFast = Math.max(1, renderModel.model.binsFast)
  const binsSlow = Math.max(1, renderModel.model.binsSlow)

  for (const block of surface.unavailableBlocks) {
    addUnavailableBlock(positions, colors, block, binsFast, binsSlow, faceSamples)
  }

  for (const sample of surface.samples) {
    color.setStyle(sample.color)
    sideColor.copy(color).multiplyScalar(0.68)
    const x0 = surfaceX(sample.colStart, binsFast)
    const x1 = surfaceX(sample.colEnd, binsFast)
    const z0 = surfaceZ(sample.logicalRowStart, binsSlow)
    const z1 = surfaceZ(sample.logicalRowEnd, binsSlow)
    const y = surfaceY(sample.normalizedValue)
    const baseY = 0.015
    addQuad(positions, colors, [
      [x0, y, z0],
      [x1, y, z0],
      [x1, y, z1],
      [x0, y, z1],
    ], color, sample, faceSamples)
    addQuad(positions, colors, [
      [x0, baseY, z0],
      [x1, baseY, z0],
      [x1, y, z0],
      [x0, y, z0],
    ], sideColor, sample, faceSamples)
    addQuad(positions, colors, [
      [x1, baseY, z0],
      [x1, baseY, z1],
      [x1, y, z1],
      [x1, y, z0],
    ], sideColor, sample, faceSamples)
    addQuad(positions, colors, [
      [x1, baseY, z1],
      [x0, baseY, z1],
      [x0, y, z1],
      [x1, y, z1],
    ], sideColor, sample, faceSamples)
    addQuad(positions, colors, [
      [x0, baseY, z1],
      [x0, baseY, z0],
      [x0, y, z0],
      [x0, y, z1],
    ], sideColor, sample, faceSamples)
  }

  const geometry = new BufferGeometry()
  geometry.setAttribute('position', new Float32BufferAttribute(positions, 3))
  geometry.setAttribute('color', new Float32BufferAttribute(colors, 3))
  geometry.computeVertexNormals()

  const material = new MeshStandardMaterial({
    vertexColors: true,
    roughness: 0.58,
    metalness: 0.04,
    side: DoubleSide,
  })
  const mesh = new Mesh(geometry, material)
  mesh.userData.faceSamples = faceSamples
  return mesh
}

function rebuildScene(): void {
  const renderModel = props.renderModel
  const surface = surfaceModel.value
  surfaceReady.value = false
  hoverSample.value = null
  hideTooltip()
  if (!renderModel || !surface || !surface.samples.length) {
    disposeSceneObjects(scene)
    scene = null
    surfaceMesh = null
    renderFrame()
    return
  }
  if (!ensureRenderer()) return

  disposeSceneObjects(scene)
  const nextScene = new Scene()
  nextScene.add(new HemisphereLight(0xf8fafc, 0x1f2937, 1.7))
  const keyLight = new DirectionalLight(0xffffff, 2.2)
  keyLight.position.set(5, 8, 7)
  nextScene.add(keyLight)
  const fillLight = new DirectionalLight(0x60a5fa, 0.9)
  fillLight.position.set(-6, 4, -5)
  nextScene.add(fillLight)

  const floor = new Mesh(
    new PlaneGeometry(SURFACE_WIDTH, SURFACE_DEPTH),
    new MeshStandardMaterial({
      color: 0x64748b,
      transparent: true,
      opacity: 0.16,
      roughness: 0.82,
      side: DoubleSide,
    }),
  )
  floor.rotation.x = -Math.PI / 2
  floor.position.y = 0
  nextScene.add(floor)

  const grid = new GridHelper(SURFACE_WIDTH, 10, 0x94a3b8, 0xcbd5e1)
  grid.material.transparent = true
  grid.material.opacity = 0.28
  grid.position.y = 0.012
  nextScene.add(grid)

  surfaceMesh = buildSurfaceMesh(renderModel, surface)
  nextScene.add(surfaceMesh)
  scene = nextScene
  surfaceReady.value = true
  legendRef.value?.paint(sliceModel.value?.range ?? renderModel.range, sliceModel.value?.palette ?? renderModel.palette)
  resetCamera()
  resizeRenderer()
  startAnimation()
}

function resetCamera(): void {
  if (!camera || !controls) return
  camera.position.set(6.4, 4.9, 7.8)
  controls.target.set(0, 0.9, 0)
  controls.update()
  renderFrame()
}

function pickSample(clientX: number, clientY: number): HeatmapSurface3dSample | null {
  if (!renderer || !camera || !surfaceMesh) return null
  const rect = renderer.domElement.getBoundingClientRect()
  if (!rect.width || !rect.height) return null
  pointer.x = ((clientX - rect.left) / rect.width) * 2 - 1
  pointer.y = -((clientY - rect.top) / rect.height) * 2 + 1
  raycaster.setFromCamera(pointer, camera)
  const hit = raycaster.intersectObject(surfaceMesh, false)[0]
  if (!hit || hit.faceIndex == null) return null
  const faceSamples = surfaceMesh.userData.faceSamples as SurfaceFaceSample[] | undefined
  return faceSamples?.[hit.faceIndex] ?? null
}

function tooltipText(sample: HeatmapSurface3dSample): string {
  const renderModel = props.renderModel
  if (!renderModel) return ''
  const fastLabel = renderModel.request.fastMaType.toUpperCase() || 'FAST'
  const slowLabel = renderModel.request.slowMaType.toUpperCase() || 'SLOW'
  return `${activeSlice.value?.label ?? 'Slice'}\n${fastLabel} ${sample.fast} · ${slowLabel} ${sample.slow}\n${sliceModel.value?.scoreLabel ?? renderModel.displayLabel} ${formatHeatmapDisplay(sample.displayValue, 4)}`
}

function hideTooltip(): void {
  const tooltip = tooltipRef.value
  if (tooltip) tooltip.style.display = 'none'
}

function showTooltip(event: PointerEvent, sample: HeatmapSurface3dSample): void {
  const viewport = viewportRef.value
  const tooltip = tooltipRef.value
  if (!viewport || !tooltip) return
  const rect = viewport.getBoundingClientRect()
  const left = Math.max(12, Math.min(rect.width - 220, event.clientX - rect.left + 14))
  const top = Math.max(12, Math.min(rect.height - 74, event.clientY - rect.top + 14))
  tooltip.textContent = tooltipText(sample)
  tooltip.style.left = `${left}px`
  tooltip.style.top = `${top}px`
  tooltip.style.display = 'block'
}

function handlePointerDown(event: PointerEvent): void {
  pointerDownPoint = { x: event.clientX, y: event.clientY }
  pointerDragged = false
}

function handlePointerMove(event: PointerEvent): void {
  if (pointerDownPoint) {
    const dx = event.clientX - pointerDownPoint.x
    const dy = event.clientY - pointerDownPoint.y
    if (Math.hypot(dx, dy) > 5) pointerDragged = true
  }
  const sample = pickSample(event.clientX, event.clientY)
  hoverSample.value = sample
  if (sample) {
    showTooltip(event, sample)
  } else {
    hideTooltip()
  }
}

function handlePointerLeave(): void {
  hoverSample.value = null
  pointerDownPoint = null
  pointerDragged = false
  hideTooltip()
}

function handleClick(event: PointerEvent): void {
  const renderModel = props.renderModel
  if (!renderModel || pointerDragged) return
  const sample = pickSample(event.clientX, event.clientY)
  if (!sample) return
  const params = sample.params ?? buildHeatmapCellParams(renderModel, sample.representativeCell)
  if (params) emit('request-candidate-detail', params)
}

function exportPng(fileName = 'heatmap-3d.png'): boolean {
  if (!renderer || !sliceModel.value || typeof document === 'undefined') return false
  renderFrame()
  const a = document.createElement('a')
  a.href = renderer.domElement.toDataURL('image/png')
  a.download = fileName || 'heatmap-3d.png'
  a.style.display = 'none'
  document.body?.appendChild(a)
  a.click()
  a.remove()
  return true
}

onMounted(() => {
  void nextTick(rebuildScene)
})

watch(
  () => sliceModel.value?.slices.map((slice) => slice.key).join('|') ?? '',
  () => {
    const slices = sliceModel.value?.slices ?? []
    if (!slices.some((slice) => slice.key === selectedSliceKey.value)) {
      selectedSliceKey.value = slices[0]?.key ?? ''
    }
  },
  { immediate: true },
)

watch(
  () => `${props.renderModel?.signature ?? ''}:${activeSlice.value?.key ?? ''}`,
  () => {
    void nextTick(rebuildScene)
  },
)

onBeforeUnmount(() => {
  stopAnimation()
  resizeObserver?.disconnect()
  resizeObserver = null
  controls?.dispose()
  controls = null
  disposeSceneObjects(scene)
  scene = null
  surfaceMesh = null
  renderer?.dispose()
  renderer?.forceContextLoss()
  renderer?.domElement.remove()
  renderer = null
  camera = null
})

defineExpose({ exportPng, resetCamera })
</script>

<template>
  <div id="heatmapSurface3DHostVue" class="heatmapSurface3dHost">
    <div class="heatmapSurface3dHeader">
      <h3 class="panelPaneTitle">3D Slices</h3>
    </div>
    <div id="heatmap3dEmptyHintVue" v-show="!renderModel || !sliceModel" class="panelStatus" role="status" aria-live="polite">
      {{ sliceSummary }}
    </div>
    <div id="heatmapSurface3DWrapVue" v-show="!!renderModel && !!sliceModel" class="heatmapSurface3dWrap">
      <div id="heatmapSurface3DMetaVue" class="panelStatus heatmapSurface3dRenderMeta" role="status" aria-live="polite">
        {{ sliceSummary }}
      </div>
      <div
        v-if="unavailableDomainSummary"
        id="heatmapSurface3DDomainSummaryVue"
        class="panelMuted heatmapSurface3dSummary"
      >
        {{ unavailableDomainSummary }}
      </div>
      <div
        v-if="samplingSummary"
        id="heatmapSurface3DSamplingSummaryVue"
        class="panelMuted heatmapSurface3dSummary"
      >
        {{ samplingSummary }}
      </div>
      <div
        v-if="sliceModel"
        class="heatmapSurface3dSliceBar"
        role="group"
        aria-label="3D slice controls"
      >
        <label for="heatmapSurface3DSliceSelectVue">Slice</label>
        <select
          id="heatmapSurface3DSliceSelectVue"
          v-model="selectedSliceKey"
          class="heatmapSurface3dSliceSelect"
        >
          <option
            v-for="slice in sliceModel.slices"
            :key="slice.key"
            :value="slice.key"
          >
            {{ slice.label }}
          </option>
        </select>
        <AppIconButton
          :icon="RotateCcw"
          label="Reset 3D camera"
          title="Reset camera"
          size="sm"
          variant="subtle"
          @click.stop="resetCamera"
        />
      </div>
      <div
        id="heatmapSurface3DViewportVue"
        ref="viewportRef"
        class="heatmapSurface3dViewport chartSurface"
        role="img"
        :aria-label="ariaLabel"
        :data-render-ready="surfaceReady ? 'true' : 'false'"
        tabindex="0"
        @pointerdown="handlePointerDown"
        @pointermove="handlePointerMove"
        @pointerleave="handlePointerLeave"
        @click="handleClick"
      >
        <div
          v-if="renderError"
          id="heatmapSurface3DErrorVue"
          class="panelStatus heatmapSurface3dError"
          role="status"
        >
          {{ renderError }}
        </div>
        <div id="heatmapSurface3DTooltipVue" ref="tooltipRef" class="heatmapSurface3dTooltip" style="display: none;"></div>
      </div>
      <div class="heatmapSurface3dLegendRow">
        <HeatmapLegend ref="legendRef" :note="sliceModel?.scoreLabel || ''" />
        <div>
          <label>&nbsp;</label>
          <div class="panelMuted heatmapSurface3dLegendNote">
            Height and color use exact retained full-output rows for the selected slice. Click a populated bin to inspect that retained candidate.
          </div>
        </div>
      </div>
    </div>
    <div
      v-if="visibleSelectedSummary"
      id="heatmapSurface3DSelectedSummaryVue"
      class="panelStatus mono panelNumeric"
      role="status"
      aria-live="polite"
    >
      {{ visibleSelectedSummary }}
    </div>
  </div>
</template>

<style scoped>
.heatmapSurface3dHost {
  display: grid;
  gap: 0.65rem;
}

.heatmapSurface3dHeader {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.heatmapSurface3dHeader .panelPaneTitle {
  margin: 0;
}

.heatmapSurface3dSliceBar {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  max-width: 100%;
  padding: 0.35rem 0.45rem;
  border: 1px solid color-mix(in oklab, var(--border) 84%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--panel) 94%, transparent);
  box-shadow: var(--shadow-elevated);
}

.heatmapSurface3dSliceBar label {
  color: var(--text-muted);
  font-size: 0.74rem;
  font-weight: 700;
}

.heatmapSurface3dSliceSelect {
  max-width: min(26rem, 44vw);
  background: color-mix(in oklab, var(--panel) 92%, transparent);
}

.heatmapSurface3dWrap {
  display: grid;
  gap: 0.55rem;
}

.heatmapSurface3dRenderMeta {
  margin: 0;
}

.heatmapSurface3dSummary {
  max-width: 64rem;
}

.chartSurface:focus-visible {
  outline: 2px solid var(--focus-border);
  outline-offset: 2px;
  box-shadow: var(--focus-ring);
}

.heatmapSurface3dViewport {
  position: relative;
  isolation: isolate;
  width: 100%;
  height: clamp(22rem, 42vh, 36rem);
  min-height: 22rem;
  overflow: hidden;
  border: 1px solid color-mix(in oklab, var(--border) 82%, transparent);
  border-radius: 10px;
  background:
    linear-gradient(180deg, rgb(255 255 255 / 0.08), transparent 32%),
    color-mix(in oklab, var(--surface-working) 96%, var(--panel-footer));
  cursor: grab;
  touch-action: none;
}

.heatmapSurface3dViewport:active {
  cursor: grabbing;
}

.heatmapSurface3dViewport :deep(canvas),
.heatmapSurface3dCanvas {
  position: absolute;
  inset: 0;
  display: block;
  width: 100%;
  height: 100%;
}

.heatmapSurface3dError {
  position: absolute;
  inset: 1rem;
  z-index: 2;
  display: grid;
  place-items: center;
  text-align: center;
}

.heatmapSurface3dTooltip {
  position: absolute;
  z-index: 5;
  max-width: min(18rem, calc(100% - 1.5rem));
  white-space: pre-line;
  pointer-events: none;
  border: 1px solid color-mix(in oklab, var(--border) 86%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--panel) 94%, var(--surface-inline));
  box-shadow: var(--shadow-elevated);
  color: var(--text);
  font-family: var(--mono);
  font-size: 0.76rem;
  line-height: 1.25;
  padding: 0.45rem 0.55rem;
}

.heatmapSurface3dLegendRow {
  margin-top: 0.2rem;
  display: grid;
  grid-template-columns: minmax(0, auto) minmax(0, 1fr);
  gap: 0.75rem;
  align-items: start;
}

.heatmapSurface3dLegendNote {
  margin-top: 0.5rem;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 760px) {
  .heatmapSurface3dHeader {
    align-items: flex-start;
    flex-direction: column;
  }

  .heatmapSurface3dSliceBar {
    align-items: stretch;
    width: 100%;
  }

  .heatmapSurface3dSliceSelect {
    max-width: none;
    min-width: 0;
  }

  .heatmapSurface3dViewport {
    height: min(30rem, 72vh);
    min-height: 22rem;
  }

  .heatmapSurface3dLegendRow {
    grid-template-columns: 1fr;
  }
}
</style>
