<script setup lang="ts">
import { computed, onBeforeUnmount, watch, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useMediaQuery } from '@vueuse/core'
import animatedLogoUrl from '../assets/branding/vectoralpha-animated.svg'
import staticLogoUrl from '../assets/branding/vectoralpha-static.svg'
import { useRuntimeStore } from '../stores/runtimeStore'

type OverlayStage = 'hidden' | 'intro' | 'exiting' | 'dismissed'

const MIN_VISIBLE_MS = 5000
const SIGNATURE_MS = 5000
const EXIT_MS = 180
const SKIP_UNLOCK_MS = 600
const HOLD_QUERY_VALUE = 'hold'
const HOLD_QUERY_KEY = 'startup_overlay'
const HOLD_STORAGE_KEY = 'VTA_STARTUP_OVERLAY_HOLD'

const runtimeStore = useRuntimeStore()
const { bootPhase, bootStage } = storeToRefs(runtimeStore)
const prefersReducedMotion = useMediaQuery('(prefers-reduced-motion: reduce)')

const overlayStage = ref<OverlayStage>('hidden')
const canSkip = ref(false)
const introStartedAt = ref<number | null>(null)

let skipUnlockTimer: number | null = null
let signatureTimer: number | null = null
let exitTimer: number | null = null
let readyDelayTimer: number | null = null
let keyListenerBound = false

function clearTimer(timer: number | null): void {
  if (timer) window.clearTimeout(timer)
}

