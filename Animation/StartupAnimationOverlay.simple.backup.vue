<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useMediaQuery } from '@vueuse/core'
import VectorAlphaLogo from './VectorAlphaLogo.vue'
import { useRuntimeStore } from '../stores/runtimeStore'

type OverlayStage = 'hidden' | 'intro' | 'exiting' | 'dismissed'

const MIN_VISIBLE_MS = 360
const SIGNATURE_MS = 900
const EXIT_MS = 180
const SKIP_UNLOCK_MS = MIN_VISIBLE_MS
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
const railTone = computed(() => {
  if (bootPhase.value === 'error') return 'error'
  if (overlayStage.value === 'exiting') return 'settling'
  return 'booting'
})
const stageLabel = computed(() => String(bootStage.value || '').trim() || 'Starting VectorGrid')
const phaseLabel = computed(() => {
  if (bootPhase.value === 'error') return 'Failed'
  if (overlayStage.value === 'exiting' && bootPhase.value === 'ready') return 'Ready'
  if (overlayStage.value === 'exiting') return 'Finishing'
  return 'Booting'
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
    <section class="startupScene" role="region" aria-label="VectorGrid startup overlay">
      <div class="startupRail" :class="railTone" role="status" aria-live="polite">
        <div class="startupRailAccent" aria-hidden="true"></div>
        <div class="startupMark" aria-hidden="true">
          <VectorAlphaLogo class="startupMarkLogo" decorative :size="34" />
        </div>
        <div class="startupCopy">
          <div class="startupMetaRow">
            <span class="startupProductLabel">VectorGrid</span>
            <span class="startupPhaseTag">{{ phaseLabel }}</span>
          </div>
          <p class="startupStageLine">{{ stageLabel }}</p>
        </div>
        <button
          v-if="skipHintVisible"
          type="button"
          class="startupSkipButton"
          @click.stop="requestSkip"
        >
          Dismiss
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
  overflow: hidden;
  opacity: 1;
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
    radial-gradient(circle at 18% 16%, color-mix(in oklab, var(--accent-soft) 24%, transparent), transparent 28%),
    linear-gradient(
      180deg,
      color-mix(in oklab, var(--surface-sticky) 36%, transparent),
      color-mix(in oklab, var(--header-bg) 18%, transparent) 42%,
      color-mix(in oklab, var(--surface-sticky) 52%, transparent)
    );
  backdrop-filter: blur(10px) saturate(1.04);
  opacity: 0.9;
  pointer-events: none;
}

.startupScene {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.startupRail {
  --startup-rail-accent: var(--accent);
  position: absolute;
  top: calc(var(--desktop-titlebar-h, 0px) + 10px);
  left: 10px;
  max-width: min(58ch, calc(100vw - 20px));
  display: grid;
  grid-template-columns: auto auto minmax(0, 1fr) auto;
  gap: 0.72rem;
  align-items: center;
  padding: 0.65rem 0.8rem;
  border-radius: 12px;
  border: 1px solid color-mix(in oklab, var(--startup-rail-accent) 34%, var(--border));
  background: color-mix(in oklab, var(--surface-sticky) 96%, var(--panel-footer));
  box-shadow: var(--shadow-focus);
  backdrop-filter: blur(8px);
}

.startupRail.settling {
  --startup-rail-accent: var(--accent-soft);
}

.startupRail.error {
  --startup-rail-accent: var(--danger);
}

.startupRailAccent {
  width: 3px;
  align-self: stretch;
  border-radius: 999px;
  background:
    linear-gradient(
      180deg,
      color-mix(in srgb, white 14%, var(--startup-rail-accent)),
      color-mix(in srgb, black 8%, var(--startup-rail-accent))
    );
  box-shadow: 0 0 0 1px color-mix(in oklab, var(--startup-rail-accent) 16%, transparent);
}

.startupMark {
  width: 2.4rem;
  height: 2.4rem;
  display: grid;
  place-items: center;
  border-radius: 10px;
  background:
    radial-gradient(circle at 50% 50%, color-mix(in oklab, var(--surface-working) 72%, transparent), transparent 70%),
    color-mix(in oklab, var(--surface-sunken) 52%, transparent);
  box-shadow:
    inset 0 0 0 1px color-mix(in oklab, var(--border) 60%, transparent),
    0 0 0 1px color-mix(in srgb, white 3%, transparent);
}

.startupMarkLogo {
  opacity: 0.98;
}

.startupCopy {
  min-width: 0;
  display: grid;
  gap: 0.15rem;
}

.startupMetaRow {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  min-width: 0;
}

.startupProductLabel {
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted-soft);
}

.startupPhaseTag {
  display: inline-flex;
  align-items: center;
  min-width: 0;
  padding: 0.14rem 0.38rem;
  border-radius: 999px;
  border: 1px solid color-mix(in oklab, var(--startup-rail-accent) 28%, var(--border));
  background: color-mix(in oklab, var(--startup-rail-accent) 8%, var(--surface-inline));
  font-size: 0.66rem;
  font-weight: 700;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--text);
}

.startupStageLine {
  margin: 0;
  font-size: 0.84rem;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.startupSkipButton {
  pointer-events: auto;
  min-height: 34px;
  padding: 0.42rem 0.72rem;
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

@media (prefers-reduced-motion: reduce) {
  .startupOverlay {
    transition: none;
  }
}

@media (max-width: 720px) {
  .startupRail {
    left: 10px;
    right: 10px;
    max-width: none;
    grid-template-columns: auto auto minmax(0, 1fr);
  }

  .startupSkipButton {
    grid-column: 1 / -1;
    justify-self: start;
  }
}
</style>
