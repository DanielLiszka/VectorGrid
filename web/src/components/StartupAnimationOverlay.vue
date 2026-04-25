<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useMediaQuery } from '@vueuse/core'
import VectorAlphaLogo from './VectorAlphaLogo.vue'
import { BOOT_STAGES, useRuntimeStore } from '../stores/runtimeStore'

type OverlayStage = 'hidden' | 'intro' | 'exiting' | 'dismissed'
type StepTone = 'pending' | 'active' | 'done' | 'error'

type StageStep = {
  key: string
  label: string
  tone: StepTone
}

const MIN_VISIBLE_MS = 360
const SIGNATURE_MS = 900
const EXIT_MS = 180
const SKIP_UNLOCK_MS = MIN_VISIBLE_MS
const HOLD_QUERY_VALUE = 'hold'
const HOLD_QUERY_KEY = 'startup_overlay'
const HOLD_STORAGE_KEY = 'VTA_STARTUP_OVERLAY_HOLD'

const STAGE_STRIP = [
  { key: 'restore', label: 'Restore' },
  { key: 'desktop', label: 'Desktop' },
  { key: 'shell', label: 'Shell' },
] as const

const runtimeStore = useRuntimeStore()
const { bootPhase, bootStage } = storeToRefs(runtimeStore)
const prefersReducedMotion = useMediaQuery('(prefers-reduced-motion: reduce)')

const overlayStage = ref<OverlayStage>('hidden')
const canSkip = ref(false)
const introStartedAt = ref<number | null>(null)
const lastBootingStage = ref<string>(BOOT_STAGES.RESTORING_WORKSPACE_STATE)

let skipUnlockTimer: number | null = null
let signatureTimer: number | null = null
let exitTimer: number | null = null
let readyDelayTimer: number | null = null
let keyListenerBound = false

function clearTimer(timer: number | null): void {
  if (timer !== null) window.clearTimeout(timer)
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
    if (sessionStorage.getItem(HOLD_STORAGE_KEY) === '1') return true
    if (localStorage.getItem(HOLD_STORAGE_KEY) === '1') {


      localStorage.removeItem(HOLD_STORAGE_KEY)
    }
    return false
  } catch {
    return false
  }
}