function clearScheduledTransitions(): void {
  clearTimer(skipUnlockTimer)
  clearTimer(signatureTimer)
  clearTimer(exitTimer)
  clearTimer(readyDelayTimer)
  skipUnlockTimer = null
  signatureTimer = null
  exitTimer = null
  readyDelayTimer = null
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

const holdDebugState = computed(() => readHoldDebugFlag())
const showOverlay = computed(() => overlayStage.value === 'intro' || overlayStage.value === 'exiting')
const animatedLogoEnabled = computed(() => !prefersReducedMotion.value && bootPhase.value !== 'error')
const stageLabel = computed(() => String(bootStage.value || '').trim() || 'Starting VectorGrid')
const statusLabel = computed(() => {
  if (bootPhase.value === 'error') return 'Startup failed'
  if (overlayStage.value === 'exiting') return 'Finalizing startup'
  return 'Starting workspace runtime'
})
const skipHintVisible = computed(() => canSkip.value && bootPhase.value !== 'error')

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

function startExit(): void {
  if (overlayStage.value !== 'intro') return
  clearTimer(skipUnlockTimer)
  clearTimer(signatureTimer)
  clearTimer(readyDelayTimer)
  skipUnlockTimer = null
  signatureTimer = null
  readyDelayTimer = null
  canSkip.value = false
  overlayStage.value = 'exiting'
  exitTimer = window.setTimeout(() => {
    overlayStage.value = 'dismissed'
    exitTimer = null
    unbindKeyListener()
  }, EXIT_MS)
}

function scheduleExitAfterMinimum(): void {
  if (overlayStage.value !== 'intro') return
  clearTimer(readyDelayTimer)
  const startedAt = introStartedAt.value ?? Date.now()
  const remaining = Math.max(0, MIN_VISIBLE_MS - (Date.now() - startedAt))
  readyDelayTimer = window.setTimeout(() => {
    startExit()
  }, remaining)
}

function requestSkip(): void {
  if (!canSkip.value || overlayStage.value !== 'intro' || bootPhase.value === 'error') return
  startExit()
}

function handleKeydown(event: KeyboardEvent): void {
  if (!skipHintVisible.value) return
  if (event.repeat) return
  requestSkip()
}

function startIntro(): void {
  clearScheduledTransitions()
  overlayStage.value = 'intro'
  introStartedAt.value = Date.now()
  canSkip.value = false
  bindKeyListener()

  skipUnlockTimer = window.setTimeout(() => {
    canSkip.value = true
    skipUnlockTimer = null
  }, SKIP_UNLOCK_MS)

  if (!holdDebugState.value) {
    signatureTimer = window.setTimeout(() => {
      if (bootPhase.value === 'booting') startExit()
      signatureTimer = null
    }, SIGNATURE_MS)
  }
}

watch(
  bootPhase,
  (phase) => {
    if (phase === 'booting') {
      if (overlayStage.value === 'hidden') startIntro()
      return
    }
    if (phase === 'ready') {
      if (holdDebugState.value) return
      scheduleExitAfterMinimum()
      return
    }
    if (phase === 'error') {
      startExit()
      return
    }
    if (overlayStage.value === 'hidden') {
      overlayStage.value = 'dismissed'
    }
  },
  { immediate: true },
)

watch(holdDebugState, (hold) => {
  if (!hold && bootPhase.value === 'ready' && overlayStage.value === 'intro') {
    scheduleExitAfterMinimum()
    return
  }
  if (!hold && bootPhase.value === 'booting' && overlayStage.value === 'intro' && !signatureTimer) {
    const startedAt = introStartedAt.value ?? Date.now()
    const remaining = Math.max(0, SIGNATURE_MS - (Date.now() - startedAt))
    signatureTimer = window.setTimeout(() => {
      if (bootPhase.value === 'booting') startExit()
      signatureTimer = null
    }, remaining)
  }
})

onBeforeUnmount(() => {
  clearScheduledTransitions()
  unbindKeyListener()
})
</script>

<template>
  <div
    v-if="showOverlay"
    id="startupAnimationOverlay"
    class="startupOverlay"
    :class="[overlayStage, { reduced: prefersReducedMotion }]"
    :data-stage="overlayStage"
    @click="requestSkip"
  >
    <div class="startupVeil" aria-hidden="true"></div>
    <section class="startupScene" role="status" aria-live="polite" aria-label="VectorGrid startup">
      <div class="startupBrand">
        <div class="startupLogoHalo">
          <img
            v-if="animatedLogoEnabled"
            class="startupLogo startupLogoAnimated"
            :src="animatedLogoUrl"
            alt="VectorAlpha animated logo"
            draggable="false"
          />
          <img
            v-else
            class="startupLogo startupLogoStatic"
            :src="staticLogoUrl"
            alt="VectorAlpha logo"
            draggable="false"
          />
        </div>
        <div class="startupBrandCopy">
          <div class="startupEyebrow">VectorAlpha Compute Engine</div>
          <h1 class="startupTitle">VectorGrid</h1>
          <p class="startupSubtitle">{{ stageLabel }}</p>
        </div>
      </div>

      <div class="startupGridField" aria-hidden="true">
        <div class="startupGridMesh"></div>
        <div class="startupGridGlow"></div>
        <div class="startupGridCells">
          <span
            v-for="index in 18"
            :key="`startup_grid_${index}`"
            class="startupGridCell"
          ></span>
        </div>
        <div class="startupSweep"></div>
        <div class="startupFocusTrail"></div>
      </div>

      <div class="startupResultRail" aria-hidden="true">
        <div class="startupResultMeta">Exact-grid scan</div>
        <div class="startupResultBars">
          <span class="startupMetricBar"></span>
          <span class="startupMetricBar highlight"></span>
          <span class="startupMetricBar"></span>
          <span class="startupMetricNote"></span>
        </div>
      </div>

      <div class="startupStatusRail">
        <div class="startupStatus">
          <span class="startupStatusDot" aria-hidden="true"></span>
          <span class="startupStatusLabel">{{ statusLabel }}</span>
        </div>
        <button
          v-if="skipHintVisible"
          type="button"
          class="startupSkipButton"
          @click.stop="requestSkip"
        >
          Skip intro
        </button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.startupOverlay {
  --startup-overlay-opacity: 0.8;
  position: fixed;
  inset: 0;
  z-index: 5000;
  overflow: hidden;
  opacity: var(--startup-overlay-opacity);
  transition: opacity 180ms var(--ease-standard, cubic-bezier(0.2, 0, 0, 1));
  pointer-events: auto;
}

.startupOverlay.exiting {
  opacity: 0;
}

.startupVeil {
  position: absolute;
  inset: 0;
  background:
    radial-gradient(circle at 18% 16%, color-mix(in oklab, var(--accent-soft) 52%, transparent), transparent 30%),
    radial-gradient(circle at 78% 24%, color-mix(in oklab, var(--selection-fill-strong) 34%, transparent), transparent 32%),
    linear-gradient(180deg, color-mix(in oklab, var(--surface-sticky) 52%, transparent), color-mix(in oklab, var(--header-bg) 26%, transparent) 40%, color-mix(in oklab, var(--surface-sticky) 72%, transparent));
  backdrop-filter: blur(14px) saturate(1.08);
  opacity: 0.86;
  pointer-events: none;
}

.startupScene {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.startupBrand {
  position: absolute;
  top: calc(var(--desktop-titlebar-h, 52px) + clamp(1rem, 2.4vh, 1.6rem));
  left: clamp(1rem, 3.4vw, 2.4rem);
  max-width: min(32rem, calc(100vw - 2rem));
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 0.8rem;
  align-items: center;
  padding: 0.8rem 0.9rem;
  border-radius: 18px;
  border: 1px solid color-mix(in oklab, var(--panel-focus-edge) 20%, var(--border));
  background: color-mix(in oklab, var(--surface-sticky) 66%, transparent);
  box-shadow:
    0 18px 44px color-mix(in srgb, black 20%, transparent),
    inset 0 1px 0 color-mix(in srgb, white 12%, transparent);
  backdrop-filter: blur(10px);
  opacity: 0.94;
}

.startupLogoHalo {
  width: clamp(54px, 8vw, 68px);
  aspect-ratio: 1;
  display: grid;
  place-items: center;
  border-radius: 18px;
  background:
    radial-gradient(circle at 50% 50%, color-mix(in oklab, var(--surface-working) 82%, transparent), transparent 74%),
    color-mix(in oklab, var(--surface-sunken) 58%, transparent);
  box-shadow:
    inset 0 0 0 1px color-mix(in oklab, var(--border) 62%, transparent),
    0 0 34px color-mix(in srgb, var(--accent-soft) 18%, transparent);
}

.startupLogo {
  width: 72%;
  height: auto;
  display: block;
  object-fit: contain;
  user-select: none;
  -webkit-user-drag: none;
  filter: drop-shadow(0 8px 18px color-mix(in srgb, var(--accent-soft) 22%, transparent));
}

.startupBrandCopy {
  min-width: 0;
  display: grid;
  gap: 0.16rem;
}

.startupEyebrow {
  font-size: 0.68rem;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--muted-soft);
}

.startupTitle {
  margin: 0;
  font-size: clamp(1.45rem, 2.6vw, 2rem);
  line-height: 0.92;
  letter-spacing: -0.04em;
  color: var(--text);
  text-wrap: balance;
}

.startupSubtitle {
  margin: 0;
  font-size: 0.8rem;
  color: var(--muted-strong);
  max-width: 28ch;
}

.startupGridField {
  position: absolute;
  top: calc(var(--desktop-titlebar-h, 52px) + clamp(6rem, 16vh, 8.5rem));
  left: clamp(1rem, 5vw, 3rem);
  right: clamp(1rem, 7vw, 4rem);
  bottom: clamp(4.8rem, 14vh, 7.2rem);
  border-radius: 34px;
  border: 1px solid color-mix(in oklab, var(--panel-focus-edge) 16%, var(--border));
  background:
    linear-gradient(135deg, color-mix(in oklab, var(--surface-sticky) 28%, transparent), transparent 55%),
    color-mix(in oklab, var(--surface-sticky) 18%, transparent);
  box-shadow:
    inset 0 0 0 1px color-mix(in srgb, white 7%, transparent),
    0 30px 80px color-mix(in srgb, black 20%, transparent);
  overflow: hidden;
  opacity: 0.88;
}

.startupGridMesh,
.startupGridGlow,
.startupSweep,
.startupFocusTrail,
.startupGridCells {
  position: absolute;
}

.startupGridMesh {
  inset: 0;
  background:
    linear-gradient(180deg, color-mix(in oklab, var(--surface-working) 8%, transparent), transparent 30%, color-mix(in oklab, var(--surface-working) 12%, transparent)),
    repeating-linear-gradient(
      90deg,
      transparent 0,
      transparent calc(16.66% - 1px),
      color-mix(in oklab, var(--border) 28%, transparent) calc(16.66% - 1px),
      color-mix(in oklab, var(--border) 28%, transparent) 16.66%
    ),
    repeating-linear-gradient(
      180deg,
      transparent 0,
      transparent calc(25% - 1px),
      color-mix(in oklab, var(--border) 24%, transparent) calc(25% - 1px),
      color-mix(in oklab, var(--border) 24%, transparent) 25%
    );
  opacity: 0.9;
}

.startupGridGlow {
  inset: 14% 24% 18% 10%;
  background:
    radial-gradient(circle at 28% 42%, color-mix(in srgb, var(--accent-soft) 30%, transparent), transparent 42%),
    radial-gradient(circle at 68% 58%, color-mix(in srgb, var(--selection-fill-strong) 34%, transparent), transparent 38%);
  filter: blur(24px);
  opacity: 0.9;
}

.startupGridCells {
  inset: 16% 18% 20% 10%;
  display: grid;
  grid-template-columns: repeat(6, minmax(0, 1fr));
  gap: clamp(0.5rem, 1vw, 0.95rem);
}

.startupGridCell {
  border-radius: 16px;
  background:
    linear-gradient(180deg, color-mix(in oklab, var(--surface-inline) 70%, transparent), color-mix(in oklab, var(--surface-sunken) 82%, transparent)),
    color-mix(in oklab, var(--surface-inline) 72%, transparent);
  box-shadow:
    inset 0 0 0 1px color-mix(in oklab, var(--border) 42%, transparent),
    0 0 0 1px color-mix(in srgb, white 4%, transparent);
  opacity: 0.42;
  transform: scale(0.98);
  animation: startup-grid-cell 900ms var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)) infinite;
}

