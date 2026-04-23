<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useMediaQuery } from '@vueuse/core'
import VectorAlphaLogo from './VectorAlphaLogo.vue'
import { useRuntimeStore } from '../stores/runtimeStore'

type OverlayStage = 'hidden' | 'intro' | 'active' | 'exiting' | 'dismissed'
type MilestoneState = 'pending' | 'active' | 'done'
type SceneKey = 'signal' | 'topology' | 'convergence' | 'handoff'

type TelemetryCard = {
  label: string
  value: string
  detail: string
}

type ParticleSeed = {
  id: string
  x: number
  y: number
  size: number
  drift: number
  delay: number
  alpha: number
}

type BarSeed = {
  id: string
  base: number
  variance: number
  delay: number
}

type ColumnSeed = {
  id: string
  x: number
  height: number
  delay: number
}

type ShardSeed = {
  id: string
  x: number
  y: number
  rotation: number
  size: number
  delay: number
}

type LatticeNode = {
  id: string
  x: number
  y: number
  emphasis?: boolean
}

type LatticeLink = {
  id: string
  from: string
  to: string
}

const props = withDefaults(
  defineProps<{
    productName?: string
    tagline?: string
  }>(),
  {
    productName: 'VectorGrid',
    tagline: 'Spatial runtime orchestration',
  },
)

const MIN_VISIBLE_MS = 1500
const READY_SETTLE_MS = 420
const ERROR_SETTLE_MS = 660
const EXIT_MS = 760
const ACTIVE_DELAY_MS = 170
const SKIP_UNLOCK_MS = 1150
const FAILSAFE_DISMISS_MS = 12000
const BOOT_PROGRESS_CAP = 0.94
const HOLD_QUERY_VALUE = 'hold'
const HOLD_QUERY_KEY = 'startup_overlay'
const HOLD_STORAGE_KEY = 'VTA_STARTUP_OVERLAY_HOLD'

const PARTICLE_SEEDS: readonly ParticleSeed[] = [
  { id: 'p1', x: 8, y: 18, size: 2.6, drift: 14.2, delay: -1.8, alpha: 0.36 },
  { id: 'p2', x: 14, y: 34, size: 1.9, drift: 17.4, delay: -6.5, alpha: 0.28 },
  { id: 'p3', x: 22, y: 12, size: 2.3, drift: 13.8, delay: -3.4, alpha: 0.42 },
  { id: 'p4', x: 28, y: 62, size: 2.8, drift: 16.8, delay: -7.2, alpha: 0.35 },
  { id: 'p5', x: 35, y: 22, size: 1.7, drift: 18.6, delay: -1.1, alpha: 0.24 },
  { id: 'p6', x: 44, y: 76, size: 2.4, drift: 14.8, delay: -4.7, alpha: 0.31 },
  { id: 'p7', x: 52, y: 15, size: 2.1, drift: 12.9, delay: -2.5, alpha: 0.39 },
  { id: 'p8', x: 59, y: 48, size: 2.9, drift: 19.2, delay: -8.1, alpha: 0.29 },
  { id: 'p9', x: 65, y: 28, size: 1.8, drift: 15.7, delay: -5.6, alpha: 0.26 },
  { id: 'p10', x: 72, y: 12, size: 2.7, drift: 17.8, delay: -2.9, alpha: 0.37 },
  { id: 'p11', x: 78, y: 56, size: 2.2, drift: 13.6, delay: -6.1, alpha: 0.32 },
  { id: 'p12', x: 84, y: 24, size: 2.5, drift: 16.2, delay: -7.5, alpha: 0.34 },
  { id: 'p13', x: 88, y: 72, size: 1.9, drift: 18.9, delay: -4.2, alpha: 0.23 },
  { id: 'p14', x: 93, y: 42, size: 2.8, drift: 14.1, delay: -3.7, alpha: 0.33 },
]

const BAR_SEEDS: readonly BarSeed[] = [
  { id: 'b1', base: 0.18, variance: 0.11, delay: -0.4 },
  { id: 'b2', base: 0.28, variance: 0.08, delay: -1.1 },
  { id: 'b3', base: 0.34, variance: 0.07, delay: -0.8 },
  { id: 'b4', base: 0.22, variance: 0.12, delay: -1.6 },
  { id: 'b5', base: 0.42, variance: 0.1, delay: -0.2 },
  { id: 'b6', base: 0.31, variance: 0.14, delay: -1.4 },
  { id: 'b7', base: 0.26, variance: 0.09, delay: -0.9 },
  { id: 'b8', base: 0.39, variance: 0.08, delay: -1.8 },
  { id: 'b9', base: 0.24, variance: 0.12, delay: -0.6 },
  { id: 'b10', base: 0.3, variance: 0.1, delay: -1.3 },
]

const COLUMN_SEEDS: readonly ColumnSeed[] = [
  { id: 'c1', x: 13, height: 0.52, delay: -0.6 },
  { id: 'c2', x: 21, height: 0.68, delay: -1.4 },
  { id: 'c3', x: 31, height: 0.42, delay: -0.9 },
  { id: 'c4', x: 42, height: 0.78, delay: -1.8 },
  { id: 'c5', x: 58, height: 0.74, delay: -1.1 },
  { id: 'c6', x: 69, height: 0.46, delay: -0.7 },
  { id: 'c7', x: 79, height: 0.66, delay: -1.5 },
  { id: 'c8', x: 88, height: 0.5, delay: -0.3 },
]

const SHARD_SEEDS: readonly ShardSeed[] = [
  { id: 's1', x: 17, y: 34, rotation: -26, size: 72, delay: -1.8 },
  { id: 's2', x: 26, y: 71, rotation: 18, size: 88, delay: -3.2 },
  { id: 's3', x: 38, y: 16, rotation: -8, size: 64, delay: -0.7 },
  { id: 's4', x: 62, y: 18, rotation: 12, size: 76, delay: -2.4 },
  { id: 's5', x: 74, y: 72, rotation: -14, size: 92, delay: -1.1 },
  { id: 's6', x: 83, y: 37, rotation: 22, size: 68, delay: -2.9 },
]

const LATTICE_NODES: readonly LatticeNode[] = [
  { id: 'n1', x: 16, y: 28 },
  { id: 'n2', x: 28, y: 18 },
  { id: 'n3', x: 44, y: 26, emphasis: true },
  { id: 'n4', x: 61, y: 18 },
  { id: 'n5', x: 78, y: 31 },
  { id: 'n6', x: 22, y: 63 },
  { id: 'n7', x: 40, y: 56 },
  { id: 'n8', x: 58, y: 66, emphasis: true },
  { id: 'n9', x: 76, y: 57 },
]

const LATTICE_LINKS: readonly LatticeLink[] = [
  { id: 'l1', from: 'n1', to: 'n2' },
  { id: 'l2', from: 'n2', to: 'n3' },
  { id: 'l3', from: 'n3', to: 'n4' },
  { id: 'l4', from: 'n4', to: 'n5' },
  { id: 'l5', from: 'n1', to: 'n6' },
  { id: 'l6', from: 'n2', to: 'n6' },
  { id: 'l7', from: 'n3', to: 'n7' },
  { id: 'l8', from: 'n4', to: 'n8' },
  { id: 'l9', from: 'n5', to: 'n9' },
  { id: 'l10', from: 'n6', to: 'n7' },
  { id: 'l11', from: 'n7', to: 'n8' },
  { id: 'l12', from: 'n8', to: 'n9' },
  { id: 'l13', from: 'n3', to: 'n8' },
]

const runtimeStore = useRuntimeStore()
const { bootPhase, bootStage } = storeToRefs(runtimeStore)
const prefersReducedMotion = useMediaQuery('(prefers-reduced-motion: reduce)')

const overlayStage = ref<OverlayStage>('hidden')
const canSkip = ref(false)
const progress = ref(0)
const introStartedAt = ref<number | null>(null)
const holdDebugState = ref(false)

let activeTimer: number | null = null
let skipUnlockTimer: number | null = null
let exitQueueTimer: number | null = null
let exitTimer: number | null = null
let failSafeTimer: number | null = null
let progressRaf: number | null = null
let keyListenerBound = false
let holdListenersBound = false

const minVisibleMs = computed(() => (prefersReducedMotion.value ? 520 : MIN_VISIBLE_MS))
const readySettleMs = computed(() => (prefersReducedMotion.value ? 120 : READY_SETTLE_MS))
const errorSettleMs = computed(() => (prefersReducedMotion.value ? 180 : ERROR_SETTLE_MS))
const exitMs = computed(() => (prefersReducedMotion.value ? 180 : EXIT_MS))
const activeDelayMs = computed(() => (prefersReducedMotion.value ? 0 : ACTIVE_DELAY_MS))
const skipUnlockMs = computed(() => (prefersReducedMotion.value ? 260 : SKIP_UNLOCK_MS))
const failSafeMs = computed(() => (prefersReducedMotion.value ? 3000 : FAILSAFE_DISMISS_MS))

function clamp01(value: number): number {
  return Math.min(1, Math.max(0, value))
}

function lerp(min: number, max: number, amount: number): number {
  return min + (max - min) * amount
}

function clearTimer(timer: number | null): void {
  if (timer !== null) window.clearTimeout(timer)
}

function cancelProgressLoop(): void {
  if (progressRaf !== null) {
    window.cancelAnimationFrame(progressRaf)
    progressRaf = null
  }
}

function clearScheduledTransitions(): void {
  clearTimer(activeTimer)
  clearTimer(skipUnlockTimer)
  clearTimer(exitQueueTimer)
  clearTimer(exitTimer)
  clearTimer(failSafeTimer)
  activeTimer = null
  skipUnlockTimer = null
  exitQueueTimer = null
  exitTimer = null
  failSafeTimer = null
}

