<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { RotateCcw } from '@lucide/vue'
import {
  BufferGeometry,
  Color,
  DoubleSide,
  Float32BufferAttribute,
  Mesh,
  MeshBasicMaterial,
  Object3D,
  PerspectiveCamera,
  PlaneGeometry,
  Ray,
  SRGBColorSpace,
  Vector3,
  WebGLRenderer,
  type Intersection,
  type Raycaster,
} from 'three'
import {
  buildHeatmapCellParams,
  formatHeatmapDisplay,
  type HeatmapCanvasRenderModel,
  type HeatmapCellParams,
} from '../../visualizations/heatmapCanvasModel'
import {
  buildHeatmapSurface3dModel,
  resolveHeatmapSurface3dKeyboardIndex,
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
const ORBIT_TARGET_Y = 0.9
const ORBIT_MIN_DISTANCE = 5
const ORBIT_MAX_DISTANCE = 18
const ORBIT_MIN_POLAR = 0.2
const ORBIT_MAX_POLAR = Math.PI * 0.48
const viewportRef = ref<HTMLElement | null>(null)
const tooltipRef = ref<HTMLElement | null>(null)
const legendRef = ref<InstanceType<typeof HeatmapLegend> | null>(null)
const surfaceReady = ref(false)
const renderError = ref('')
const hoverSample = ref<HeatmapSurface3dSample | null>(null)
const keyboardSampleIndex = ref(-1)
const selectedSliceKey = ref('')

const sliceModel = computed(() => props.renderModel?.slice3d ?? null)
const activeSlice = computed(() => {
  const slices = sliceModel.value?.slices ?? []
  return slices.find((slice) => slice.key === selectedSliceKey.value) ?? slices[0] ?? null
})
const surfaceModel = computed(() =>
  props.renderModel && activeSlice.value
    ? buildHeatmapSurface3dModel(props.renderModel, { slice: activeSlice.value })
    : null,
)
const unavailableDomainSummary = computed(() => {
  const surface = surfaceModel.value
  if (!surface || !surface.unavailableCells || !surface.totalCells) return ''
  return `${surface.unavailableCells}/${surface.totalCells} bins unavailable; shown as muted base cells.`
})
const samplingSummary = computed(() => {
  const surface = surfaceModel.value
  if (!surface || (surface.colStride <= 1 && surface.rowStride <= 1)) return ''
  return `Sampled to ${surface.sampledBinsFast}x${surface.sampledBinsSlow} blocks for responsive 3D interaction.`
})
function sampleLines(sample: HeatmapSurface3dSample | null): string[] | null {
  const renderModel = props.renderModel
  if (!renderModel || !sample) return null
  const fastLabel = renderModel.request.fastMaType.toUpperCase() || 'FAST'
  const slowLabel = renderModel.request.slowMaType.toUpperCase() || 'SLOW'
  return [
    activeSlice.value?.label ?? 'Slice',
    `${fastLabel} ${sample.fast} · ${slowLabel} ${sample.slow}`,
    `${sliceModel.value?.scoreLabel ?? renderModel.displayLabel} ${formatHeatmapDisplay(sample.displayValue, 4)}`,
  ]
}
function sampleSummary(sample: HeatmapSurface3dSample | null): string {
  return sampleLines(sample)?.join(' · ') ?? ''
}
const visibleSelectedSummary = computed(
  () =>
    sampleSummary(hoverSample.value) ||
    sampleSummary(surfaceModel.value?.samples[keyboardSampleIndex.value] ?? null) ||
    props.selectedSummary,
)
const sliceSummary = computed(() => {
  const model = sliceModel.value
  const slice = activeSlice.value
  if (!model || !slice) {
    return '3D slices require include_all=true; add two retained variants.'
  }
  const index = Math.max(0, model.slices.findIndex((entry) => entry.key === slice.key))
  return `${model.axisLabel}: ${slice.label} (${index + 1}/${model.slices.length}) · ${model.retainedSliceRowCount} retained rows · ${model.scoreLabel} ${formatHeatmapDisplay(model.range.min, 4)}–${formatHeatmapDisplay(model.range.max, 4)}`
})

let renderer: WebGLRenderer | null = null
let scene: Object3D | null = null
let camera: PerspectiveCamera | null = null
let surfaceMesh: Mesh | null = null
let resizeObserver: ResizeObserver | null = null
let pointerDownPoint: { x: number; y: number } | null = null
let pointerDragged = false
let orbitDistance: number
let orbitPolar: number
let orbitAzimuth: number
type SurfaceFaceSample = HeatmapSurface3dSample | null

const pointerRayOrigin = new Vector3()
const pointerRayDirection = new Vector3()
const pointerRaycaster = {
  ray: new Ray(),
  near: 0.1,
  far: 100,
} as Raycaster

function disposeSceneObjects(nextScene: Object3D | null): void {
  nextScene?.traverse((object) => {
    const mesh = object as Mesh
    mesh.geometry?.dispose?.()
    if (Array.isArray(mesh.material)) {
      mesh.material.forEach((material) => material.dispose())
    } else {
      mesh.material?.dispose()
    }
  })
}

function renderFrame(): void {
  if (!renderer || !scene || !camera) return
  renderer.render(scene, camera)
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
  if (renderer && camera) return true

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

  if (typeof ResizeObserver !== 'undefined') {
    resizeObserver = new ResizeObserver(resizeRenderer)
    resizeObserver.observe(viewport)
  }
  resetCamera()
  return true
}

function surfaceAxis(index: number, bins: number): number {
  return (index / Math.max(1, bins) - 0.5) * SURFACE_WIDTH
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
  points: [
    [number, number, number],
    [number, number, number],
    [number, number, number],
    [number, number, number],
  ],
  color: Color,
  sample: SurfaceFaceSample,
  faceSamples: SurfaceFaceSample[],
): void {
  const [a, b, c, d] = points
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
  const x0 = surfaceAxis(block.colStart, binsFast)
  const x1 = surfaceAxis(block.colEnd, binsFast)
  const z0 = surfaceAxis(block.logicalRowStart, binsSlow)
  const z1 = surfaceAxis(block.logicalRowEnd, binsSlow)
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
    const x0 = surfaceAxis(sample.colStart, binsFast)
    const x1 = surfaceAxis(sample.colEnd, binsFast)
    const z0 = surfaceAxis(sample.logicalRowStart, binsSlow)
    const z1 = surfaceAxis(sample.logicalRowEnd, binsSlow)
    const y = SURFACE_BASE_Y + sample.normalizedValue * SURFACE_HEIGHT
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

  const material = new MeshBasicMaterial({
    vertexColors: true,
    side: DoubleSide,
  })
  const mesh = new Mesh(geometry, material)
  mesh.userData.faceSamples = faceSamples
  return mesh
}

function buildGridMesh(): Mesh {
  const positions: number[] = []
  const half = SURFACE_WIDTH / 2
  const line = 0.012
  const addRect = (x0: number, z0: number, x1: number, z1: number) => {
    positions.push(
      x0, line, z0,
      x1, line, z0,
      x1, line, z1,
      x0, line, z0,
      x1, line, z1,
      x0, line, z1,
    )
  }
  for (let index = 0; index <= 10; index += 1) {
    const offset = index - half
    addRect(offset - line, -half, offset + line, half)
    addRect(-half, offset - line, half, offset + line)
  }
  const geometry = new BufferGeometry()
  geometry.setAttribute('position', new Float32BufferAttribute(positions, 3))
  return new Mesh(
    geometry,
    new MeshBasicMaterial({
      color: 0xcbd5e1,
      transparent: true,
      opacity: 0.28,
      depthWrite: false,
      side: DoubleSide,
    }),
  )
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
  if (!ensureRenderer()) {
    return
  }

  disposeSceneObjects(scene)
  const nextScene = new Object3D()

  const floor = new Mesh(
    new PlaneGeometry(SURFACE_WIDTH, SURFACE_DEPTH),
    new MeshBasicMaterial({
      color: 0x64748b,
      transparent: true,
      opacity: 0.16,
      side: DoubleSide,
    }),
  )
  floor.rotation.x = -Math.PI / 2
  nextScene.add(floor)

  nextScene.add(buildGridMesh())

  surfaceMesh = buildSurfaceMesh(renderModel, surface)
  nextScene.add(surfaceMesh)
  scene = nextScene
  surfaceReady.value = true
  legendRef.value?.paint(sliceModel.value?.range ?? renderModel.range, sliceModel.value?.palette ?? renderModel.palette)
  resetCamera()
  resizeRenderer()
}

function resetCamera(): void {
  orbitDistance = 10.85357084097211
  orbitPolar = 1.1933558658631362
  orbitAzimuth = 0.6871242101414392
  updateCamera()
}

function updateCamera(): void {
  if (!camera) return
  const horizontalDistance = orbitDistance * Math.sin(orbitPolar)
  camera.position.set(
    horizontalDistance * Math.sin(orbitAzimuth),
    ORBIT_TARGET_Y + orbitDistance * Math.cos(orbitPolar),
    horizontalDistance * Math.cos(orbitAzimuth),
  )
  camera.lookAt(0, ORBIT_TARGET_Y, 0)
  renderFrame()
}

function pickSample(clientX: number, clientY: number): HeatmapSurface3dSample | null {
  if (!renderer || !camera || !surfaceMesh) return null
  const rect = renderer.domElement.getBoundingClientRect()
  if (!rect.width || !rect.height) return null
  camera.updateMatrixWorld()
  pointerRayOrigin.setFromMatrixPosition(camera.matrixWorld)
  pointerRayDirection
    .set(
      ((clientX - rect.left) / rect.width) * 2 - 1,
      -((clientY - rect.top) / rect.height) * 2 + 1,
      0.5,
    )
    .unproject(camera)
    .sub(pointerRayOrigin)
    .normalize()
  pointerRaycaster.ray.set(pointerRayOrigin, pointerRayDirection)
  const hits: Intersection[] = []
  surfaceMesh.raycast(pointerRaycaster, hits)
  hits.sort((lhs, rhs) => lhs.distance - rhs.distance)
  const hit = hits[0]
  if (!hit || hit.faceIndex == null) return null
  const faceSamples = surfaceMesh.userData.faceSamples as SurfaceFaceSample[] | undefined
  return faceSamples?.[hit.faceIndex] ?? null
}

function tooltipText(sample: HeatmapSurface3dSample): string {
  return sampleLines(sample)?.join('\n') ?? ''
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
  if (event.button !== 0) return
  pointerDownPoint = { x: event.clientX, y: event.clientY }
  pointerDragged = false
  const viewport = event.currentTarget as HTMLElement | null
  viewport?.setPointerCapture?.(event.pointerId)
}

function handlePointerMove(event: PointerEvent): void {
  if (pointerDownPoint) {
    const dx = event.clientX - pointerDownPoint.x
    const dy = event.clientY - pointerDownPoint.y
    if (Math.hypot(dx, dy) > 5) pointerDragged = true
    if (pointerDragged) {
      const viewportHeight = Math.max(1, viewportRef.value?.clientHeight || 520)
      orbitAzimuth -= (dx / viewportHeight) * Math.PI * 2
      orbitPolar = Math.max(
        ORBIT_MIN_POLAR,
        Math.min(ORBIT_MAX_POLAR, orbitPolar + (dy / viewportHeight) * Math.PI),
      )
      pointerDownPoint = { x: event.clientX, y: event.clientY }
      hoverSample.value = null
      hideTooltip()
      updateCamera()
      return
    }
  }
  const sample = pickSample(event.clientX, event.clientY)
  hoverSample.value = sample
  if (sample) {
    showTooltip(event, sample)
  } else {
    hideTooltip()
  }
}

function handlePointerUp(event: PointerEvent): void {
  const viewport = event.currentTarget as HTMLElement | null
  if (viewport?.hasPointerCapture?.(event.pointerId)) {
    viewport.releasePointerCapture(event.pointerId)
  }
  pointerDownPoint = null
}

function handlePointerLeave(): void {
  hoverSample.value = null
  pointerDownPoint = null
  pointerDragged = false
  hideTooltip()
}

function handleClick(event: MouseEvent): void {
  const wasDragged = pointerDragged
  pointerDragged = false
  if (wasDragged) return
  const sample = pickSample(event.clientX, event.clientY)
  requestCandidateDetail(sample)
}

function handleSurfaceWheel(event: WheelEvent): void {
  orbitDistance = Math.max(
    ORBIT_MIN_DISTANCE,
    Math.min(ORBIT_MAX_DISTANCE, orbitDistance * Math.exp(event.deltaY * 0.001)),
  )
  updateCamera()
}

function requestCandidateDetail(sample: HeatmapSurface3dSample | null): void {
  const renderModel = props.renderModel
  if (!renderModel || !sample) return
  const params = sample.params ?? buildHeatmapCellParams(renderModel, sample.representativeCell)
  if (params) emit('request-candidate-detail', params)
}

function handleSurfaceKeydown(event: KeyboardEvent): void {
  const samples = surfaceModel.value?.samples ?? []
  if (!samples.length) return
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault()
    const index = keyboardSampleIndex.value >= 0 ? keyboardSampleIndex.value : 0
    keyboardSampleIndex.value = index
    requestCandidateDetail(samples[index] ?? null)
    return
  }
  const nextIndex = resolveHeatmapSurface3dKeyboardIndex(
    samples,
    keyboardSampleIndex.value >= 0 ? keyboardSampleIndex.value : 0,
    event.key,
  )
  if (nextIndex == null) return
  event.preventDefault()
  keyboardSampleIndex.value = nextIndex
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
    keyboardSampleIndex.value = surfaceModel.value?.samples.length ? 0 : -1
    void nextTick(rebuildScene)
  },
)