function resolveStageIndex(stage: string): number {
  if (stage === BOOT_STAGES.INITIALIZING_DESKTOP_SERVICES) return 1
  if (stage === BOOT_STAGES.MOUNTING_WORKSPACE_SHELL || stage === BOOT_STAGES.FRONTEND_RUNTIME_READY) return 2
  return 0
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
  if (bootPhase.value === 'error') return 'Error'
  if (overlayStage.value === 'exiting' && bootPhase.value === 'ready') return 'Ready'
  if (overlayStage.value === 'exiting') return 'Finishing'
  return 'Booting'
})
const stageReference = computed(() => {
  if (bootPhase.value === 'error') return lastBootingStage.value
  return String(bootStage.value || '').trim()
})
const stageSteps = computed<StageStep[]>(() => {
  const activeIndex = resolveStageIndex(stageReference.value)
  return STAGE_STRIP.map((step, index) => {
    let tone: StepTone = 'pending'

    if (bootPhase.value === 'ready') {
      tone = 'done'
    } else if (bootPhase.value === 'error') {
      if (index < activeIndex) tone = 'done'
      else if (index === activeIndex) tone = 'error'
    } else if (index < activeIndex) {
      tone = 'done'
    } else if (index === activeIndex) {
      tone = 'active'
    }

    return {
      key: step.key,
      label: step.label,
      tone,
    }
  })
})
const assistLabel = computed(() => {
  if (bootPhase.value === 'error') return 'Handing off to runtime diagnostics.'
  if (overlayStage.value === 'exiting' && bootPhase.value === 'ready') return 'Workspace live. Releasing the startup veil.'
  if (canSkip.value) return 'Dismiss the veil without interrupting real startup.'
  return 'Shell is already mounted underneath while startup finishes.'
})
const skipHintVisible = computed(() => canSkip.value && bootPhase.value !== 'error')
const skipButtonLabel = computed(() => (bootPhase.value === 'ready' ? 'Enter workspace' : 'Dismiss'))

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
  if (event.key !== 'Escape' && event.key !== 'Enter' && event.key !== ' ') return
  event.preventDefault()
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
  [bootPhase, bootStage],
  ([phase, stage]) => {
    const stageText = String(stage || '').trim()
    if (phase === 'booting' && stageText) {
      lastBootingStage.value = stageText
    }
  },
  { immediate: true },
)

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
    <div class="startupSurvey" aria-hidden="true">
      <span class="startupSurveyLine startupSurveyLine--top"></span>
      <span class="startupSurveyLine startupSurveyLine--mid"></span>
      <span class="startupSurveyGlow"></span>
    </div>
    <section class="startupScene" role="region" aria-label="VectorGrid startup overlay">
      <div class="startupRail" :class="railTone" role="status" aria-live="polite">
        <div class="startupRailAccent" aria-hidden="true"></div>
        <div class="startupMarkCluster" aria-hidden="true">
          <div class="startupMarkHalo"></div>
          <div class="startupMarkPlate">
            <VectorAlphaLogo class="startupMarkLogo" decorative :size="40" />
          </div>
        </div>
        <div class="startupCopy">
          <div class="startupMetaRow">
            <span class="startupProductLabel">VectorGrid</span>
            <span class="startupPhaseTag">{{ phaseLabel }}</span>
          </div>
          <p class="startupStageLine">{{ stageLabel }}</p>
          <div class="startupAssistRow">
            <p class="startupAssistLabel">{{ assistLabel }}</p>
            <div class="startupStageSteps" aria-hidden="true">
              <span
                v-for="step in stageSteps"
                :key="step.key"
                class="startupStageStep"
                :class="step.tone"
              >
                <span class="startupStageDot"></span>
                <span class="startupStageText">{{ step.label }}</span>
              </span>
            </div>
          </div>
        </div>
        <button
          v-if="skipHintVisible"
          type="button"
          class="startupSkipButton"
          @click.stop="requestSkip"
        >
          {{ skipButtonLabel }}
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
  isolation: isolate;
  pointer-events: auto;
  transition: opacity 180ms var(--ease-standard, cubic-bezier(0.2, 0, 0, 1));
}

.startupOverlay.exiting {
  opacity: 0;
}

.startupVeil {
  position: absolute;
  inset: 0;
  background:
    radial-gradient(circle at 12% 10%, color-mix(in oklab, var(--accent-soft) 42%, transparent), transparent 34%),
    radial-gradient(circle at 84% 12%, color-mix(in oklab, var(--surface-accent) 68%, transparent), transparent 22%),
    linear-gradient(
      180deg,
      color-mix(in oklab, var(--header-bg) 44%, transparent),
      color-mix(in oklab, var(--surface-sticky) 22%, transparent) 26%,
      color-mix(in oklab, var(--surface-overlay) 14%, transparent) 58%,
      color-mix(in oklab, var(--surface-sticky) 12%, transparent) 100%
    );
  backdrop-filter: blur(14px) saturate(1.05);
  opacity: 0.96;
  pointer-events: none;
}

.startupSurvey {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.startupSurveyLine,
.startupSurveyGlow {
  position: absolute;
  opacity: 0;
}

.startupSurveyLine {
  left: 0;
  right: 0;
  height: 1px;
  background: linear-gradient(
    90deg,
    color-mix(in oklab, var(--accent) 0%, transparent) 0%,
    color-mix(in oklab, var(--accent) 26%, transparent) 14%,
    color-mix(in oklab, var(--border-strong) 28%, transparent) 48%,
    transparent 100%
  );
  animation: startupFieldReveal 300ms var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)) both;
}

.startupSurveyLine--top {
  top: calc(var(--desktop-titlebar-h, 0px) + 14px);
}

.startupSurveyLine--mid {
  top: calc(var(--desktop-titlebar-h, 0px) + 76px);
  animation-delay: 80ms;
}

