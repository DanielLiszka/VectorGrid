<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { RotateCcw } from '@lucide/vue'
import {
  BufferGeometry,
  Color,
  ConeGeometry,
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
import {
  observeDevicePixelRatioChanges,
  resolveDevicePixelRatio,
} from '../../visualizations/devicePixelRatioObserver'
import { resolveLightweightChartTheme } from '../../visualizations/chartTheme'
import {
  heatmapVisualizationTokens,
  observeHeatmapTheme,
  type HeatmapVisualizationTokens,
} from '../../visualizations/heatmapTheme'
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
const MAX_SURFACE_PIXEL_RATIO = 2
const WEBGL_CONTEXT_LOST_MESSAGE =
  '3D slices temporarily unavailable because the graphics context was lost. Waiting for graphics recovery.'
const viewportRef = ref<HTMLElement | null>(null)
const tooltipRef = ref<HTMLElement | null>(null)
const legendRef = ref<InstanceType<typeof HeatmapLegend> | null>(null)
const surfaceReady = ref(false)
const renderError = ref('')
const hoverSample = ref<HeatmapSurface3dSample | null>(null)
const keyboardSampleIndex = ref(-1)
const selectedSliceKey = ref('')
const activeTheme = ref(resolveLightweightChartTheme())
const activeTokens = computed(() => heatmapVisualizationTokens(activeTheme.value))
const surfaceThemeStyle = computed(() => ({
  '--heatmap-axis-fast': activeTokens.value.axisFast,
  '--heatmap-axis-slow': activeTokens.value.axisSlow,
}))

const sliceModel = computed(() => props.renderModel?.slice3d ?? null)
const activeSlice = computed(() => {
  const slices = sliceModel.value?.slices ?? []
  return slices.find((slice) => slice.key === selectedSliceKey.value) ?? slices[0] ?? null
})
const surfaceModel = computed(() =>
  props.renderModel && activeSlice.value
    ? buildHeatmapSurface3dModel(props.renderModel, {
        slice: activeSlice.value,
        theme: activeTheme.value,
      })
    : null,
)
const unavailableDomainSummary = computed(() => {
  const surface = surfaceModel.value
  if (!surface || !surface.unavailableCells || !surface.totalCells) return ''
  if (surface.colStride > 1 || surface.rowStride > 1) {
    return `${surface.unavailableCells}/${surface.totalCells} source bins unavailable. Fully empty sampled blocks sit at the muted base; gaps inside partial blocks contribute to coverage muting.`
  }
  return `${surface.unavailableCells}/${surface.totalCells} bins unavailable; shown as muted base cells.`
})
const samplingSummary = computed(() => {
  const surface = surfaceModel.value
  if (!surface || (surface.colStride <= 1 && surface.rowStride <= 1)) return ''
  return `3D display averages available best-in-bin scores into ${surface.sampledBinsFast}x${surface.sampledBinsSlow} source blocks; partially populated blocks are muted.`
})
const axisGuide = computed(() => {
  const renderModel = props.renderModel
  if (!renderModel) return null
  const fastLabel = renderModel.request.fastAxisLabel || 'Fast length'
  const slowLabel = renderModel.request.slowAxisLabel || 'Slow length'
  return {
    fast: `${fastLabel}: ${renderModel.model.fastMin} → ${renderModel.model.fastMax}`,
    slow: `${slowLabel}: ${renderModel.model.slowMin} → ${renderModel.model.slowMax}`,
    fastDescription: renderModel.request.fastAxisDescription,
    slowDescription: renderModel.request.slowAxisDescription,
  }
})
function sampleLines(sample: HeatmapSurface3dSample | null): string[] | null {
  const renderModel = props.renderModel
  if (!renderModel || !sample) return null
  const fastLabel = renderModel.request.fastAxisLabel || 'Fast length'
  const slowLabel = renderModel.request.slowAxisLabel || 'Slow length'
  const scoreLabel = sliceModel.value?.scoreLabel ?? renderModel.displayLabel
  const isBlockMean = sample.blockCellCount > 1
  const coverage = `${sample.finiteCellCount}/${sample.blockCellCount} populated ${sample.blockCellCount === 1 ? 'bin' : 'bins'}`
  const valueLine = isBlockMean
    ? `Block mean ${scoreLabel} ${formatHeatmapDisplay(sample.displayValue, 4)} · ${coverage}`
    : `Best retained ${scoreLabel} ${formatHeatmapDisplay(sample.displayValue, 4)} · ${coverage}`
  const coordinateLine = isBlockMean
    ? `Source grid bins X ${sample.colStart + 1}–${sample.colEnd} · Y ${sample.logicalRowStart + 1}–${sample.logicalRowEnd}`
    : `${fastLabel} ${sample.fast} · ${slowLabel} ${sample.slow}`
  const detailLine = isBlockMean
    ? `Detail opens representative ${fastLabel} ${sample.fast} · ${slowLabel} ${sample.slow}`
    : ''
  return [
    activeSlice.value?.label ?? 'Slice',
    coordinateLine,
    valueLine,
    detailLine,
  ].filter(Boolean)
}
function sampleSummary(sample: HeatmapSurface3dSample | null): string {
  return sampleLines(sample)?.join(' · ') ?? ''
}
const visibleSelectedSummary = computed(() => {
  if (renderError.value) return ''
  const inspectedSample =
    sampleSummary(hoverSample.value) ||
    sampleSummary(surfaceModel.value?.samples[keyboardSampleIndex.value] ?? null)
  return inspectedSample || props.selectedSummary
})
const sliceSummary = computed(() => {
  const model = sliceModel.value
  const slice = activeSlice.value
  if (!model || !slice) {
    return '3D slices require include_all=true; add two retained variants.'
  }
  const index = Math.max(0, model.slices.findIndex((entry) => entry.key === slice.key))
  return `${model.axisLabel}: ${slice.label} (${index + 1}/${model.slices.length}) · ${slice.sourceRowCount} retained rows in this slice · ${model.scoreLabel} ${formatHeatmapDisplay(model.range.min, 4)}–${formatHeatmapDisplay(model.range.max, 4)}`
})

let renderer: WebGLRenderer | null = null
let scene: Object3D | null = null
let camera: PerspectiveCamera | null = null
let surfaceMesh: Mesh | null = null
let keyboardSelectionMarker: Object3D | null = null
let resizeObserver: ResizeObserver | null = null
let disconnectPixelRatioObserver: (() => void) | null = null
let disconnectThemeObserver: (() => void) | null = null
let pointerDownPoint: { x: number; y: number } | null = null
let pointerDragged = false
let orbitDistance = 10.85357084097211
let orbitPolar = 1.1933558658631362
let orbitAzimuth = 0.6871242101414392
const exportReady = computed(() => (
  surfaceReady.value &&
  !renderError.value &&
  Boolean(renderer && sliceModel.value)
))
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

function handleWebglContextLost(event: Event): void {
  event.preventDefault()
  surfaceReady.value = false
  hoverSample.value = null
  hideTooltip()
  renderError.value = WEBGL_CONTEXT_LOST_MESSAGE
}

function handleWebglContextRestored(): void {
  const surface = surfaceModel.value
  renderError.value = ''
  if (!renderer || !camera || !scene || !surface?.samples.length) {
    void nextTick(rebuildScene)
    return
  }
  try {
    resizeRenderer()
    surfaceReady.value = true
  } catch (error) {
    surfaceReady.value = false
    renderError.value = `3D slices unavailable after graphics recovery: ${error instanceof Error ? error.message : 'WebGL failed to render.'}`
  }
}

function addRendererContextListeners(canvas: HTMLCanvasElement): void {
  canvas.addEventListener('webglcontextlost', handleWebglContextLost)
  canvas.addEventListener('webglcontextrestored', handleWebglContextRestored)
}

function removeRendererContextListeners(canvas: HTMLCanvasElement | null | undefined): void {
  canvas?.removeEventListener('webglcontextlost', handleWebglContextLost)
  canvas?.removeEventListener('webglcontextrestored', handleWebglContextRestored)
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

function refreshRendererPixelRatio(pixelRatio: number): void {
  if (!renderer) return
  renderer.setPixelRatio(pixelRatio)
  resizeRenderer()
}

function ensureRenderer(): boolean {
  const viewport = viewportRef.value
  if (!viewport) return false
  if (renderer && camera) return true

  let nextRenderer: WebGLRenderer | null = null
  try {
    nextRenderer = new WebGLRenderer({
      antialias: true,
      alpha: true,
      preserveDrawingBuffer: true,
    })
    nextRenderer.setPixelRatio(resolveDevicePixelRatio(MAX_SURFACE_PIXEL_RATIO))
    nextRenderer.outputColorSpace = SRGBColorSpace
    nextRenderer.setClearColor(0x000000, 0)
    nextRenderer.domElement.id = 'heatmapSurface3DCanvasVue'
    nextRenderer.domElement.className = 'heatmapSurface3dCanvas'
    nextRenderer.domElement.setAttribute('aria-hidden', 'true')
    addRendererContextListeners(nextRenderer.domElement)
    viewport.prepend(nextRenderer.domElement)

    renderer = nextRenderer
    camera = new PerspectiveCamera(42, 1, 0.1, 100)

    if (typeof ResizeObserver !== 'undefined') {
      resizeObserver = new ResizeObserver(resizeRenderer)
      resizeObserver.observe(viewport)
    }
    disconnectPixelRatioObserver = observeDevicePixelRatioChanges(
      refreshRendererPixelRatio,
      { maxPixelRatio: MAX_SURFACE_PIXEL_RATIO },
    )
    renderError.value = ''
    resetCamera()
    return true
  } catch (error) {
    resizeObserver?.disconnect()
    resizeObserver = null
    disconnectPixelRatioObserver?.()
    disconnectPixelRatioObserver = null
    removeRendererContextListeners(nextRenderer?.domElement)
    try {
      nextRenderer?.dispose()
      nextRenderer?.forceContextLoss()
      nextRenderer?.domElement.remove()
    } catch {
      // A partially initialized WebGL renderer may not support normal cleanup.
    }
    renderer = null
    camera = null
    surfaceReady.value = false
    renderError.value = `3D slices unavailable: ${error instanceof Error ? error.message : 'WebGL failed to initialize.'}`
    return false
  }
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
  unavailableColor: Color,
): void {
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
  ], unavailableColor, null, faceSamples)
}