onBeforeUnmount(() => {
  resizeObserver?.disconnect()
  resizeObserver = null
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
    <div class="heatmapSurface3dAssist">
      <div id="heatmapSurface3DInstructionsVue" class="panelMuted">
        Choose a slice. On the focused 3D surface, use Arrow keys, Home, or End to move; Enter or Space opens candidate detail. Top results is exact.
      </div>
      <div class="actions">
        <button type="button" @click="$emit('review-top-results')">Review Top results</button>
      </div>
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
        role="region"
        :aria-label="ariaLabel"
        :tabindex="renderModel && sliceModel ? 0 : -1"
        aria-describedby="heatmapSurface3DInstructionsVue heatmapSurface3DMetaVue"
        :data-render-ready="surfaceReady ? 'true' : 'false'"
        @pointerdown="handlePointerDown"
        @pointermove="handlePointerMove"
        @pointerup="handlePointerUp"
        @pointercancel="handlePointerUp"
        @pointerleave="handlePointerLeave"
        @wheel.prevent="handleSurfaceWheel"
        @click="handleClick"
        @keydown="handleSurfaceKeydown"
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
          <div class="heatmapSurface3dLegendSpacer" aria-hidden="true"></div>
          <div class="panelMuted heatmapSurface3dLegendNote">
            Exact retained rows drive height and color. Click or use keys to inspect a populated bin.
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

.heatmapSurface3dAssist {
  display: grid;
  gap: 0.35rem;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.actions button {
  padding: 0.42rem 0.6rem;
  font-size: 0.76rem;
}

.heatmapSurface3dSliceBar {
  display: inline-flex;
  flex-wrap: wrap;
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
  color: var(--muted-strong);
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

.heatmapSurface3dLegendSpacer {
  min-height: 1.1rem;
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