.startupGridCell:nth-child(3),
.startupGridCell:nth-child(4),
.startupGridCell:nth-child(5),
.startupGridCell:nth-child(8),
.startupGridCell:nth-child(9),
.startupGridCell:nth-child(10),
.startupGridCell:nth-child(11),
.startupGridCell:nth-child(14),
.startupGridCell:nth-child(15) {
  background:
    linear-gradient(135deg, color-mix(in oklab, var(--selection-fill) 80%, var(--surface-inline)), color-mix(in oklab, var(--accent) 44%, var(--selection-fill-strong)));
  box-shadow:
    inset 0 0 0 1px color-mix(in oklab, var(--selection-fill-strong) 34%, transparent),
    0 0 28px color-mix(in srgb, var(--accent-soft) 18%, transparent);
  opacity: 1;
}

.startupGridCell:nth-child(3n + 2) { animation-delay: 70ms; }
.startupGridCell:nth-child(4n + 3) { animation-delay: 140ms; }

.startupSweep {
  top: -12%;
  bottom: -12%;
  left: -18%;
  width: 28%;
  background: linear-gradient(90deg, transparent, color-mix(in srgb, var(--selection-fill-strong) 62%, transparent), transparent);
  filter: blur(20px);
  opacity: 0.75;
  animation: startup-sweep 920ms ease-in-out infinite;
}