function buildSurfaceMesh(
  renderModel: HeatmapCanvasRenderModel,
  surface: HeatmapSurface3dModel,
  tokens: Readonly<HeatmapVisualizationTokens>,
): Mesh {
  const positions: number[] = []
  const colors: number[] = []
  const faceSamples: SurfaceFaceSample[] = []
  const color = new Color()
  const sideColor = new Color()
  const unavailableColor = new Color(tokens.surfaceUnavailable)
  const partialBlockColor = new Color(tokens.surfacePartial)
  const binsFast = Math.max(1, renderModel.model.binsFast)
  const binsSlow = Math.max(1, renderModel.model.binsSlow)

  for (const block of surface.unavailableBlocks) {
    addUnavailableBlock(
      positions,
      colors,
      block,
      binsFast,
      binsSlow,
      faceSamples,
      unavailableColor,
    )
  }

  for (const sample of surface.samples) {
    color.setStyle(sample.color)
    const coverage = sample.blockCellCount > 0
      ? sample.finiteCellCount / sample.blockCellCount
      : 0
    if (coverage < 1) {
      color.lerp(partialBlockColor, Math.min(0.68, (1 - coverage) * 0.68))
    }
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

function buildGridMesh(tokens: Readonly<HeatmapVisualizationTokens>): Mesh {
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
      color: new Color(tokens.surfaceGrid),
      transparent: true,
      opacity: 0.28,
      depthWrite: false,
      side: DoubleSide,
    }),
  )
}

