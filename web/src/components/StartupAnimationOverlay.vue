<script setup lang="ts">
import { computed, onBeforeUnmount, watch, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useMediaQuery } from '@vueuse/core'
import animatedLogoUrl from '../assets/branding/vectoralpha-animated.svg'
import staticLogoUrl from '../assets/branding/vectoralpha-static.svg'
import { useRuntimeStore } from '../stores/runtimeStore'

type OverlayStage = 'hidden' | 'intro' | 'exiting' | 'dismissed'

const MIN_VISIBLE_MS = 360
const SIGNATURE_MS = 1400
const EXIT_MS = 180
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
const overlayStageAttr = computed(() => overlayStage.value)

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
  }, MIN_VISIBLE_MS)

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
    :data-stage="overlayStageAttr"
    @click="requestSkip"
  >
    <section
      class="startupShell"
      role="status"
      aria-live="polite"
      aria-label="VectorGrid startup"
      @click.stop
    >
      <div class="startupBackdrop" aria-hidden="true"></div>

      <div class="startupBrand">
        <div class="startupLogoFrame">
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

      <div class="startupNarrative" aria-hidden="true">
        <div class="startupGridAccent">
          <span v-for="index in 20" :key="`startup_grid_${index}`" class="startupGridDot"></span>
        </div>
        <div class="startupResultAccent">
          <span class="startupMetricBar"></span>
          <span class="startupMetricBar highlight"></span>
          <span class="startupMetricBar"></span>
          <span class="startupMetricNote"></span>
        </div>
      </div>

      <div class="startupFooter">
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
  position: fixed;
  inset: 0;
  z-index: 5000;
  display: grid;
  place-items: center;
  padding: max(1.2rem, env(safe-area-inset-top)) max(1rem, env(safe-area-inset-right))
    max(1rem, env(safe-area-inset-bottom)) max(1rem, env(safe-area-inset-left));
  background:
    radial-gradient(circle at 18% 18%, color-mix(in oklab, var(--accent-soft) 32%, transparent), transparent 34%),
    radial-gradient(circle at 82% 18%, color-mix(in oklab, var(--selection-fill) 18%, transparent), transparent 30%),
    linear-gradient(180deg, color-mix(in oklab, var(--surface-sticky) 96%, black 4%), color-mix(in oklab, var(--header-bg) 88%, black 12%));
  backdrop-filter: blur(18px);
  opacity: 1;
  transition: opacity 180ms var(--ease-standard, cubic-bezier(0.2, 0, 0, 1));
  pointer-events: auto;
}

.startupOverlay.exiting {
  opacity: 0;
}

.startupShell {
  position: relative;
  width: min(640px, calc(100vw - 2rem));
  display: grid;
  gap: 1.15rem;
  padding: 1.35rem 1.4rem 1.15rem;
  border-radius: 24px;
  border: 1px solid color-mix(in oklab, var(--panel-focus-edge) 26%, var(--border));
  background:
    linear-gradient(180deg, color-mix(in oklab, var(--surface-working) 96%, transparent), color-mix(in oklab, var(--surface-inline) 92%, transparent)),
    color-mix(in oklab, var(--surface-sticky) 92%, var(--panel-footer));
  box-shadow:
    0 32px 80px color-mix(in srgb, black 46%, transparent),
    inset 0 1px 0 color-mix(in srgb, white 8%, transparent);
  overflow: hidden;
}

.startupBackdrop {
  position: absolute;
  inset: 0;
  background:
    linear-gradient(120deg, transparent 0%, color-mix(in oklab, var(--selection-fill) 10%, transparent) 28%, transparent 56%),
    linear-gradient(180deg, transparent, color-mix(in oklab, var(--accent-soft) 10%, transparent));
  pointer-events: none;
}

.startupBrand,
.startupNarrative,
.startupFooter {
  position: relative;
  z-index: 1;
}

.startupBrand {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 0.95rem;
  align-items: center;
}

.startupLogoFrame {
  width: clamp(74px, 12vw, 96px);
  aspect-ratio: 6 / 5;
  display: grid;
  place-items: center;
  border-radius: 20px;
  background: color-mix(in oklab, var(--surface-sunken) 68%, transparent);
  box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--border) 72%, transparent);
}

.startupLogo {
  width: 78%;
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
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--muted-soft);
}

.startupTitle {
  margin: 0;
  font-size: clamp(1.7rem, 3vw, 2.3rem);
  line-height: 0.96;
  letter-spacing: -0.04em;
  color: var(--text);
}

.startupSubtitle {
  margin: 0;
  font-size: 0.84rem;
  color: var(--muted-strong);
  max-width: 32ch;
}

.startupNarrative {
  display: grid;
  grid-template-columns: minmax(0, 1.15fr) minmax(0, 0.95fr);
  gap: 0.95rem;
  align-items: center;
}

.startupGridAccent,
.startupResultAccent {
  border-radius: 18px;
  border: 1px solid color-mix(in oklab, var(--border) 76%, transparent);
  background: color-mix(in oklab, var(--surface-sunken) 80%, transparent);
}

.startupGridAccent {
  padding: 0.9rem;
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 0.55rem;
}

.startupGridDot {
  width: 100%;
  aspect-ratio: 1;
  border-radius: 999px;
  background:
    radial-gradient(circle at 35% 35%, color-mix(in srgb, white 22%, transparent), transparent 38%),
    color-mix(in oklab, var(--muted-soft) 54%, var(--surface-inline));
  box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--border) 56%, transparent);
  animation: startup-grid-dot 1.4s var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)) infinite;
}

.startupGridDot:nth-child(5n + 2) { animation-delay: 80ms; }
.startupGridDot:nth-child(5n + 3) { animation-delay: 160ms; }
.startupGridDot:nth-child(5n + 4) { animation-delay: 240ms; }
.startupGridDot:nth-child(5n + 5) { animation-delay: 320ms; }

.startupResultAccent {
  padding: 1rem 0.95rem;
  display: grid;
  gap: 0.5rem;
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

.startupFooter {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
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
.startupOverlay.reduced .startupGridDot,
.startupOverlay.reduced .startupMetricBar.highlight {
  animation: none;
}

@media (prefers-reduced-motion: reduce) {
  .startupOverlay,
  .startupGridDot,
  .startupMetricBar.highlight,
  .startupStatusDot {
    animation: none;
    transition: none;
  }
}

@media (max-width: 760px) {
  .startupOverlay {
    padding: 0.85rem;
  }

  .startupShell {
    width: 100%;
    padding: 1rem;
    gap: 0.95rem;
  }

  .startupBrand {
    grid-template-columns: 1fr;
    justify-items: start;
  }

  .startupNarrative {
    grid-template-columns: 1fr;
  }

  .startupFooter {
    align-items: flex-start;
    flex-direction: column;
  }
}

@keyframes startup-grid-dot {
  0%, 100% {
    transform: scale(1);
    opacity: 0.72;
  }
  45% {
    transform: scale(1.12);
    opacity: 1;
    background:
      radial-gradient(circle at 35% 35%, color-mix(in srgb, white 34%, transparent), transparent 38%),
      color-mix(in oklab, var(--selection-fill) 76%, var(--surface-inline));
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