.startupFocusTrail {
  top: 22%;
  right: 10%;
  width: min(28vw, 260px);
  height: 28%;
  border-radius: 24px;
  border: 1px solid color-mix(in oklab, var(--selection-fill-strong) 34%, transparent);
  box-shadow:
    inset 0 0 0 1px color-mix(in srgb, white 5%, transparent),
    0 0 24px color-mix(in srgb, var(--accent-soft) 16%, transparent);
  background: linear-gradient(135deg, color-mix(in oklab, var(--selection-fill) 14%, transparent), transparent 58%);
}

.startupResultRail {
  position: absolute;
  right: clamp(1rem, 4vw, 2.8rem);
  bottom: clamp(1.15rem, 4vh, 2rem);
  width: min(320px, calc(100vw - 2rem));
  display: grid;
  gap: 0.4rem;
  padding: 0.8rem 0.9rem;
  border-radius: 18px;
  border: 1px solid color-mix(in oklab, var(--panel-focus-edge) 18%, var(--border));
  background: color-mix(in oklab, var(--surface-sticky) 64%, transparent);
  box-shadow:
    0 18px 44px color-mix(in srgb, black 18%, transparent),
    inset 0 1px 0 color-mix(in srgb, white 10%, transparent);
  backdrop-filter: blur(10px);
  opacity: 0.94;
}

.startupResultMeta {
  font-size: 0.68rem;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--muted-soft);
}

.startupResultBars {
  display: grid;
  gap: 0.46rem;
}

.startupMetricBar {
  height: 10px;
  border-radius: 999px;
  background: color-mix(in oklab, var(--surface-inline) 74%, var(--muted-soft));
  box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--border) 54%, transparent);
}