function addFlatRect(
  positions: number[],
  x0: number,
  z0: number,
  x1: number,
  z1: number,
  y: number,
): void {
  positions.push(
    x0, y, z0,
    x1, y, z0,
    x1, y, z1,
    x0, y, z0,
    x1, y, z1,
    x0, y, z1,
  )
}

function flatMesh(
  positions: number[],
  color: number,
  options: { opacity?: number; depthTest?: boolean } = {},
): Mesh {
  const geometry = new BufferGeometry()
  geometry.setAttribute('position', new Float32BufferAttribute(positions, 3))
  const material = new MeshBasicMaterial({
    color,
    transparent: (options.opacity ?? 1) < 1,
    opacity: options.opacity ?? 1,
    depthTest: options.depthTest ?? true,
    depthWrite: false,
    side: DoubleSide,
  })
  return new Mesh(geometry, material)
}

function buildAxisGuides(tokens: Readonly<HeatmapVisualizationTokens>): Object3D {
  const guides = new Object3D()
  guides.name = 'heatmap-axis-guides'
  const half = SURFACE_WIDTH / 2
  const axisOffset = 0.28
  const axisWidth = 0.055
  const tickLength = 0.22
  const arrowLength = 0.34
  const y = 0.045

  const fast: number[] = []
  const fastZ = half + axisOffset
  addFlatRect(fast, -half, fastZ - axisWidth, half, fastZ + axisWidth, y)
  for (const x of [-half, 0, half]) {
    addFlatRect(fast, x - axisWidth, fastZ - tickLength, x + axisWidth, fastZ + tickLength, y)
  }
  fast.push(
    half, y, fastZ - tickLength,
    half, y, fastZ + tickLength,
    half + arrowLength, y, fastZ,
  )
  const fastMesh = flatMesh(fast, new Color(tokens.axisFast).getHex(), { opacity: 0.96 })
  fastMesh.renderOrder = 4
  guides.add(fastMesh)

  const slow: number[] = []
  const slowX = half + axisOffset
  addFlatRect(slow, slowX - axisWidth, -half, slowX + axisWidth, half, y)
  for (const z of [-half, 0, half]) {
    addFlatRect(slow, slowX - tickLength, z - axisWidth, slowX + tickLength, z + axisWidth, y)
  }
  slow.push(
    slowX - tickLength, y, half,
    slowX + tickLength, y, half,
    slowX, y, half + arrowLength,
  )
  const slowMesh = flatMesh(slow, new Color(tokens.axisSlow).getHex(), { opacity: 0.96 })
  slowMesh.renderOrder = 4
  guides.add(slowMesh)
  return guides
}