function readHoldDebugFlag(): boolean {
  if (typeof window === 'undefined') return false
  try {
    const params = new URLSearchParams(window.location.search)
    if (params.get(HOLD_QUERY_KEY) === HOLD_QUERY_VALUE) return true
    return localStorage.getItem(HOLD_STORAGE_KEY) === '1'
  } catch {
    return false
  }
}

function refreshHoldDebugState(): void {
  holdDebugState.value = readHoldDebugFlag()
}

function bindHoldListeners(): void {
  if (typeof window === 'undefined' || holdListenersBound) return
  window.addEventListener('popstate', refreshHoldDebugState)
  window.addEventListener('storage', refreshHoldDebugState)
  window.addEventListener('focus', refreshHoldDebugState)
  holdListenersBound = true
}

function unbindHoldListeners(): void {
  if (typeof window === 'undefined' || !holdListenersBound) return
  window.removeEventListener('popstate', refreshHoldDebugState)
  window.removeEventListener('storage', refreshHoldDebugState)
  window.removeEventListener('focus', refreshHoldDebugState)
  holdListenersBound = false
}

function bindKeyListener(): void {
  if (typeof window === 'undefined' || keyListenerBound) return
  window.addEventListener('keydown', handleKeydown)
  keyListenerBound = true
}

function unbindKeyListener(): void {
  if (typeof window === 'undefined' || !keyListenerBound) return
  window.removeEventListener('keydown', handleKeydown)
  keyListenerBound = false
}

const showOverlay = computed(
  () => overlayStage.value === 'intro' || overlayStage.value === 'active' || overlayStage.value === 'exiting',
)

const phaseTone = computed(() => {
  if (bootPhase.value === 'error') return 'error'
  if (bootPhase.value === 'ready') return 'ready'
  return 'booting'
})

const sceneKey = computed<SceneKey>(() => {
  if (bootPhase.value === 'error') return 'handoff'
  if (bootPhase.value === 'ready' || overlayStage.value === 'exiting') return 'handoff'
  if (progress.value < 0.24) return 'signal'
  if (progress.value < 0.56) return 'topology'
  if (progress.value < 0.84) return 'convergence'
  return 'handoff'
})

const sceneIndex = computed(() => {
  switch (sceneKey.value) {
    case 'signal':
      return 1
    case 'topology':
      return 2
    case 'convergence':
      return 3
    default:
      return 4
  }
})

const stageLabel = computed(() => {
  const label = String(bootStage.value || '').trim()
  if (label) return label

  switch (sceneKey.value) {
    case 'signal':
      return `Acquiring source signature for ${props.productName}`
    case 'topology':
      return 'Weaving routes, assets, and session state into a unified mesh'
    case 'convergence':
      return 'Balancing renderer, input, and data channels for live launch'
    default:
      return bootPhase.value === 'error'
        ? 'Primary surface degraded — opening fail-safe handoff route'
        : 'Preparing the final interface handoff'
  }
})

const phaseLabel = computed(() => {
  if (bootPhase.value === 'error') return 'Fallback'
  if (bootPhase.value === 'ready') return 'Ready'
  if (overlayStage.value === 'exiting') return 'Handoff'
  return 'Booting'
})

const progressPercent = computed(() => Math.round(progress.value * 100))
const signalStrength = computed(() => Math.round(lerp(24, 98, progress.value)))
const meshNodeCount = computed(() => Math.round(lerp(48, 1424, progress.value)))
const meshNodeCountLabel = computed(() => meshNodeCount.value.toLocaleString())
const meshClusterCountLabel = computed(() => Math.round(lerp(3, 17, progress.value)).toString())
const renderSyncLabel = computed(() => `${Math.round(lerp(18, 240, progress.value))} fps`)
const latencyValue = computed(() => {
  if (bootPhase.value === 'error') return Math.round(lerp(34, 76, progress.value))
  return Math.max(5, Math.round(lerp(32, 7, progress.value)))
})
const pressureLabel = computed(() => {
  if (bootPhase.value === 'error') return 'reroute'
  return `${Math.round(lerp(38, 96, progress.value))}%`
})

const statusHeadline = computed(() => {
  if (bootPhase.value === 'error') return 'Fail-safe launch surface staged'
  if (bootPhase.value === 'ready') return 'Runtime synchronized and live'
  if (overlayStage.value === 'exiting') return 'Releasing startup overlay'

  switch (sceneKey.value) {
    case 'signal':
      return 'Acquiring source signature'
    case 'topology':
      return 'Weaving dependency topology'
    case 'convergence':
      return 'Converging live subsystems'
    default:
      return 'Preparing interface handoff'
  }
})

const sceneTitle = computed(() => {
  switch (sceneKey.value) {
    case 'signal':
      return 'Signal acquisition'
    case 'topology':
      return 'Topology bloom'
    case 'convergence':
      return 'Convergence corridor'
    default:
      return bootPhase.value === 'error' ? 'Recovery handoff' : 'Interface handoff'
  }
})

const sceneSubtitle = computed(() => {
  switch (sceneKey.value) {
    case 'signal':
      return 'Carrier lanes stabilize while the runtime resolves identity and origin state.'
    case 'topology':
      return 'Routes, assets, and dependencies fold into a single operational mesh.'
    case 'convergence':
      return 'Renderer, input, and session services align on the launch corridor.'
    default:
      return bootPhase.value === 'error'
        ? 'The primary route degraded, but a resilient fallback surface is now staged.'
        : 'The live shell is staged and ready to assume focus.'
  }
})

const syncSignature = computed(() => {
  if (bootPhase.value === 'error') return 'Recovery mesh engaged'
  if (bootPhase.value === 'ready') return 'Session mesh locked'

  switch (sceneKey.value) {
    case 'signal':
      return 'Origin signature verified'
    case 'topology':
      return 'Dependency lattice woven'
    case 'convergence':
      return 'Runtime pressure balanced'
    default:
      return 'Final shell preloaded'
  }
})

const vectorCode = computed(() => `VX-${Math.round(progress.value * 4095).toString(16).toUpperCase().padStart(3, '0')}`)

const skipHintVisible = computed(() => canSkip.value && bootPhase.value !== 'error')
const skipHintText = computed(() => {
  if (bootPhase.value === 'error') return 'Opening recovery surface…'
  if (!canSkip.value) return 'Stabilizing cinematic boot lattice…'
  return prefersReducedMotion.value ? 'Tap, click, or press Esc to continue' : 'Click anywhere or press Esc / Enter to continue'
})

const skipButtonText = computed(() => {
  if (bootPhase.value === 'ready') return 'Enter workspace'
  return 'Skip intro'
})

const signalMeterCaption = computed(() => {
  if (bootPhase.value === 'error') return 'carrier integrity lost — rerouting'
  if (progress.value < 0.2) return 'spectral bands stabilizing'
  if (progress.value < 0.45) return 'carrier lock approaching threshold'
  return 'signal lattice within launch tolerances'
})

const handoffHeadline = computed(() => {
  if (bootPhase.value === 'error') return 'Recovery surface open'
  if (bootPhase.value === 'ready') return 'Live shell unlocked'
  return 'Final interface staging'
})

const handoffCopy = computed(() => {
  if (bootPhase.value === 'error') {
    return 'The primary surface degraded during launch, but a resilient recovery path has been exposed so the session can continue.'
  }
  if (bootPhase.value === 'ready') {
    return 'All core services are synchronized. The startup overlay is releasing into the live interface.'
  }
  return 'Background services are still settling while the shell preloads the last scene transition.'
})

const sceneCallouts = computed(() => {
  switch (sceneKey.value) {
    case 'signal':
      return [
        'Handshake lanes warming across source clusters',
        'Entropy filter suppressing carrier spill',
        'Identity mesh resolving origin signature',
      ]
    case 'topology':
      return [
        'Asset graph and route groups are stitching together',
        'Dependency lattice is mapping cross-surface links',
        'Session context is fusing into the live mesh',
      ]
    case 'convergence':
      return [
        'Renderer cadence settling against input streams',
        'State transitions balancing under launch pressure',
        'Memory and cache surfaces converging on the corridor',
      ]
    default:
      return bootPhase.value === 'error'
        ? [
            'Launching degraded-mode shell with resilient defaults',
            'Restoring minimum viable controls and navigation',
            'Background diagnostics continuing after handoff',
          ]
        : [
            'UI shell unlocked and ready to assume focus',
            'Background systems remain online after handoff',
            'Overlay release queued on steady-state confirmation',
          ]
  }
})

const leftTelemetryCards = computed<TelemetryCard[]>(() => [
  {
    label: 'Stability',
    value: `${Math.round(lerp(72, 99, progress.value))}%`,
    detail: bootPhase.value === 'error' ? 'recovery envelope' : 'coherence envelope',
  },
  {
    label: 'Mesh nodes',
    value: meshNodeCountLabel.value,
    detail: 'distributed surfaces',
  },
  {
    label: 'Latency',
    value: `${latencyValue.value} ms`,
    detail: bootPhase.value === 'error' ? 'fail-safe channel' : 'round-trip jitter',
  },
])

