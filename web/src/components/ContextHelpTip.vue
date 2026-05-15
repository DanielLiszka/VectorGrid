<script setup lang="ts">
import { computed, useId } from 'vue'
import { CircleHelp } from '@lucide/vue'
import {
  TooltipContent,
  TooltipPortal,
  TooltipProvider,
  TooltipRoot,
  TooltipTrigger,
} from 'reka-ui'
import { useHelpMode } from '../composables/useHelpMode'

interface Props {
  title?: string
  summary: string
  detail?: string
  ariaLabel?: string
  badge?: string
  side?: 'top' | 'right' | 'bottom' | 'left'
  align?: 'start' | 'center' | 'end'
}

const props = withDefaults(defineProps<Props>(), {
  title: '',
  detail: '',
  ariaLabel: '',
  badge: '',
  side: 'top',
  align: 'center',
})

const { mode, isFull, isOff } = useHelpMode()

const conciseTooltipDelayMs = 450
const componentId = useId()

const label = computed(() => String(props.ariaLabel || props.title || 'Help').trim() || 'Help')
const detailBody = computed(() => String(props.detail || '').trim())
const textBadge = computed(() => String(props.badge || '').trim())
const idBase = computed(() => `context_help_${componentId.replace(/[^a-zA-Z0-9_-]+/g, '_')}`)
const fullTriggerId = computed(() => `${idBase.value}_full_trigger`)
const fullContentId = computed(() => `${idBase.value}_full_content`)
const fullTitleId = computed(() => `${idBase.value}_full_title`)
const fullSummaryId = computed(() => `${idBase.value}_full_summary`)
const fullDetailId = computed(() => `${idBase.value}_full_detail`)
const conciseContentId = computed(() => `${idBase.value}_concise_content`)
const fullDescriptionIds = computed(() =>
  detailBody.value ? `${fullSummaryId.value} ${fullDetailId.value}` : fullSummaryId.value,
)
const tooltipId = computed(() => (isFull.value ? fullContentId.value : conciseContentId.value))
</script>

<template>
  <span v-if="!isOff" class="contextHelpRoot" :data-help-mode="mode">
    <TooltipProvider>
      <TooltipRoot :delay-duration="conciseTooltipDelayMs">
        <TooltipTrigger as-child>
          <button
            :id="isFull ? fullTriggerId : undefined"
            type="button"
            class="contextHelpTrigger"
            :aria-label="label"
            :aria-describedby="tooltipId"
          >
            <span v-if="textBadge" class="contextHelpBadgeText">{{ textBadge }}</span>
            <CircleHelp v-else class="contextHelpIcon" aria-hidden="true" />
          </button>
        </TooltipTrigger>
        <TooltipPortal>
          <TooltipContent
            :id="tooltipId"
            class="contextHelpCard"
            :class="{ contextHelpTooltip: !isFull }"
            :aria-labelledby="isFull && title ? fullTitleId : undefined"
            :aria-label="isFull && !title ? label : undefined"
            :aria-describedby="isFull ? fullDescriptionIds : undefined"
            :side="side"
            :align="align"
            :side-offset="8"
            :collision-padding="12"
          >
            <div v-if="isFull && title" :id="fullTitleId" class="contextHelpTitle">{{ title }}</div>
            <div :id="isFull ? fullSummaryId : undefined" class="contextHelpSummary">{{ summary }}</div>
            <div v-if="isFull && detailBody" :id="fullDetailId" class="contextHelpDetail">{{ detailBody }}</div>
          </TooltipContent>
        </TooltipPortal>
      </TooltipRoot>
    </TooltipProvider>
  </span>
</template>

<style scoped>
.contextHelpRoot {
  display: inline-flex;
  align-items: center;
}

.contextHelpTrigger {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.5rem;
  height: 1.5rem;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 999px;
  background: color-mix(in oklab, var(--surface-inline) 96%, var(--panel));
  color: var(--muted);
  font-size: 0.76rem;
  font-weight: 700;
  line-height: 1;
  box-shadow: var(--shadow-inline);
  touch-action: manipulation;
  transition:
    transform var(--motion-fast) var(--ease-standard),
    border-color var(--motion-fast) var(--ease-standard),
    background-color var(--motion-fast) var(--ease-standard),
    color var(--motion-fast) var(--ease-standard);
}

.contextHelpBadgeText {
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.contextHelpIcon {
  width: 0.82rem;
  height: 0.82rem;
  stroke-width: 2.1;
}

.contextHelpTrigger:hover {
  transform: translateY(-1px);
  border-color: color-mix(in oklab, var(--panel-focus-edge) 76%, var(--border));
  color: var(--selection-text);
}

.contextHelpTrigger:focus-visible {
  outline: none;
  box-shadow: var(--focus-ring), var(--shadow-inline);
}

.contextHelpCard {
  z-index: 4100;
  width: min(24rem, calc(100vw - 24px));
  border: 1px solid color-mix(in oklab, var(--panel-focus-edge) 72%, var(--border));
  border-radius: 12px;
  background: linear-gradient(
    145deg,
    color-mix(in oklab, var(--surface-focus) 92%, var(--panel-footer)),
    color-mix(in oklab, var(--panel-footer) 94%, var(--surface-working))
  );
  color: var(--text);
  box-shadow: var(--shadow-focus);
  padding: 0.7rem 0.78rem;
  display: grid;
  gap: 0.35rem;
  text-wrap: pretty;
}

.contextHelpTooltip {
  width: min(18rem, calc(100vw - 24px));
}

.contextHelpTitle {
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted);
}

.contextHelpSummary {
  font-size: var(--text-sm);
  line-height: var(--leading-compact);
  color: var(--muted-strong);
}

.contextHelpDetail {
  font-size: var(--text-xs);
  line-height: var(--leading-body);
  color: var(--muted);
}

@media (prefers-reduced-motion: reduce) {
  .contextHelpTrigger {
    transition: none;
  }
}
</style>