function buildKeyboardSelectionMarker(
  renderModel: HeatmapCanvasRenderModel,
  sample: HeatmapSurface3dSample,
  tokens: Readonly<HeatmapVisualizationTokens>,
): Object3D {
  const marker = new Object3D()
  marker.name = 'heatmap-keyboard-selection'
  const binsFast = Math.max(1, renderModel.model.binsFast)
  const binsSlow = Math.max(1, renderModel.model.binsSlow)
  const x0 = surfaceAxis(sample.colStart, binsFast)
  const x1 = surfaceAxis(sample.colEnd, binsFast)
  const z0 = surfaceAxis(sample.logicalRowStart, binsSlow)
  const z1 = surfaceAxis(sample.logicalRowEnd, binsSlow)
  const width = Math.max(0.02, x1 - x0)
  const depth = Math.max(0.02, z1 - z0)
  const top = SURFACE_BASE_Y + sample.normalizedValue * SURFACE_HEIGHT
  const borderWidth = Math.min(0.09, Math.max(0.025, Math.min(width, depth) * 0.2))
  const borderY = top + 0.055
  const border: number[] = []
  addFlatRect(border, x0, z0, x1, z0 + borderWidth, borderY)
  addFlatRect(border, x0, z1 - borderWidth, x1, z1, borderY)
  addFlatRect(border, x0, z0 + borderWidth, x0 + borderWidth, z1 - borderWidth, borderY)
  addFlatRect(border, x1 - borderWidth, z0 + borderWidth, x1, z1 - borderWidth, borderY)
  const borderMesh = flatMesh(
    border,
    new Color(tokens.markerFill).getHex(),
    { depthTest: false },
  )
  borderMesh.renderOrder = 20
  marker.add(borderMesh)

  const fill: number[] = []
  const fillInset = Math.min(borderWidth, Math.min(width, depth) * 0.16)
  addFlatRect(fill, x0 + fillInset, z0 + fillInset, x1 - fillInset, z1 - fillInset, top + 0.04)
  const fillMesh = flatMesh(
    fill,
    new Color(tokens.markerStroke).getHex(),
    { opacity: 0.3, depthTest: false },
  )
  fillMesh.renderOrder = 19
  marker.add(fillMesh)

  const locatorRadius = Math.min(0.2, Math.max(0.075, Math.min(width, depth) * 0.28))
  const locator = new Mesh(
    new ConeGeometry(locatorRadius, 0.34, 4),
    new MeshBasicMaterial({
      color: new Color(tokens.selectionLocator),
      depthTest: false,
      depthWrite: false,
    }),
  )
  locator.name = 'heatmap-keyboard-selection-locator'
  locator.position.set((x0 + x1) / 2, top + 0.29, (z0 + z1) / 2)
  locator.rotation.z = Math.PI
  locator.renderOrder = 21
  marker.add(locator)
  return marker
}