const milestones = computed(() => {
  const thresholdMap = [
    { key: 'signal', label: 'Signal lock', detail: 'Origin carrier resolved', threshold: 0.17 },
    { key: 'topology', label: 'Topology weave', detail: 'Dependency lattice connected', threshold: 0.4 },
    { key: 'convergence', label: 'System convergence', detail: 'Renderer and state aligned', threshold: 0.68 },
    {
      key: 'handoff',
      label: bootPhase.value === 'error' ? 'Recovery handoff' : 'UI handoff',
      detail: bootPhase.value === 'error' ? 'Fallback surface exposed' : 'Live shell released',
      threshold: 0.93,
    },
  ]

  return thresholdMap.map((item) => {
    let state: MilestoneState = 'pending'

    if (progress.value >= item.threshold + 0.08 || (bootPhase.value === 'ready' && progress.value >= 0.99)) {
      state = 'done'
    } else if (progress.value >= item.threshold - 0.06) {
      state = 'active'
    }

    return {
      ...item,
      state,
    }
  })
})

const sceneVars = computed<Record<string, string>>(() => ({
  '--startup-progress': progress.value.toFixed(4),
  '--startup-tone':
    bootPhase.value === 'error'
      ? 'var(--danger, #ff6b6b)'
      : bootPhase.value === 'ready'
        ? 'var(--accent-soft, var(--accent, #7ad3ff))'
        : 'var(--accent, #6fc5ff)',
  '--startup-depth': `${lerp(14, 32, progress.value).toFixed(2)}px`,
  '--startup-glow': (0.38 + progress.value * 0.32).toFixed(3),
}))

const signalBars = computed(() =>
  BAR_SEEDS.map((bar, index) => {
    const wave = Math.sin(progress.value * 8.2 + index * 0.82)
    const level = clamp01(bar.base + progress.value * 0.42 + wave * bar.variance)

    return {
      id: bar.id,
      style: {
        '--level': level.toFixed(3),
        '--delay': `${bar.delay}s`,
      } as Record<string, string>,
    }
  }),
)

const backdropParticles = PARTICLE_SEEDS.map((particle) => ({
  id: particle.id,
  style: {
    '--x': `${particle.x}%`,
    '--y': `${particle.y}%`,
    '--size': `${particle.size}px`,
    '--drift': `${particle.drift}s`,
    '--delay': `${particle.delay}s`,
    '--alpha': particle.alpha.toFixed(3),
  } as Record<string, string>,
}))

const corridorColumns = COLUMN_SEEDS.map((column) => ({
  id: column.id,
  style: {
    '--x': `${column.x}%`,
    '--height': column.height.toFixed(3),
    '--delay': `${column.delay}s`,
  } as Record<string, string>,
}))

const handoffShards = SHARD_SEEDS.map((shard) => ({
  id: shard.id,
  style: {
    '--x': `${shard.x}%`,
    '--y': `${shard.y}%`,
    '--rotation': `${shard.rotation}deg`,
    '--size': `${shard.size}px`,
    '--delay': `${shard.delay}s`,
  } as Record<string, string>,
}))

const latticeNodeMap = new Map(LATTICE_NODES.map((node) => [node.id, node]))
const latticeSegments = LATTICE_LINKS.map((link) => {
  const from = latticeNodeMap.get(link.from)
  const to = latticeNodeMap.get(link.to)

  if (!from || !to) {
    throw new Error(`Invalid lattice link: ${link.id}`)
  }

  return {
    id: link.id,
    x1: from.x,
    y1: from.y,
    x2: to.x,
    y2: to.y,
  }
})

const topologyNodes = computed(() =>
  LATTICE_NODES.map((node, index) => ({
    ...node,
    active: progress.value >= 0.12 + index * 0.05 || bootPhase.value === 'ready' || overlayStage.value === 'exiting',
  })),
)

const topologySegments = computed(() =>
  latticeSegments.map((segment, index) => ({
    ...segment,
    active: progress.value >= 0.18 + index * 0.035 || bootPhase.value === 'ready' || overlayStage.value === 'exiting',
  })),
)

function resolveProgressTarget(): number {
  if (bootPhase.value === 'ready' || bootPhase.value === 'error' || overlayStage.value === 'exiting') return 1
  return BOOT_PROGRESS_CAP
}

function stepProgress(): void {
  const target = resolveProgressTarget()
  const delta = target - progress.value
  const easing = target === 1 ? 0.16 : 0.046
  const drift = target === 1 ? 0.0038 : 0.0014

  if (Math.abs(delta) > 0.0007) {
    progress.value = clamp01(progress.value + delta * easing + (delta > 0 ? drift : 0))
  } else {
    progress.value = target
  }

  if (showOverlay.value) {
    progressRaf = window.requestAnimationFrame(stepProgress)
    return
  }

  cancelProgressLoop()
}

function ensureProgressLoop(): void {
  if (typeof window === 'undefined' || progressRaf !== null) return
  progressRaf = window.requestAnimationFrame(stepProgress)
}

function scheduleFailsafeDismiss(): void {
  clearTimer(failSafeTimer)
  failSafeTimer = null

  if (holdDebugState.value) return
  if (overlayStage.value !== 'intro' && overlayStage.value !== 'active') return

  const startedAt = introStartedAt.value ?? Date.now()
  const remaining = Math.max(0, failSafeMs.value - (Date.now() - startedAt))

  failSafeTimer = window.setTimeout(() => {
    if (bootPhase.value === 'booting') startExit()
    failSafeTimer = null
  }, remaining)
}

function startExit(afterMs = 0): void {
  if (overlayStage.value === 'hidden' || overlayStage.value === 'dismissed' || overlayStage.value === 'exiting') return

  const beginExit = () => {
    clearTimer(activeTimer)
    clearTimer(skipUnlockTimer)
    clearTimer(exitQueueTimer)
    clearTimer(failSafeTimer)
    activeTimer = null
    skipUnlockTimer = null
    exitQueueTimer = null
    failSafeTimer = null
    canSkip.value = false
    overlayStage.value = 'exiting'
    ensureProgressLoop()

    exitTimer = window.setTimeout(() => {
      overlayStage.value = 'dismissed'
      exitTimer = null
      unbindKeyListener()
      cancelProgressLoop()
    }, exitMs.value)
  }

  if (afterMs > 0) {
    clearTimer(exitQueueTimer)
    exitQueueTimer = window.setTimeout(() => {
      beginExit()
      exitQueueTimer = null
    }, afterMs)
    return
  }

  beginExit()
}

function scheduleExitAfterMinimum(settleMs: number): void {
  if (overlayStage.value !== 'intro' && overlayStage.value !== 'active') return
  if (holdDebugState.value) return

  const startedAt = introStartedAt.value ?? Date.now()
  const remaining = Math.max(0, minVisibleMs.value - (Date.now() - startedAt)) + settleMs
  startExit(remaining)
}

function requestSkip(): void {
  if (!canSkip.value) return
  if (overlayStage.value !== 'intro' && overlayStage.value !== 'active') return
  if (bootPhase.value === 'error') return
  startExit()
}

function handleKeydown(event: KeyboardEvent): void {
  if (!skipHintVisible.value) return
  if (event.repeat) return
  if (event.key !== 'Escape' && event.key !== 'Enter' && event.key !== ' ') return

  event.preventDefault()
  requestSkip()
}

function startIntro(): void {
  clearScheduledTransitions()
  overlayStage.value = 'intro'
  introStartedAt.value = Date.now()
  progress.value = 0
  canSkip.value = false
  bindKeyListener()
  ensureProgressLoop()

  activeTimer = window.setTimeout(() => {
    overlayStage.value = 'active'
    activeTimer = null
  }, activeDelayMs.value)

  skipUnlockTimer = window.setTimeout(() => {
    canSkip.value = true
    skipUnlockTimer = null
  }, skipUnlockMs.value)

  scheduleFailsafeDismiss()
}

refreshHoldDebugState()
bindHoldListeners()

watch(
  bootPhase,
  (phase) => {
    if (phase === 'booting') {
      if (overlayStage.value === 'hidden' || overlayStage.value === 'dismissed') {
        startIntro()
      } else {
        ensureProgressLoop()
        scheduleFailsafeDismiss()
      }
      return
    }

    if (phase === 'ready') {
      if (overlayStage.value === 'hidden') {
        progress.value = 1
        overlayStage.value = 'dismissed'
        return
      }
      scheduleExitAfterMinimum(readySettleMs.value)
      return
    }

    if (phase === 'error') {
      if (overlayStage.value === 'hidden') {
        progress.value = 1
        overlayStage.value = 'dismissed'
        return
      }
      scheduleExitAfterMinimum(errorSettleMs.value)
      return
    }

    if (overlayStage.value === 'hidden') {
      overlayStage.value = 'dismissed'
    }
  },
  { immediate: true },
)

watch(holdDebugState, (hold) => {
  if (hold) {
    clearTimer(failSafeTimer)
    clearTimer(exitQueueTimer)
    failSafeTimer = null
    exitQueueTimer = null
    return
  }

  if (bootPhase.value === 'ready') {
    scheduleExitAfterMinimum(readySettleMs.value)
    return
  }

  if (bootPhase.value === 'error') {
    scheduleExitAfterMinimum(errorSettleMs.value)
    return
  }

  if (bootPhase.value === 'booting' && (overlayStage.value === 'intro' || overlayStage.value === 'active')) {
    scheduleFailsafeDismiss()
  }
})

watch(showOverlay, (visible) => {
  if (visible) {
    ensureProgressLoop()
    return
  }

  cancelProgressLoop()
})

onBeforeUnmount(() => {
  clearScheduledTransitions()
  cancelProgressLoop()
  unbindKeyListener()
  unbindHoldListeners()
})
</script>