.startupSurveyGlow {
  top: calc(var(--desktop-titlebar-h, 0px) + 4px);
  left: -6%;
  width: min(68rem, 94vw);
  height: 12rem;
  background: radial-gradient(circle at 18% 36%, color-mix(in oklab, var(--accent-soft-2) 88%, transparent), transparent 58%);
  filter: blur(26px);
  animation: startupGlowReveal 360ms var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)) both;
}

.startupScene {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.startupRail {
  --startup-rail-accent: var(--accent);
  position: absolute;
  top: calc(var(--desktop-titlebar-h, 0px) + 14px);
  left: 12px;
  width: min(64ch, calc(100vw - 24px));
  display: grid;
  grid-template-columns: auto auto minmax(0, 1fr) auto;
  gap: 0.9rem;
  align-items: center;
  padding: 0.82rem 0.98rem;
  border-radius: 16px;
  border: 1px solid color-mix(in oklab, var(--startup-rail-accent) 22%, var(--border-strong));
  background:
    linear-gradient(
      155deg,
      color-mix(in oklab, var(--surface-overlay) 92%, var(--surface-working)),
      color-mix(in oklab, var(--panel-footer) 96%, var(--surface-working))
    );
  box-shadow:
    0 18px 34px rgb(var(--shadow-overflow-rgb, 15 23 42) / 0.13),
    inset 0 1px 0 rgb(255 255 255 / 0.18);
  backdrop-filter: blur(16px);
  animation: startupRailReveal 280ms var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)) both;
}

.startupRail::after {
  content: '';
  position: absolute;
  left: 1.3rem;
  right: 1.3rem;
  bottom: 0;
  height: 1px;
  background: linear-gradient(
    90deg,
    color-mix(in oklab, var(--startup-rail-accent) 30%, transparent),
    color-mix(in oklab, var(--startup-rail-accent) 10%, transparent) 54%,
    transparent 100%
  );
  opacity: 0.9;
}

.startupRail.settling {
  --startup-rail-accent: var(--accent-hover);
}

.startupRail.error {
  --startup-rail-accent: var(--danger);
}

.startupRailAccent {
  width: 4px;
  align-self: stretch;
  border-radius: 999px;
  background: linear-gradient(
    180deg,
    color-mix(in srgb, white 18%, var(--startup-rail-accent)),
    color-mix(in srgb, black 4%, var(--startup-rail-accent))
  );
  box-shadow:
    0 0 0 1px color-mix(in oklab, var(--startup-rail-accent) 18%, transparent),
    0 0 18px color-mix(in oklab, var(--startup-rail-accent) 22%, transparent);
}

.startupMarkCluster {
  position: relative;
  width: 3rem;
  height: 3rem;
}

.startupMarkHalo {
  position: absolute;
  inset: -18%;
  border-radius: 18px;
  background: radial-gradient(circle, color-mix(in oklab, var(--startup-rail-accent) 18%, transparent), transparent 72%);
}

.startupMarkPlate {
  position: relative;
  width: 100%;
  height: 100%;
  display: grid;
  place-items: center;
  border-radius: 14px;
  border: 1px solid color-mix(in oklab, var(--startup-rail-accent) 18%, var(--border));
  background:
    linear-gradient(180deg, rgb(255 255 255 / 0.1), transparent 42%),
    color-mix(in oklab, var(--surface-overlay) 86%, var(--surface-sunken));
  box-shadow:
    inset 0 1px 0 rgb(255 255 255 / 0.18),
    0 10px 18px rgb(var(--shadow-overflow-rgb, 15 23 42) / 0.1);
}

.startupMarkLogo {
  opacity: 0.98;
}

.startupCopy {
  min-width: 0;
  display: grid;
  gap: 0.26rem;
}

.startupMetaRow {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  min-width: 0;
}

.startupProductLabel {
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--muted-soft);
}

.startupPhaseTag {
  display: inline-flex;
  align-items: center;
  min-width: 0;
  padding: 0.16rem 0.42rem;
  border-radius: 999px;
  border: 1px solid color-mix(in oklab, var(--startup-rail-accent) 22%, var(--border));
  background: color-mix(in oklab, var(--startup-rail-accent) 7%, var(--surface-inline));
  font-size: 0.65rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text);
}