function updateKeyboardSelectionMarker(): void {
  if (keyboardSelectionMarker && scene) {
    scene.remove(keyboardSelectionMarker)
    disposeSceneObjects(keyboardSelectionMarker)
  }
  keyboardSelectionMarker = null
  const renderModel = props.renderModel
  const sample = surfaceModel.value?.samples[keyboardSampleIndex.value] ?? null
  if (!scene || !renderModel || !sample) {
    renderFrame()
    return
  }
  keyboardSelectionMarker = buildKeyboardSelectionMarker(
    renderModel,
    sample,
    activeTokens.value,
  )
  scene.add(keyboardSelectionMarker)
  renderFrame()
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
    keyboardSelectionMarker = null
    renderFrame()
    return
  }
  if (!ensureRenderer()) {
    return
  }

  disposeSceneObjects(scene)
  keyboardSelectionMarker = null
  const nextScene = new Object3D()
  const tokens = activeTokens.value

  const floor = new Mesh(
    new PlaneGeometry(SURFACE_WIDTH, SURFACE_DEPTH),
    new MeshBasicMaterial({
      color: new Color(tokens.surfaceFloor),
      transparent: true,
      opacity: 0.16,
      side: DoubleSide,
    }),
  )
  floor.rotation.x = -Math.PI / 2
  nextScene.add(floor)

  nextScene.add(buildGridMesh(tokens))
  nextScene.add(buildAxisGuides(tokens))

  surfaceMesh = buildSurfaceMesh(renderModel, surface, tokens)
  nextScene.add(surfaceMesh)
  scene = nextScene
  keyboardSelectionMarker = null
  updateKeyboardSelectionMarker()
  surfaceReady.value = true
  legendRef.value?.paint(
    sliceModel.value?.range ?? renderModel.range,
    sliceModel.value?.palette ?? renderModel.palette,
    activeTheme.value,
  )
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
      const nextAzimuth = orbitAzimuth - (dx / viewportHeight) * Math.PI * 2
      const nextPolar = Math.max(
        ORBIT_MIN_POLAR,
        Math.min(ORBIT_MAX_POLAR, orbitPolar + (dy / viewportHeight) * Math.PI),
      )
      pointerDownPoint = { x: event.clientX, y: event.clientY }
      const cameraChanged = (
        Math.abs(nextAzimuth - orbitAzimuth) > 1.0e-6 ||
        Math.abs(nextPolar - orbitPolar) > 1.0e-6
      )
      if (!cameraChanged) return
      orbitAzimuth = nextAzimuth
      orbitPolar = nextPolar
      hoverSample.value = null
      hideTooltip()
      updateCamera()
      event.preventDefault()
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
  if (!surfaceReady.value || (!event.ctrlKey && !event.metaKey)) return
  const delta = Number(event.deltaY)
  if (!Number.isFinite(delta) || delta === 0) return
  const nextDistance = Math.max(
    ORBIT_MIN_DISTANCE,
    Math.min(ORBIT_MAX_DISTANCE, orbitDistance * Math.exp(delta * 0.001)),
  )
  if (Math.abs(nextDistance - orbitDistance) <= 1.0e-6) return
  orbitDistance = nextDistance
  event.preventDefault()
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
    hoverSample.value = null
    hideTooltip()
    const index = keyboardSampleIndex.value >= 0 ? keyboardSampleIndex.value : 0
    keyboardSampleIndex.value = index
    updateKeyboardSelectionMarker()
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
  hoverSample.value = null
  hideTooltip()
  keyboardSampleIndex.value = nextIndex
  updateKeyboardSelectionMarker()
}