<template>
  <div
    v-if="showOverlay"
    id="startupAnimationOverlay"
    class="startupOverlay"
    :class="[overlayStage, { reduced: prefersReducedMotion }]"
    :data-tone="phaseTone"
    :data-scene="sceneKey"
    :data-stage="overlayStage"
    :style="sceneVars"
    @click="requestSkip"
  >
    <div class="startupBackdrop" aria-hidden="true">
      <div class="startupAura startupAura--north"></div>
      <div class="startupAura startupAura--south"></div>
      <div class="startupBeamField"></div>
      <div class="startupBackdropGrid startupBackdropGrid--horizon"></div>
      <div class="startupBackdropGrid startupBackdropGrid--plane"></div>
      <div class="startupParticles">
        <span
          v-for="particle in backdropParticles"
          :key="particle.id"
          class="startupParticle"
          :style="particle.style"
        ></span>
      </div>
      <div class="startupScanlines"></div>
      <div class="startupNoise"></div>
    </div>

    <section class="startupStage" role="region" :aria-label="`${productName} startup overlay`">
      <header class="startupHeader">
        <div class="startupGlassCard startupBrandPanel">
          <span class="startupTinyLabel">Launch sequence</span>
          <div class="startupBrandRow">
            <VectorAlphaLogo class="startupHeaderLogo" decorative :size="42" />
            <div class="startupBrandCopy">
              <h1>{{ productName }}</h1>
              <p>{{ tagline }}</p>
            </div>
          </div>
          <strong class="startupHeadline">{{ statusHeadline }}</strong>
        </div>

        <div class="startupGlassCard startupStatusPanel" role="status" aria-live="polite">
          <div class="startupStatusMeta">
            <span class="startupPill">{{ phaseLabel }}</span>
            <span class="startupSceneToken">Scene {{ sceneIndex }} · {{ sceneTitle }}</span>
          </div>
          <p class="startupStageLine">{{ stageLabel }}</p>
          <div class="startupStatusRows">
            <span>{{ syncSignature }}</span>
            <span>{{ vectorCode }}</span>
          </div>
        </div>
      </header>

      <aside class="startupTelemetryColumn startupTelemetryColumn--left">
        <div
          v-for="telemetry in leftTelemetryCards"
          :key="telemetry.label"
          class="startupGlassCard startupTelemetryCard"
        >
          <span class="startupTinyLabel">{{ telemetry.label }}</span>
          <strong>{{ telemetry.value }}</strong>
          <p>{{ telemetry.detail }}</p>
        </div>
      </aside>

      <div class="startupSceneStack" aria-hidden="true">
        <article class="startupSceneCard startupSceneCard--signal" :class="{ active: sceneKey === 'signal' }">
          <div class="startupSignalFrame">
            <div class="startupGlassCard startupSignalMeter">
              <span class="startupTinyLabel">Handshake lanes</span>
              <div class="startupSignalBars">
                <div v-for="bar in signalBars" :key="bar.id" class="startupSignalBar" :style="bar.style">
                  <span class="startupSignalBarFill"></span>
                </div>
              </div>
              <p>{{ signalMeterCaption }}</p>
            </div>

            <div class="startupCoreStage">
              <div class="startupCoreHalo"></div>
              <div class="startupCoreRing startupCoreRing--outer"></div>
              <div class="startupCoreRing startupCoreRing--middle"></div>
              <div class="startupCoreRing startupCoreRing--inner"></div>
              <div class="startupCoreSweep"></div>
              <div class="startupCoreOrbit">
                <span class="startupCoreOrbitDot startupCoreOrbitDot--a"></span>
                <span class="startupCoreOrbitDot startupCoreOrbitDot--b"></span>
                <span class="startupCoreOrbitDot startupCoreOrbitDot--c"></span>
                <span class="startupCoreOrbitDot startupCoreOrbitDot--d"></span>
              </div>
              <svg class="startupCoreDiagram" viewBox="0 0 420 420">
                <circle cx="210" cy="210" r="164"></circle>
                <circle cx="210" cy="210" r="116"></circle>
                <circle cx="210" cy="210" r="62"></circle>
                <path d="M210 26V92"></path>
                <path d="M210 328V394"></path>
                <path d="M26 210H92"></path>
                <path d="M328 210H394"></path>
                <path d="M110 122L210 210L310 122"></path>
                <path d="M122 308L210 210L298 308"></path>
              </svg>
              <div class="startupCoreEmblem">
                <VectorAlphaLogo decorative :size="92" />
              </div>
              <div class="startupCoreCaption">
                <span class="startupTinyLabel">Phase one</span>
                <strong>Signal acquisition</strong>
              </div>
            </div>

            <div class="startupSceneInlineCards">
              <div class="startupGlassCard startupInlineCard">
                <span class="startupTinyLabel">Wavefront</span>
                <strong>{{ signalStrength }}%</strong>
                <p>carrier integrity</p>
              </div>
              <div class="startupGlassCard startupInlineCard">
                <span class="startupTinyLabel">Route seed</span>
                <strong>{{ vectorCode }}</strong>
                <p>staging channel</p>
              </div>
            </div>
          </div>
        </article>

        <article class="startupSceneCard startupSceneCard--topology" :class="{ active: sceneKey === 'topology' }">
          <div class="startupTopologyFrame">
            <div class="startupGlassCard startupSceneCopyCard">
              <span class="startupTinyLabel">Phase two</span>
              <strong>Topology bloom</strong>
              <p>Routes, assets, and dependency surfaces are folding into a single live mesh.</p>
            </div>

            <div class="startupTopologyDiagramWrap">
              <div class="startupTopologyBeam startupTopologyBeam--a"></div>
              <div class="startupTopologyBeam startupTopologyBeam--b"></div>
              <svg class="startupTopologySvg" viewBox="0 0 100 100">
                <line
                  v-for="segment in topologySegments"
                  :key="segment.id"
                  :class="{ active: segment.active }"
                  :x1="segment.x1"
                  :y1="segment.y1"
                  :x2="segment.x2"
                  :y2="segment.y2"
                />
                <circle
                  v-for="node in topologyNodes"
                  :key="node.id"
                  :class="{ active: node.active, emphasis: node.emphasis }"
                  :cx="node.x"
                  :cy="node.y"
                  :r="node.emphasis ? 3.1 : 2.3"
                />
              </svg>
              <div class="startupTopologyPulse"></div>
            </div>

            <div class="startupSceneInlineCards">
              <div class="startupGlassCard startupInlineCard">
                <span class="startupTinyLabel">Clusters</span>
                <strong>{{ meshClusterCountLabel }}</strong>
                <p>mesh domains</p>
              </div>
              <div class="startupGlassCard startupInlineCard">
                <span class="startupTinyLabel">Assets</span>
                <strong>{{ meshNodeCountLabel }}</strong>
                <p>indexed surfaces</p>
              </div>
            </div>
          </div>
        </article>

        <article class="startupSceneCard startupSceneCard--convergence" :class="{ active: sceneKey === 'convergence' }">
          <div class="startupConvergenceFrame">
            <div class="startupGlassCard startupSceneCopyCard">
              <span class="startupTinyLabel">Phase three</span>
              <strong>Convergence corridor</strong>
              <p>Render, input, and session systems align as launch pressure tapers into a steady corridor.</p>
            </div>

            <div class="startupCorridor">
              <div class="startupCorridorFloor"></div>
              <div class="startupCorridorFloor startupCorridorFloor--upper"></div>
              <div class="startupCorridorWall startupCorridorWall--left"></div>
              <div class="startupCorridorWall startupCorridorWall--right"></div>
              <span
                v-for="column in corridorColumns"
                :key="column.id"
                class="startupCorridorColumn"
                :style="column.style"
              ></span>
              <div class="startupAperture">
                <div class="startupApertureRing startupApertureRing--outer"></div>
                <div class="startupApertureRing startupApertureRing--inner"></div>
                <div class="startupApertureCore">
                  <VectorAlphaLogo decorative :size="68" />
                </div>
              </div>
            </div>

            <div class="startupSceneInlineCards">
              <div class="startupGlassCard startupInlineCard">
                <span class="startupTinyLabel">Render sync</span>
                <strong>{{ renderSyncLabel }}</strong>
                <p>frame cadence</p>
              </div>
              <div class="startupGlassCard startupInlineCard">
                <span class="startupTinyLabel">Pressure</span>
                <strong>{{ pressureLabel }}</strong>
                <p>launch load</p>
              </div>
            </div>
          </div>
        </article>

        <article class="startupSceneCard startupSceneCard--handoff" :class="{ active: sceneKey === 'handoff' }">
          <div class="startupHandoffFrame">
            <div class="startupHandoffBurst"></div>
            <div class="startupHandoffGaugeWrap">
              <svg class="startupHandoffGauge" viewBox="0 0 140 140">
                <circle class="startupHandoffGaugeTrack" cx="70" cy="70" r="52"></circle>
                <circle class="startupHandoffGaugeFill" cx="70" cy="70" r="52"></circle>
              </svg>
              <div class="startupHandoffCenter">
                <VectorAlphaLogo decorative :size="78" />
                <span class="startupPill startupPill--soft">{{ phaseLabel }}</span>
                <strong>{{ progressPercent }}%</strong>
              </div>
            </div>
            <div class="startupHandoffCopy">
              <span class="startupTinyLabel">Phase four</span>
              <strong>{{ handoffHeadline }}</strong>
              <p>{{ handoffCopy }}</p>
            </div>
            <div class="startupShards">
              <span v-for="shard in handoffShards" :key="shard.id" class="startupShard" :style="shard.style"></span>
            </div>
          </div>
        </article>
      </div>

      <aside class="startupTelemetryColumn startupTelemetryColumn--right">
        <div class="startupGlassCard startupNarrativeCard">
          <span class="startupTinyLabel">Current scene</span>
          <strong>{{ sceneTitle }}</strong>
          <p>{{ sceneSubtitle }}</p>
        </div>

        <div class="startupGlassCard startupChecklistCard">
          <span class="startupTinyLabel">Live operations</span>
          <ul class="startupChecklist">
            <li v-for="item in sceneCallouts" :key="item">{{ item }}</li>
          </ul>
        </div>
      </aside>

      <footer class="startupFooter">
        <div class="startupGlassCard startupProgressPanel">
          <div class="startupProgressTop">
            <span>Boot sequence</span>
            <strong>{{ progressPercent }}%</strong>
          </div>
          <div class="startupProgressRail" aria-hidden="true">
            <div class="startupProgressFill"></div>
            <div class="startupProgressGlow"></div>
          </div>
          <p class="startupSkipHint">{{ skipHintText }}</p>
        </div>

        <div class="startupGlassCard startupMilestonePanel">
          <span class="startupTinyLabel">Pipeline milestones</span>
          <ul class="startupMilestoneList">
            <li
              v-for="milestone in milestones"
              :key="milestone.key"
              class="startupMilestone"
              :class="milestone.state"
            >
              <span class="startupMilestoneDot"></span>
              <div>
                <strong>{{ milestone.label }}</strong>
                <p>{{ milestone.detail }}</p>
              </div>
            </li>
          </ul>
        </div>

        <button
          v-if="skipHintVisible"
          type="button"
          class="startupSkipButton"
          @click.stop="requestSkip"
        >
          {{ skipButtonText }}
        </button>
      </footer>
    </section>
  </div>