.startupMetricBar:nth-child(1) { width: 74%; }
.startupMetricBar:nth-child(2) { width: 92%; }
.startupMetricBar:nth-child(3) { width: 58%; }

.startupMetricBar.highlight {
  background:
    linear-gradient(90deg, color-mix(in oklab, var(--selection-fill) 78%, var(--surface-inline)), color-mix(in oklab, var(--accent) 62%, var(--selection-fill)));
  box-shadow:
    inset 0 0 0 1px color-mix(in oklab, var(--selection-fill-strong) 38%, transparent),
    0 0 18px color-mix(in srgb, var(--accent-soft) 26%, transparent);
  animation: startup-highlight-pulse 1.2s ease-in-out infinite;
}

.startupMetricNote {
  width: 38%;
  height: 8px;
  border-radius: 999px;
  background: color-mix(in oklab, var(--muted) 48%, var(--surface-inline));
}

.startupStatusRail {
  position: absolute;
  left: clamp(1rem, 3.4vw, 2.4rem);
  bottom: clamp(1.2rem, 4vh, 2rem);
  display: flex;
  align-items: center;
  gap: 0.8rem;
  min-width: 0;
  opacity: 0.92;
}

.startupStatus {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  min-width: 0;
  font-size: 0.8rem;
  color: var(--muted-strong);
}

.startupStatusDot {
  width: 0.62rem;
  height: 0.62rem;
  border-radius: 999px;
  background: color-mix(in oklab, var(--accent) 70%, var(--selection-fill));
  box-shadow: 0 0 0 6px color-mix(in srgb, var(--accent-soft) 12%, transparent);
  animation: startup-status-pulse 1.3s ease-in-out infinite;
}

.startupSkipButton {
  pointer-events: auto;
  min-height: 34px;
  padding: 0.45rem 0.72rem;
  border-radius: 10px;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  background: color-mix(in oklab, var(--surface-inline) 84%, var(--panel-footer));
  color: var(--text);
  box-shadow: var(--shadow-inline);
  font-size: 0.76rem;
}

.startupSkipButton:hover {
  background: color-mix(in oklab, var(--surface-focus) 88%, var(--panel-footer));
}

.startupOverlay.reduced .startupStatusDot,
.startupOverlay.reduced .startupGridCell,
.startupOverlay.reduced .startupSweep,
.startupOverlay.reduced .startupMetricBar.highlight {
  animation: none;
}

@media (prefers-reduced-motion: reduce) {
  .startupOverlay,
  .startupGridCell,
  .startupSweep,
  .startupMetricBar.highlight,
  .startupStatusDot {
    animation: none;
    transition: none;
  }
}

@media (max-width: 760px) {
  .startupBrand {
    right: 1rem;
    max-width: none;
  }

  .startupGridField {
    top: calc(var(--desktop-titlebar-h, 52px) + 7rem);
    left: 0.85rem;
    right: 0.85rem;
    bottom: 6.8rem;
  }

  .startupGridCells {
    inset: 18% 9% 24% 9%;
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .startupFocusTrail {
    width: 42vw;
    height: 24%;
  }

  .startupResultRail,
  .startupStatusRail {
    right: 0.85rem;
    left: 0.85rem;
  }

  .startupResultRail {
    bottom: 4.75rem;
  }

  .startupStatusRail {
    bottom: 0.85rem;
    justify-content: space-between;
  }
}

@keyframes startup-grid-cell {
  0%, 100% {
    transform: translateY(0) scale(0.98);
    opacity: 0.42;
  }
  50% {
    transform: translateY(-3px) scale(1);
    opacity: 0.7;
  }
}

@keyframes startup-sweep {
  0% {
    transform: translateX(0);
    opacity: 0;
  }
  18% {
    opacity: 0.78;
  }
  100% {
    transform: translateX(420%);
    opacity: 0;
  }
}

@keyframes startup-highlight-pulse {
  0%, 100% {
    transform: translateX(0);
    filter: saturate(1);
  }
  50% {
    transform: translateX(2px);
    filter: saturate(1.08);
  }
}

@keyframes startup-status-pulse {
  0%, 100% {
    transform: scale(1);
    opacity: 0.84;
  }
  50% {
    transform: scale(1.08);
    opacity: 1;
  }
}
</style>