function exportPng(fileName = 'heatmap-3d.png'): boolean {
  if (!exportReady.value || !renderer || typeof document === 'undefined') return false
  try {
    renderFrame()
    const a = document.createElement('a')
    a.href = renderer.domElement.toDataURL('image/png')
    a.download = fileName || 'heatmap-3d.png'
    a.style.display = 'none'
    document.body?.appendChild(a)
    a.click()
    a.remove()
    return true
  } catch {
    return false
  }
}

onMounted(() => {
  disconnectThemeObserver = observeHeatmapTheme((theme) => {
    activeTheme.value = theme
    void nextTick(rebuildScene)
  })
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
  disconnectThemeObserver?.()
  disconnectThemeObserver = null
  resizeObserver?.disconnect()
  resizeObserver = null
  disconnectPixelRatioObserver?.()
  disconnectPixelRatioObserver = null
  disposeSceneObjects(scene)
  scene = null
  surfaceMesh = null
  keyboardSelectionMarker = null
  removeRendererContextListeners(renderer?.domElement)
  renderer?.dispose()
  renderer?.forceContextLoss()
  renderer?.domElement.remove()
  renderer = null
  camera = null
})

defineExpose({ exportPng, exportReady, resetCamera })
</script>

<template>
  <div
    id="heatmapSurface3DHostVue"
    class="heatmapSurface3dHost"
    :style="surfaceThemeStyle"
    :data-heatmap-theme="activeTheme"
  >
    <div class="heatmapSurface3dHeader">
      <h3 class="panelPaneTitle">3D Slices</h3>
    </div>
    <div id="heatmap3dEmptyHintVue" v-show="!renderModel || !sliceModel" class="panelStatus" role="status" aria-live="polite">
      {{ sliceSummary }}
    </div>
    <div class="heatmapSurface3dAssist">
      <div id="heatmapSurface3DInstructionsVue" class="panelMuted">
        Choose a slice. On the focused 3D surface, use Arrow keys, Home, or End to move; Enter or Space opens candidate detail. Hold Ctrl/Command while scrolling to zoom. Top results is exact.
      </div>
      <div class="actions">
        <button type="button" @click="$emit('review-top-results')">Review Top results</button>
      </div>
    </div>
    <div id="heatmapSurface3DWrapVue" v-show="!!renderModel && !!sliceModel" class="heatmapSurface3dWrap">
      <div id="heatmapSurface3DMetaVue" class="panelStatus heatmapSurface3dRenderMeta">
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
        :aria-describedby="axisGuide && !renderError
          ? 'heatmapSurface3DInstructionsVue heatmapSurface3DMetaVue heatmapSurface3DAxisGuideVue'
          : 'heatmapSurface3DInstructionsVue heatmapSurface3DMetaVue'"
        :data-render-ready="surfaceReady ? 'true' : 'false'"
        :data-export-ready="exportReady ? 'true' : 'false'"
        :data-keyboard-selected-index="keyboardSampleIndex"
        @pointerdown="handlePointerDown"
        @pointermove="handlePointerMove"
        @pointerup="handlePointerUp"
        @pointercancel="handlePointerUp"
        @pointerleave="handlePointerLeave"
        @wheel="handleSurfaceWheel"
        @click="handleClick"
        @keydown="handleSurfaceKeydown"
      >
        <div
          v-if="renderError"
          id="heatmapSurface3DErrorVue"
          class="panelStatus heatmapSurface3dError"
          role="alert"
        >
          {{ renderError }}
        </div>
        <div
          v-if="axisGuide && !renderError"
          id="heatmapSurface3DAxisGuideVue"
          class="heatmapSurface3dAxisGuide"
          aria-label="Parameter axis orientation"
        >
          <div class="heatmapSurface3dAxisGuideItem heatmapSurface3dAxisGuideFast">
            <span class="heatmapSurface3dAxisGuideLine" aria-hidden="true"></span>
            <span
              :title="axisGuide.fastDescription"
              :aria-label="`${axisGuide.fast}. ${axisGuide.fastDescription}`"
            >
              {{ axisGuide.fast }}
            </span>
          </div>
          <div class="heatmapSurface3dAxisGuideItem heatmapSurface3dAxisGuideSlow">
            <span class="heatmapSurface3dAxisGuideLine" aria-hidden="true"></span>
            <span
              :title="axisGuide.slowDescription"
              :aria-label="`${axisGuide.slow}. ${axisGuide.slowDescription}`"
            >
              {{ axisGuide.slow }}
            </span>
          </div>
          <div class="heatmapSurface3dAxisGuideNote">Colored surface guides point toward increasing values.</div>
        </div>
        <div id="heatmapSurface3DTooltipVue" ref="tooltipRef" class="heatmapSurface3dTooltip" style="display: none;"></div>
      </div>
      <div class="heatmapSurface3dLegendRow">
        <HeatmapLegend ref="legendRef" :note="sliceModel?.scoreLabel || ''" />
        <div>
          <div class="heatmapSurface3dLegendSpacer" aria-hidden="true"></div>
          <div class="panelMuted heatmapSurface3dLegendNote">
            Retained-row best-in-bin scores drive height and color; downsampled blocks show their available-bin mean. Top results remains the exact ranked-candidate path.
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
  touch-action: pan-y pinch-zoom;
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