</template>

<style scoped>
.startupOverlay {
  --startup-bg-0: color-mix(in oklab, var(--header-bg, #05070d) 88%, black);
  --startup-bg-1: color-mix(in oklab, var(--surface-sticky, #0a1220) 82%, black);
  --startup-bg-2: color-mix(in oklab, var(--surface-base, #11192a) 72%, black);
  --startup-panel: color-mix(in oklab, var(--surface-base, rgba(8, 12, 20, 0.84)) 84%, transparent);
  --startup-panel-strong: color-mix(in oklab, var(--surface-inline, rgba(13, 19, 31, 0.94)) 88%, transparent);
  --startup-border: color-mix(in oklab, var(--border, rgba(255, 255, 255, 0.18)) 78%, transparent);
  --startup-text: var(--text, rgba(246, 249, 255, 0.96));
  --startup-muted: var(--muted-soft, rgba(190, 201, 216, 0.74));
  --startup-shadow: 0 24px 60px rgba(0, 0, 0, 0.32);
  position: fixed;
  inset: 0;
  z-index: 5000;
  overflow: hidden;
  opacity: 1;
  isolation: isolate;
  background:
    radial-gradient(circle at 50% 20%, color-mix(in oklab, var(--startup-tone) 14%, transparent), transparent 36%),
    linear-gradient(180deg, var(--startup-bg-0) 0%, var(--startup-bg-1) 48%, var(--startup-bg-2) 100%);
  transition: opacity 720ms var(--ease-standard, cubic-bezier(0.2, 0, 0, 1));
  pointer-events: auto;
}

.startupOverlay.exiting {
  opacity: 0;
}

.startupOverlay[data-stage='intro'] .startupStage {
  opacity: 0.96;
  transform: scale(0.986);
}

.startupOverlay[data-stage='active'] .startupStage,
.startupOverlay[data-stage='exiting'] .startupStage {
  opacity: 1;
  transform: scale(1);
}

.startupOverlay[data-tone='error'] {
  background:
    radial-gradient(circle at 50% 20%, color-mix(in oklab, var(--startup-tone) 18%, transparent), transparent 34%),
    linear-gradient(180deg, #090508 0%, #16090d 48%, #12080c 100%);
}

.startupBackdrop {
  position: absolute;
  inset: 0;
  overflow: hidden;
  pointer-events: none;
}

.startupAura {
  position: absolute;
  border-radius: 999px;
  filter: blur(90px);
  opacity: calc(var(--startup-glow) + 0.08);
  background: radial-gradient(circle at 50% 50%, color-mix(in oklab, var(--startup-tone) 40%, white 8%), transparent 72%);
  animation: startupFloat 18s ease-in-out infinite alternate;
}

.startupAura--north {
  width: 42vw;
  height: 42vw;
  top: -16%;
  left: -10%;
}

.startupAura--south {
  width: 48vw;
  height: 48vw;
  right: -16%;
  bottom: -24%;
  animation-duration: 24s;
  animation-delay: -6s;
}

.startupBeamField {
  position: absolute;
  inset: -24%;
  opacity: 0.42;
  background:
    linear-gradient(124deg, transparent 24%, color-mix(in oklab, var(--startup-tone) 18%, transparent) 44%, transparent 62%),
    linear-gradient(304deg, transparent 28%, color-mix(in oklab, var(--startup-tone) 12%, transparent) 48%, transparent 68%);
  transform: translateY(calc(var(--startup-progress) * -1.5%)) scale(1.05);
  animation: startupSweepField 14s linear infinite;
}

.startupBackdropGrid {
  position: absolute;
  left: -14%;
  right: -14%;
  pointer-events: none;
}

.startupBackdropGrid--horizon {
  top: -6%;
  height: 48%;
  opacity: 0.16;
  background-image:
    linear-gradient(to bottom, rgba(255, 255, 255, 0.08), transparent 70%),
    repeating-linear-gradient(
      90deg,
      transparent 0,
      transparent 84px,
      color-mix(in oklab, var(--startup-tone) 18%, transparent) 85px,
      transparent 86px
    );
  transform: perspective(1000px) rotateX(78deg) translateY(-24%) scale(1.8);
}

.startupBackdropGrid--plane {
  bottom: -18%;
  height: 60%;
  opacity: 0.34;
  background-image:
    linear-gradient(to bottom, transparent 0, color-mix(in oklab, var(--startup-tone) 10%, transparent) 100%),
    repeating-linear-gradient(
      0deg,
      transparent 0,
      transparent 36px,
      color-mix(in oklab, var(--startup-tone) 20%, transparent) 37px,
      transparent 38px
    ),
    repeating-linear-gradient(
      90deg,
      transparent 0,
      transparent 52px,
      color-mix(in oklab, var(--startup-tone) 18%, transparent) 53px,
      transparent 54px
    );
  transform-origin: center bottom;
  transform: perspective(1100px) rotateX(76deg) scale(1.28);
  animation: startupGridAdvance 18s linear infinite;
}

.startupParticles {
  position: absolute;
  inset: 0;
}

.startupParticle {
  position: absolute;
  left: var(--x);
  top: var(--y);
  width: var(--size);
  height: var(--size);
  border-radius: 999px;
  opacity: var(--alpha);
  background: color-mix(in oklab, var(--startup-tone) 52%, white 24%);
  box-shadow: 0 0 12px color-mix(in oklab, var(--startup-tone) 44%, transparent);
  animation: startupParticleDrift var(--drift) ease-in-out infinite alternate;
  animation-delay: var(--delay);
}

.startupScanlines,
.startupNoise {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.startupScanlines {
  opacity: 0.14;
  background: repeating-linear-gradient(
    180deg,
    transparent 0,
    transparent 2px,
    rgba(255, 255, 255, 0.04) 3px,
    transparent 4px
  );
  mix-blend-mode: soft-light;
}

.startupNoise {
  opacity: 0.08;
  background:
    radial-gradient(circle at 18% 22%, rgba(255, 255, 255, 0.08) 0 1px, transparent 1.4px),
    radial-gradient(circle at 72% 64%, rgba(255, 255, 255, 0.06) 0 1px, transparent 1.5px),
    radial-gradient(circle at 42% 78%, rgba(255, 255, 255, 0.04) 0 1px, transparent 1.6px);
  background-size: 170px 170px, 220px 220px, 190px 190px;
  mix-blend-mode: overlay;
}

.startupStage {
  position: relative;
  z-index: 1;
  height: 100%;
  display: grid;
  grid-template-columns: minmax(220px, 0.9fr) minmax(460px, 1.6fr) minmax(220px, 0.95fr);
  grid-template-rows: auto 1fr auto;
  gap: clamp(0.9rem, 1.4vw, 1.25rem);
  padding: clamp(1rem, 2vw, 1.6rem);
  padding-top: calc(var(--desktop-titlebar-h, 0px) + clamp(1rem, 2vw, 1.6rem));
  pointer-events: none;
  transition: transform 640ms var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)), opacity 300ms ease;
}

.startupGlassCard {
  position: relative;
  border: 1px solid color-mix(in oklab, var(--startup-tone) 16%, var(--startup-border));
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.03), transparent 38%),
    var(--startup-panel);
  box-shadow:
    var(--startup-shadow),
    inset 0 1px 0 rgba(255, 255, 255, 0.06);
  backdrop-filter: blur(14px) saturate(1.08);
  border-radius: 22px;
}

.startupHeader {
  grid-column: 1 / -1;
  display: grid;
  grid-template-columns: minmax(0, 1.15fr) minmax(320px, 0.9fr);
  gap: inherit;
}

.startupBrandPanel,
.startupStatusPanel,
.startupProgressPanel,
.startupMilestonePanel,
.startupTelemetryCard,
.startupNarrativeCard,
.startupChecklistCard,
.startupSignalMeter,
.startupInlineCard,
.startupSceneCopyCard {
  padding: 1rem 1.1rem;
}

.startupTinyLabel {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.68rem;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  font-weight: 700;
  color: color-mix(in oklab, var(--startup-tone) 22%, var(--startup-muted));
}

.startupBrandRow {
  display: flex;
  align-items: center;
  gap: 0.95rem;
  margin-top: 0.7rem;
}

.startupHeaderLogo {
  opacity: 0.96;
}

.startupBrandCopy {
  min-width: 0;
}

.startupBrandCopy h1,
.startupBrandCopy p,
.startupHeadline,
.startupStageLine,
.startupStatusRows,
.startupTelemetryCard p,
.startupNarrativeCard p,
.startupChecklist,
.startupSkipHint,
.startupMilestone p,
.startupSceneCopyCard p,
.startupInlineCard p,
.startupSignalMeter p,
.startupHandoffCopy p {
  margin: 0;
}

.startupBrandCopy h1 {
  font-size: clamp(1.3rem, 1.9vw, 1.75rem);
  font-weight: 800;
  line-height: 1.05;
  color: var(--startup-text);
}

.startupBrandCopy p {
  margin-top: 0.18rem;
  font-size: 0.9rem;
  color: var(--startup-muted);
}

.startupHeadline {
  display: block;
  margin-top: 0.95rem;
  font-size: clamp(0.98rem, 1.1vw, 1.08rem);
  font-weight: 700;
  color: var(--startup-text);
}

.startupStatusMeta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  flex-wrap: wrap;
}

.startupPill,
.startupSceneToken,
.startupPill--soft {
  display: inline-flex;
  align-items: center;
  min-height: 1.9rem;
  padding: 0.22rem 0.6rem;
  border-radius: 999px;
  border: 1px solid color-mix(in oklab, var(--startup-tone) 26%, var(--startup-border));
  background: color-mix(in oklab, var(--startup-tone) 10%, transparent);
  color: var(--startup-text);
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.startupSceneToken {
  color: var(--startup-muted);
}

.startupPill--soft {
  background: color-mix(in oklab, var(--startup-tone) 12%, rgba(255, 255, 255, 0.02));
}

.startupStageLine {
  margin-top: 0.8rem;
  font-size: 0.95rem;
  line-height: 1.45;
  color: var(--startup-text);
}

.startupStatusRows {
  margin-top: 0.85rem;
  display: flex;
  justify-content: space-between;
  gap: 0.8rem;
  flex-wrap: wrap;
  font-size: 0.8rem;
  color: var(--startup-muted);
}

.startupTelemetryColumn {
  display: grid;
  gap: 0.9rem;
  align-self: center;
}

.startupTelemetryColumn--left {
  grid-column: 1;
  grid-row: 2;
}

.startupTelemetryColumn--right {
  grid-column: 3;
  grid-row: 2;
}

.startupTelemetryCard strong,
.startupNarrativeCard strong,
.startupSceneCopyCard strong,
.startupInlineCard strong,
.startupHandoffCopy strong {
  display: block;
  margin-top: 0.35rem;
  font-size: 1.15rem;
  font-weight: 800;
  color: var(--startup-text);
}

.startupTelemetryCard p,
.startupNarrativeCard p,
.startupSceneCopyCard p,
.startupInlineCard p,
.startupSignalMeter p,
.startupHandoffCopy p {
  margin-top: 0.4rem;
  font-size: 0.84rem;
  line-height: 1.45;
  color: var(--startup-muted);
}

.startupChecklist {
  padding-left: 1rem;
  margin-top: 0.75rem;
  display: grid;
  gap: 0.65rem;
  font-size: 0.84rem;
  line-height: 1.45;
  color: var(--startup-text);
}

.startupSceneStack {
  grid-column: 2;
  grid-row: 2;
  position: relative;
  display: grid;
  min-height: clamp(360px, 52vh, 620px);
  align-items: center;
}

.startupSceneStack::before {
  content: '';
  position: absolute;
  inset: 6% 5%;
  border-radius: 34px;
  border: 1px solid color-mix(in oklab, var(--startup-tone) 18%, transparent);
  background:
    radial-gradient(circle at 50% 50%, color-mix(in oklab, var(--startup-tone) 10%, transparent), transparent 70%),
    linear-gradient(180deg, rgba(255, 255, 255, 0.03), transparent 30%);
  opacity: 0.8;
}

.startupSceneCard {
  grid-area: 1 / 1;
  position: relative;
  width: 100%;
  opacity: 0;
  transform: translateY(28px) scale(0.965);
  transition:
    opacity 520ms var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)),
    transform 720ms var(--ease-standard, cubic-bezier(0.2, 0, 0, 1));
}

.startupSceneCard.active {
  opacity: 1;
  transform: translateY(0) scale(1);
}

.startupSignalFrame,
.startupTopologyFrame,
.startupConvergenceFrame {
  position: relative;
  display: grid;
  grid-template-columns: 190px minmax(0, 1fr) 210px;
  gap: 1rem;
  align-items: center;
}

.startupSignalBars {
  margin: 0.95rem 0 0.55rem;
  height: 190px;
  display: grid;
  grid-template-columns: repeat(10, minmax(0, 1fr));
  gap: 0.38rem;
  align-items: end;
}

.startupSignalBar {
  height: 100%;
  display: flex;
  align-items: flex-end;
  padding: 0.22rem;
  border-radius: 999px;
  border: 1px solid color-mix(in oklab, var(--startup-tone) 12%, var(--startup-border));
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.03), rgba(255, 255, 255, 0.01));
  overflow: hidden;
}