.startupStageLine {
  margin: 0;
  font-size: 0.98rem;
  font-weight: 650;
  line-height: 1.28;
  color: var(--text);
  text-wrap: pretty;
}

.startupAssistRow {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 0.65rem 0.9rem;
}

.startupAssistLabel {
  margin: 0;
  font-size: 0.77rem;
  line-height: 1.35;
  color: var(--muted-strong);
}

.startupStageSteps {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 0.42rem;
}

.startupStageStep {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  min-width: 0;
  padding: 0.18rem 0.42rem;
  border-radius: 999px;
  border: 1px solid color-mix(in oklab, var(--border) 84%, transparent);
  background: color-mix(in oklab, var(--surface-working) 90%, var(--surface-inline));
  color: var(--muted);
}

.startupStageDot {
  width: 0.45rem;
  height: 0.45rem;
  border-radius: 50%;
  background: color-mix(in oklab, var(--border-strong) 70%, transparent);
}

.startupStageText {
  font-size: 0.64rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.startupStageStep.active,
.startupStageStep.done,
.startupStageStep.error {
  border-color: color-mix(in oklab, var(--startup-rail-accent) 18%, var(--border));
  color: var(--text);
}

.startupStageStep.active {
  background: color-mix(in oklab, var(--startup-rail-accent) 8%, var(--surface-inline));
}

.startupStageStep.done {
  background: color-mix(in oklab, var(--startup-rail-accent) 10%, var(--surface-working));
}

.startupStageStep.error {
  background: color-mix(in oklab, var(--danger-soft) 78%, var(--surface-working));
}

.startupStageStep.active .startupStageDot,
.startupStageStep.done .startupStageDot {
  background: color-mix(in oklab, var(--startup-rail-accent) 78%, white 10%);
}

.startupStageStep.error .startupStageDot {
  background: color-mix(in oklab, var(--danger) 88%, white 8%);
}

.startupSkipButton {
  pointer-events: auto;
  min-height: 38px;
  padding: 0.5rem 0.82rem;
  border-radius: 12px;
  border: 1px solid color-mix(in oklab, var(--startup-rail-accent) 18%, var(--border));
  background:
    linear-gradient(180deg, rgb(255 255 255 / 0.12), transparent 48%),
    color-mix(in oklab, var(--surface-inline) 88%, var(--panel-footer));
  color: var(--text);
  box-shadow: var(--shadow-inline);
  font-size: 0.76rem;
  font-weight: 700;
  letter-spacing: 0.04em;
}

.startupSkipButton:hover {
  background:
    linear-gradient(180deg, rgb(255 255 255 / 0.14), transparent 48%),
    color-mix(in oklab, var(--surface-focus) 90%, var(--panel-footer));
}

@keyframes startupRailReveal {
  from {
    opacity: 0;
    transform: translateY(-10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes startupFieldReveal {
  from {
    opacity: 0;
    transform: translateY(-4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes startupGlowReveal {
  from {
    opacity: 0;
    transform: translateY(-8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@media (prefers-reduced-motion: reduce) {
  .startupSurveyLine,
  .startupSurveyGlow,
  .startupRail {
    animation: none;
  }
}

@media (max-width: 900px) {
  .startupRail {
    grid-template-columns: auto auto minmax(0, 1fr);
  }

  .startupSkipButton {
    grid-column: 1 / -1;
    justify-self: start;
  }

  .startupAssistRow {
    align-items: flex-start;
  }

  .startupStageSteps {
    justify-content: flex-start;
  }
}

@media (max-width: 620px) {
  .startupRail {
    left: 10px;
    right: 10px;
    width: auto;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 0.75rem;
    padding: 0.78rem 0.82rem;
  }

  .startupRailAccent {
    grid-row: 1 / span 2;
  }

  .startupMarkCluster {
    width: 2.75rem;
    height: 2.75rem;
  }

  .startupCopy {
    grid-column: 2;
  }

  .startupSkipButton {
    grid-column: 1 / -1;
    width: 100%;
  }

  .startupStageLine {
    font-size: 0.9rem;
  }
}
</style>