.heatmapSurface3dAxisGuide {
  position: absolute;
  bottom: 0.75rem;
  left: 0.75rem;
  z-index: 4;
  display: grid;
  gap: 0.2rem;
  max-width: min(24rem, calc(100% - 1.5rem));
  padding: 0.48rem 0.6rem;
  pointer-events: none;
  border: 1px solid color-mix(in oklab, var(--border) 82%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--panel) 86%, transparent);
  box-shadow: var(--shadow-elevated);
  color: var(--text);
  font-family: var(--mono);
  font-size: 0.72rem;
  line-height: 1.25;
}

.heatmapSurface3dAxisGuideItem {
  display: grid;
  grid-template-columns: 2rem minmax(0, 1fr);
  align-items: center;
  gap: 0.4rem;
  font-weight: 700;
}

.heatmapSurface3dAxisGuideLine {
  position: relative;
  display: block;
  height: 3px;
  border-radius: 999px;
  background: currentColor;
}

.heatmapSurface3dAxisGuideLine::after {
  position: absolute;
  top: 50%;
  right: -1px;
  width: 0;
  height: 0;
  content: '';
  border-top: 4px solid transparent;
  border-bottom: 4px solid transparent;
  border-left: 6px solid currentColor;
  transform: translateY(-50%);
}

.heatmapSurface3dAxisGuideFast {
  color: var(--heatmap-axis-fast);
}

.heatmapSurface3dAxisGuideSlow {
  color: var(--heatmap-axis-slow);
}

.heatmapSurface3dAxisGuideNote {
  color: var(--muted-strong);
  font-family: var(--font-sans);
  font-size: 0.68rem;
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