.startupSignalBarFill {
  width: 100%;
  height: calc(14% + var(--level) * 76%);
  border-radius: inherit;
  background: linear-gradient(
    180deg,
    color-mix(in oklab, var(--startup-tone) 58%, white 16%),
    color-mix(in oklab, var(--startup-tone) 72%, black 10%)
  );
  box-shadow: 0 0 16px color-mix(in oklab, var(--startup-tone) 34%, transparent);
  animation: startupSignalPulse 3.6s ease-in-out infinite;
  animation-delay: var(--delay);
}

.startupCoreStage {
  position: relative;
  width: min(100%, 560px);
  margin: 0 auto;
  aspect-ratio: 1;
  display: grid;
  place-items: center;
  filter: drop-shadow(0 28px 42px rgba(0, 0, 0, 0.28));
}

.startupCoreHalo,
.startupCoreRing,
.startupCoreSweep,
.startupCoreEmblem,
.startupCoreDiagram,
.startupCoreCaption,
.startupCoreOrbit {
  position: absolute;
}

.startupCoreHalo {
  inset: 12%;
  border-radius: 50%;
  background: radial-gradient(circle, color-mix(in oklab, var(--startup-tone) 26%, transparent) 0%, transparent 72%);
}

.startupCoreRing {
  border-radius: 50%;
  border: 1px solid color-mix(in oklab, var(--startup-tone) 26%, var(--startup-border));
  box-shadow: inset 0 0 18px rgba(255, 255, 255, 0.02);
  animation: startupSpin 16s linear infinite;
}

.startupCoreRing--outer {
  inset: 4%;
}

.startupCoreRing--middle {
  inset: 16%;
  border-style: dashed;
  animation-duration: 12s;
  animation-direction: reverse;
}

.startupCoreRing--inner {
  inset: 27%;
  animation-duration: 8s;
}

.startupCoreSweep {
  inset: 10%;
  border-radius: 50%;
  opacity: 0.78;
  background: conic-gradient(
    from 0deg,
    transparent 0 70%,
    color-mix(in oklab, var(--startup-tone) 52%, transparent) 81%,
    transparent 100%
  );
  filter: blur(3px);
  animation: startupSpin 6.2s linear infinite;
}

.startupCoreOrbit {
  inset: 0;
  animation: startupSpin 18s linear infinite reverse;
}

.startupCoreOrbitDot {
  position: absolute;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: color-mix(in oklab, var(--startup-tone) 56%, white 24%);
  box-shadow: 0 0 14px color-mix(in oklab, var(--startup-tone) 34%, transparent);
}

.startupCoreOrbitDot--a {
  top: 12%;
  left: 48%;
}

.startupCoreOrbitDot--b {
  top: 48%;
  right: 10%;
}

.startupCoreOrbitDot--c {
  bottom: 12%;
  left: 46%;
}

.startupCoreOrbitDot--d {
  top: 48%;
  left: 10%;
}

.startupCoreDiagram {
  inset: 0;
  width: 100%;
  height: 100%;
  opacity: 0.7;
}

.startupCoreDiagram circle,
.startupCoreDiagram path {
  fill: none;
  stroke: color-mix(in oklab, var(--startup-tone) 36%, rgba(255, 255, 255, 0.32));
  stroke-width: 1.35;
  vector-effect: non-scaling-stroke;
}

.startupCoreEmblem {
  inset: 33%;
  display: grid;
  place-items: center;
  border-radius: 50%;
  border: 1px solid color-mix(in oklab, var(--startup-tone) 30%, var(--startup-border));
  background:
    radial-gradient(circle at 50% 40%, rgba(255, 255, 255, 0.08), transparent 48%),
    radial-gradient(circle, color-mix(in oklab, var(--startup-tone) 18%, transparent), transparent 72%),
    color-mix(in oklab, var(--startup-panel-strong) 94%, transparent);
  box-shadow:
    0 0 0 1px rgba(255, 255, 255, 0.04),
    0 18px 32px rgba(0, 0, 0, 0.32);
}

.startupCoreCaption {
  left: 50%;
  bottom: 10%;
  transform: translateX(-50%);
  text-align: center;
  display: grid;
  gap: 0.18rem;
}

.startupCoreCaption strong {
  font-size: 1rem;
  font-weight: 800;
  color: var(--startup-text);
}

.startupSceneInlineCards {
  display: grid;
  gap: 0.85rem;
}

.startupInlineCard strong {
  font-size: 1.05rem;
}

.startupTopologyDiagramWrap {
  position: relative;
  aspect-ratio: 1.16 / 0.92;
  border-radius: 30px;
  overflow: hidden;
  border: 1px solid color-mix(in oklab, var(--startup-tone) 18%, var(--startup-border));
  background:
    radial-gradient(circle at 50% 50%, color-mix(in oklab, var(--startup-tone) 12%, transparent), transparent 54%),
    linear-gradient(180deg, rgba(255, 255, 255, 0.03), rgba(255, 255, 255, 0.01));
  box-shadow: var(--startup-shadow);
}

.startupTopologyBeam {
  position: absolute;
  width: 70%;
  height: 24%;
  left: 15%;
  background: linear-gradient(90deg, transparent, color-mix(in oklab, var(--startup-tone) 28%, transparent), transparent);
  filter: blur(12px);
  opacity: 0.5;
  animation: startupBeamSweep 7s linear infinite;
}

.startupTopologyBeam--a {
  top: 18%;
}

.startupTopologyBeam--b {
  bottom: 18%;
  animation-delay: -2.8s;
}

.startupTopologySvg {
  position: absolute;
  inset: 8%;
  width: 84%;
  height: 84%;
}

.startupTopologySvg line {
  stroke: color-mix(in oklab, var(--startup-tone) 28%, rgba(255, 255, 255, 0.24));
  stroke-width: 1.4;
  stroke-dasharray: 7 10;
  opacity: 0.22;
  transition: opacity 340ms ease, stroke 340ms ease;
}

.startupTopologySvg line.active {
  opacity: 0.94;
  stroke: color-mix(in oklab, var(--startup-tone) 56%, rgba(255, 255, 255, 0.42));
}

.startupTopologySvg circle {
  fill: rgba(255, 255, 255, 0.1);
  stroke: color-mix(in oklab, var(--startup-tone) 30%, rgba(255, 255, 255, 0.2));
  stroke-width: 1.2;
  opacity: 0.34;
  transition: opacity 340ms ease, fill 340ms ease, stroke 340ms ease;
}

.startupTopologySvg circle.active {
  fill: color-mix(in oklab, var(--startup-tone) 56%, white 10%);
  stroke: color-mix(in oklab, var(--startup-tone) 62%, white 12%);
  opacity: 1;
}

.startupTopologySvg circle.emphasis {
  filter: drop-shadow(0 0 10px color-mix(in oklab, var(--startup-tone) 40%, transparent));
}

.startupTopologyPulse {
  position: absolute;
  inset: 22%;
  border-radius: 50%;
  border: 1px solid color-mix(in oklab, var(--startup-tone) 18%, transparent);
  animation: startupPulse 3.8s ease-out infinite;
}

.startupCorridor {
  position: relative;
  height: clamp(300px, 46vh, 470px);
  border-radius: 30px;
  overflow: hidden;
  border: 1px solid color-mix(in oklab, var(--startup-tone) 18%, var(--startup-border));
  background:
    radial-gradient(circle at 50% 46%, color-mix(in oklab, var(--startup-tone) 16%, transparent), transparent 34%),
    linear-gradient(180deg, rgba(255, 255, 255, 0.03), rgba(255, 255, 255, 0.01));
  box-shadow: var(--startup-shadow);
}

.startupCorridorFloor {
  position: absolute;
  left: -12%;
  right: -12%;
  bottom: -6%;
  height: 62%;
  opacity: 0.55;
  background-image:
    linear-gradient(to bottom, transparent 0, color-mix(in oklab, var(--startup-tone) 12%, transparent) 100%),
    repeating-linear-gradient(
      0deg,
      transparent 0,
      transparent 28px,
      color-mix(in oklab, var(--startup-tone) 18%, transparent) 29px,
      transparent 30px
    ),
    repeating-linear-gradient(
      90deg,
      transparent 0,
      transparent 46px,
      color-mix(in oklab, var(--startup-tone) 16%, transparent) 47px,
      transparent 48px
    );
  transform-origin: center bottom;
  transform: perspective(1000px) rotateX(74deg) scale(1.26);
  animation: startupGridAdvance 14s linear infinite;
}

.startupCorridorFloor--upper {
  top: -18%;
  bottom: auto;
  height: 46%;
  opacity: 0.18;
  transform-origin: center top;
  transform: perspective(900px) rotateX(-76deg) scale(1.18);
}

.startupCorridorWall {
  position: absolute;
  top: 10%;
  bottom: 10%;
  width: 23%;
  border: 1px solid color-mix(in oklab, var(--startup-tone) 16%, transparent);
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.05), transparent 60%);
  opacity: 0.34;
}

.startupCorridorWall--left {
  left: 7%;
  clip-path: polygon(0 0, 74% 8%, 100% 100%, 0 94%);
}

.startupCorridorWall--right {
  right: 7%;
  clip-path: polygon(26% 8%, 100% 0, 100% 94%, 0 100%);
}

.startupCorridorColumn {
  position: absolute;
  left: var(--x);
  bottom: 18%;
  width: 18px;
  height: calc(var(--height) * 44%);
  transform: translateX(-50%) skewY(-10deg);
  border-radius: 999px;
  background: linear-gradient(
    180deg,
    color-mix(in oklab, var(--startup-tone) 64%, white 14%),
    color-mix(in oklab, var(--startup-tone) 72%, black 10%)
  );
  box-shadow: 0 0 14px color-mix(in oklab, var(--startup-tone) 28%, transparent);
  opacity: 0.82;
  animation: startupColumnDrift 5.8s ease-in-out infinite;
  animation-delay: var(--delay);
}

.startupAperture {
  position: absolute;
  left: 50%;
  top: 50%;
  width: 38%;
  aspect-ratio: 1;
  transform: translate(-50%, -50%);
  display: grid;
  place-items: center;
}

.startupApertureRing {
  position: absolute;
  border-radius: 50%;
  border: 1px solid color-mix(in oklab, var(--startup-tone) 24%, var(--startup-border));
  animation: startupSpin 12s linear infinite;
}

.startupApertureRing--outer {
  inset: 0;
}

.startupApertureRing--inner {
  inset: 18%;
  animation-direction: reverse;
  animation-duration: 8s;
}

.startupApertureCore {
  position: relative;
  width: 46%;
  aspect-ratio: 1;
  border-radius: 50%;
  display: grid;
  place-items: center;
  border: 1px solid color-mix(in oklab, var(--startup-tone) 30%, var(--startup-border));
  background:
    radial-gradient(circle at 50% 40%, rgba(255, 255, 255, 0.08), transparent 42%),
    radial-gradient(circle, color-mix(in oklab, var(--startup-tone) 20%, transparent), transparent 70%),
    var(--startup-panel-strong);
  box-shadow: 0 18px 28px rgba(0, 0, 0, 0.34);
}

.startupHandoffFrame {
  position: relative;
  min-height: clamp(360px, 52vh, 610px);
  display: grid;
  place-items: center;
}

.startupHandoffBurst {
  position: absolute;
  inset: 10% 16%;
  border-radius: 50%;
  opacity: 0.56;
  background: conic-gradient(
    from calc(var(--startup-progress) * 220deg),
    transparent 0 10%,
    color-mix(in oklab, var(--startup-tone) 26%, transparent) 14%,
    transparent 24%,
    transparent 34%,
    color-mix(in oklab, var(--startup-tone) 22%, transparent) 40%,
    transparent 50%,
    transparent 62%,
    color-mix(in oklab, var(--startup-tone) 26%, transparent) 68%,
    transparent 78%,
    transparent 88%,
    color-mix(in oklab, var(--startup-tone) 20%, transparent) 94%,
    transparent 100%
  );
  filter: blur(10px);
  animation: startupSpin 20s linear infinite;
}

.startupHandoffGaugeWrap {
  position: relative;
  width: min(100%, 370px);
  aspect-ratio: 1;
  display: grid;
  place-items: center;
  z-index: 1;
}

.startupHandoffGauge {
  width: 100%;
  height: 100%;
  transform: rotate(-90deg);
}

.startupHandoffGaugeTrack,
.startupHandoffGaugeFill {
  fill: none;
  stroke-linecap: round;
}

.startupHandoffGaugeTrack {
  stroke: color-mix(in oklab, var(--startup-tone) 14%, var(--startup-border));
  stroke-width: 4;
}

.startupHandoffGaugeFill {
  stroke: color-mix(in oklab, var(--startup-tone) 74%, white 8%);
  stroke-width: 5;
  stroke-dasharray: 327;
  stroke-dashoffset: calc(327 * (1 - var(--startup-progress)));
  filter: drop-shadow(0 0 10px color-mix(in oklab, var(--startup-tone) 34%, transparent));
}

.startupHandoffCenter {
  position: absolute;
  inset: 22%;
  display: grid;
  place-items: center;
  align-content: center;
  gap: 0.5rem;
  text-align: center;
  border-radius: 50%;
  border: 1px solid color-mix(in oklab, var(--startup-tone) 24%, var(--startup-border));
  background:
    radial-gradient(circle at 50% 34%, rgba(255, 255, 255, 0.08), transparent 44%),
    radial-gradient(circle, color-mix(in oklab, var(--startup-tone) 18%, transparent), transparent 72%),
    var(--startup-panel-strong);
  box-shadow: 0 18px 28px rgba(0, 0, 0, 0.34);
}

.startupHandoffCenter strong {
  font-size: clamp(2rem, 3vw, 2.4rem);
  font-weight: 900;
  line-height: 1;
  color: var(--startup-text);
}

.startupHandoffCopy {
  position: absolute;
  left: 50%;
  bottom: 4%;
  transform: translateX(-50%);
  width: min(42rem, calc(100% - 2rem));
  text-align: center;
}

.startupShards {
  position: absolute;
  inset: 0;
}

.startupShard {
  position: absolute;
  left: var(--x);
  top: var(--y);
  width: var(--size);
  height: calc(var(--size) * 0.34);
  border-radius: 999px;
  transform: translate(-50%, -50%) rotate(var(--rotation));
  clip-path: polygon(0 50%, 18% 0, 100% 24%, 82% 100%);
  background: linear-gradient(
    90deg,
    transparent 0,
    color-mix(in oklab, var(--startup-tone) 42%, rgba(255, 255, 255, 0.12)) 16%,
    color-mix(in oklab, var(--startup-tone) 30%, transparent) 100%
  );
  opacity: 0.54;
  filter: blur(0.3px);
  animation: startupShardOrbit 11s ease-in-out infinite;
  animation-delay: var(--delay);
}

.startupFooter {
  grid-column: 1 / -1;
  display: grid;
  grid-template-columns: minmax(0, 1.15fr) minmax(0, 1fr) auto;
  gap: inherit;
  align-items: end;
}

.startupProgressTop {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  color: var(--startup-text);
  font-size: 0.9rem;
}

.startupProgressTop strong {
  font-size: 1.12rem;
  font-weight: 800;
}

.startupProgressRail {
  position: relative;
  margin-top: 0.85rem;
  height: 12px;
  overflow: hidden;
  border-radius: 999px;
  border: 1px solid color-mix(in oklab, var(--startup-tone) 16%, var(--startup-border));
  background: rgba(255, 255, 255, 0.04);
}

.startupProgressFill,
.startupProgressGlow {
  position: absolute;
  top: 0;
  bottom: 0;
}

.startupProgressFill {
  left: 0;
  width: calc(var(--startup-progress) * 100%);
  border-radius: inherit;
  background: linear-gradient(
    90deg,
    color-mix(in oklab, var(--startup-tone) 48%, black 4%),
    color-mix(in oklab, var(--startup-tone) 80%, white 10%)
  );
}

.startupProgressGlow {
  width: 28%;
  left: calc(var(--startup-progress) * 100% - 14%);
  background: radial-gradient(circle, color-mix(in oklab, var(--startup-tone) 36%, transparent) 0%, transparent 70%);
  filter: blur(10px);
  opacity: 0.82;
}

.startupSkipHint {
  margin-top: 0.65rem;
  font-size: 0.82rem;
  color: var(--startup-muted);
}

.startupMilestoneList {
  margin: 0.85rem 0 0;
  padding: 0;
  list-style: none;
  display: grid;
  gap: 0.68rem;
}

.startupMilestone {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 0.72rem;
  align-items: start;
  opacity: 0.54;
}

.startupMilestone.active,
.startupMilestone.done {
  opacity: 1;
}

.startupMilestoneDot {
  width: 0.75rem;
  height: 0.75rem;
  margin-top: 0.18rem;
  border-radius: 50%;
  border: 1px solid color-mix(in oklab, var(--startup-tone) 24%, var(--startup-border));
  background: rgba(255, 255, 255, 0.04);
  box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.03);
}

.startupMilestone.active .startupMilestoneDot,
.startupMilestone.done .startupMilestoneDot {
  background: color-mix(in oklab, var(--startup-tone) 70%, white 10%);
  box-shadow: 0 0 14px color-mix(in oklab, var(--startup-tone) 32%, transparent);
}

.startupMilestone strong {
  display: block;
  font-size: 0.9rem;
  font-weight: 700;
  color: var(--startup-text);
}

.startupMilestone p {
  margin-top: 0.18rem;
  font-size: 0.78rem;
  color: var(--startup-muted);
}

.startupSkipButton {
  pointer-events: auto;
  min-width: 172px;
  min-height: 58px;
  padding: 0.8rem 1.1rem;
  border-radius: 18px;
  border: 1px solid color-mix(in oklab, var(--startup-tone) 26%, var(--startup-border));
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.06), rgba(255, 255, 255, 0.01)),
    color-mix(in oklab, var(--startup-panel-strong) 92%, transparent);
  color: var(--startup-text);
  box-shadow:
    0 18px 28px rgba(0, 0, 0, 0.24),
    inset 0 1px 0 rgba(255, 255, 255, 0.08);
  font-size: 0.82rem;
  font-weight: 800;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.startupSkipButton:hover {
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.08), rgba(255, 255, 255, 0.02)),
    color-mix(in oklab, var(--startup-panel-strong) 96%, transparent);
}

.startupOverlay.reduced .startupStage,
.startupOverlay.reduced .startupSceneCard {
  transition-duration: 160ms;
}

.startupOverlay.reduced .startupProgressGlow {
  display: none;
}

@keyframes startupFloat {
  from {
    transform: translate3d(0, 0, 0) scale(1);
  }
  to {
    transform: translate3d(4%, -3%, 0) scale(1.06);
  }
}

@keyframes startupSweepField {
  from {
    transform: translateY(-2%) scale(1.04);
  }
  to {
    transform: translateY(2%) scale(1.08);
  }
}

@keyframes startupGridAdvance {
  from {
    background-position: 0 0, 0 0, 0 0;
  }
  to {
    background-position: 0 0, 0 120px, 40px 0;
  }
}

@keyframes startupParticleDrift {
  from {
    transform: translate3d(0, 0, 0) scale(1);
  }
  to {
    transform: translate3d(10px, -14px, 0) scale(1.18);
  }
}

@keyframes startupSpin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

@keyframes startupSignalPulse {
  0%,
  100% {
    filter: brightness(0.92);
  }
  50% {
    filter: brightness(1.14);
  }
}

@keyframes startupBeamSweep {
  from {
    transform: translateX(-18%) rotate(-8deg);
  }
  to {
    transform: translateX(18%) rotate(8deg);
  }
}

@keyframes startupPulse {
  from {
    transform: scale(0.92);
    opacity: 0.76;
  }
  to {
    transform: scale(1.08);
    opacity: 0;
  }
}

@keyframes startupColumnDrift {
  0%,
  100% {
    transform: translateX(-50%) skewY(-10deg) translateY(0);
  }
  50% {
    transform: translateX(-50%) skewY(-10deg) translateY(-10px);
  }
}

@keyframes startupShardOrbit {
  0%,
  100% {
    transform: translate(-50%, -50%) rotate(var(--rotation)) scale(1);
  }
  50% {
    transform: translate(-50%, -50%) rotate(calc(var(--rotation) + 8deg)) scale(1.08);
  }
}

@media (prefers-reduced-motion: reduce) {
  .startupOverlay,
  .startupStage,
  .startupSceneCard {
    transition: none;
  }

  .startupAura,
  .startupBeamField,
  .startupBackdropGrid--plane,
  .startupParticle,
  .startupCoreRing,
  .startupCoreSweep,
  .startupCoreOrbit,
  .startupSignalBarFill,
  .startupTopologyBeam,
  .startupTopologyPulse,
  .startupCorridorFloor,
  .startupCorridorColumn,
  .startupApertureRing,
  .startupHandoffBurst,
  .startupShard {
    animation: none !important;
  }
}

@media (max-width: 1180px) {
  .startupStage {
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: auto auto auto auto auto;
  }

  .startupHeader,
  .startupFooter {
    grid-column: 1;
  }

  .startupTelemetryColumn--left,
  .startupTelemetryColumn--right {
    grid-column: 1;
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .startupTelemetryColumn--left {
    grid-row: 2;
  }

  .startupSceneStack {
    grid-column: 1;
    grid-row: 3;
  }

  .startupTelemetryColumn--right {
    grid-row: 4;
  }

  .startupFooter {
    grid-row: 5;
    grid-template-columns: minmax(0, 1fr);
  }

  .startupSkipButton {
    width: 100%;
  }
}

@media (max-width: 760px) {
  .startupHeader {
    grid-template-columns: minmax(0, 1fr);
  }

  .startupTelemetryColumn--left,
  .startupTelemetryColumn--right {
    grid-template-columns: minmax(0, 1fr);
  }

  .startupSignalFrame,
  .startupTopologyFrame,
  .startupConvergenceFrame {
    grid-template-columns: minmax(0, 1fr);
  }

  .startupSceneStack {
    min-height: clamp(300px, 48vh, 520px);
  }

  .startupSignalMeter,
  .startupSceneCopyCard,
  .startupSceneInlineCards {
    width: min(100%, 30rem);
    margin-inline: auto;
  }

  .startupSignalBars {
    height: 140px;
  }

  .startupHandoffFrame {
    gap: 1rem;
    align-content: center;
  }

  .startupHandoffCopy {
    position: relative;
    left: auto;
    bottom: auto;
    transform: none;
    width: auto;
    margin-top: 1rem;
    padding: 0 0.5rem;
  }
}

@media (max-width: 520px) {
  .startupStage {
    padding: 0.75rem;
    padding-top: calc(var(--desktop-titlebar-h, 0px) + 0.75rem);
    gap: 0.75rem;
  }

  .startupBrandRow {
    align-items: flex-start;
  }

  .startupBrandCopy h1 {
    font-size: 1.16rem;
  }

  .startupStageLine {
    font-size: 0.88rem;
  }

  .startupTelemetryCard strong,
  .startupNarrativeCard strong,
  .startupSceneCopyCard strong,
  .startupInlineCard strong,
  .startupHandoffCopy strong {
    font-size: 1rem;
  }

  .startupHandoffCenter strong {
    font-size: 1.72rem;
  }
}
</style>
