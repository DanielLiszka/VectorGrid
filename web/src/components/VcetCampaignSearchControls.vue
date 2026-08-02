<script setup lang="ts">
import { computed } from "vue";
import {
  MAX_SAFE_VCET_SEARCH_SEED,
  MAX_VCET_SEARCH_BUDGET,
  MAX_VCET_SEARCH_CE_MULT,
  buildVcetSearchConfigFromDraft,
  createDefaultVcetSearchDraft,
  normalizeVcetCandidateDesignMode,
  type VcetCandidateDesignMode,
  type VcetSearchDraft,
} from "../services/vcetSearchRequestBuilder";
import AppButton from "./ui/AppButton.vue";

const props = withDefaults(
  defineProps<{
    designMode: VcetCandidateDesignMode | string;
    draft: VcetSearchDraft;
    disabled?: boolean;
  }>(),
  { disabled: false },
);

const emit = defineEmits<{
  "update:design-mode": [value: VcetCandidateDesignMode];
  "update:draft": [value: VcetSearchDraft];
}>();

interface RangeControl {
  key: string;
  title: string;
  description: string;
  minKey: keyof VcetSearchDraft;
  maxKey: keyof VcetSearchDraft;
  stepKey: keyof VcetSearchDraft;
  min: number;
  max: number;
  inputStep: string;
  stepLabel: string;
}

const rangeControls: readonly RangeControl[] = [
  {
    key: "vidya_short",
    title: "VIDYA short period",
    description: "Fast adaptation horizon. Values must be at least 2.",
    minKey: "vidyaShortMin",
    maxKey: "vidyaShortMax",
    stepKey: "vidyaShortStep",
    min: 2,
    max: 65_535,
    inputStep: "1",
    stepLabel: "Step",
  },
  {
    key: "vidya_long",
    title: "VIDYA long period",
    description: "Slow adaptation horizon; every candidate requires Long > Short.",
    minKey: "vidyaLongMin",
    maxKey: "vidyaLongMax",
    stepKey: "vidyaLongStep",
    min: 2,
    max: 65_535,
    inputStep: "1",
    stepLabel: "Step",
  },
  {
    key: "vidya_alpha",
    title: "VIDYA alpha",
    description: "Adaptation strength on [0, 1], quantized to six-decimal precision.",
    minKey: "vidyaAlphaMin",
    maxKey: "vidyaAlphaMax",
    stepKey: "vidyaAlphaQuantum",
    min: 0,
    max: 1,
    inputStep: "0.01",
    stepLabel: "Quantum",
  },
  {
    key: "ce_period",
    title: "Chandelier period",
    description: "Volatility-exit lookback, independent of the VIDYA periods.",
    minKey: "cePeriodMin",
    maxKey: "cePeriodMax",
    stepKey: "cePeriodStep",
    min: 1,
    max: 65_535,
    inputStep: "1",
    stepLabel: "Step",
  },
  {
    key: "ce_mult",
    title: "Chandelier multiplier",
    description: "Positive ATR-distance multiplier with explicit quantization.",
    minKey: "ceMultMin",
    maxKey: "ceMultMax",
    stepKey: "ceMultQuantum",
    min: 0.000001,
    max: MAX_VCET_SEARCH_CE_MULT,
    inputStep: "0.05",
    stepLabel: "Quantum",
  },
  {
    key: "max_trade_bars",
    title: "Maximum trade bars",
    description: "Maximum holding duration before the execution overlay exits.",
    minKey: "maxTradeBarsMin",
    maxKey: "maxTradeBarsMax",
    stepKey: "maxTradeBarsStep",
    min: 1,
    max: 65_535,
    inputStep: "1",
    stepLabel: "Step",
  },
] as const;

const resolvedMode = computed(() =>
  normalizeVcetCandidateDesignMode(props.designMode),
);
const validation = computed(() => {
  if (resolvedMode.value !== "halton") {
    return {
      ok: true as const,
      message:
        "Legacy grid replays every valid Fast < Slow pair from the current Optimization ranges.",
    };
  }
  try {
    const result = buildVcetSearchConfigFromDraft(props.draft);
    return { ok: true as const, message: result.summary };
  } catch (error) {
    return {
      ok: false as const,
      message:
        error instanceof Error ? error.message : "Invalid VCET search design.",
    };
  }
});

function setMode(value: VcetCandidateDesignMode): void {
  if (props.disabled || value === resolvedMode.value) return;
  emit("update:design-mode", value);
}

function updateDraft(
  key: keyof VcetSearchDraft,
  event: Event,
): void {
  const target = event.target as HTMLInputElement | HTMLSelectElement;
  emit("update:draft", { ...props.draft, [key]: target.value });
}

function resetDefaults(): void {
  if (props.disabled) return;
  emit("update:draft", createDefaultVcetSearchDraft());
}
</script>

<template>
  <section
    class="vcetSearchControls"
    aria-labelledby="vcetCandidateDesignHeading"
    :data-design-mode="resolvedMode"
    :data-design-valid="validation.ok"
  >
    <div class="vcetSearchHeader">
      <div>
        <h4 id="vcetCandidateDesignHeading">Candidate design</h4>
        <p>
          Choose the candidate universe once; the sealed design is replayed on
          every selected dataset without truncation.
        </p>
      </div>
      <AppButton
        v-if="resolvedMode === 'halton'"
        size="sm"
        variant="subtle"
        :disabled="disabled"
        data-qa-action="aggregate.batch.vcetSearch.reset"
        @click="resetDefaults"
      >
        Reset search defaults
      </AppButton>
    </div>

    <fieldset class="designModeChoices" :disabled="disabled">
      <legend class="srOnly">VCET candidate design mode</legend>
      <label :class="{ active: resolvedMode === 'period_grid' }">
        <input
          type="radio"
          name="vcetCandidateDesignMode"
          value="period_grid"
          :checked="resolvedMode === 'period_grid'"
          @change="setMode('period_grid')"
        />
        <span>
          <strong>Fast×Slow grid</strong>
          <small>Legacy period pairs using the current Optimization ranges</small>
        </span>
      </label>
      <label :class="{ active: resolvedMode === 'halton' }">
        <input
          type="radio"
          name="vcetCandidateDesignMode"
          value="halton"
          :checked="resolvedMode === 'halton'"
          @change="setMode('halton')"
        />
        <span>
          <strong>Bounded multidimensional</strong>
          <small>Deterministic coverage across all effective VCET controls</small>
        </span>
      </label>
    </fieldset>

    <div v-if="resolvedMode === 'halton'" class="haltonControls">
      <div class="budgetGrid">
        <label for="vcetSearchBudget">
          <span>Candidate budget</span>
          <input
            id="vcetSearchBudget"
            type="number"
            min="1"
            :max="MAX_VCET_SEARCH_BUDGET"
            step="1"
            :disabled="disabled"
            :value="draft.budget"
            @input="updateDraft('budget', $event)"
          />
          <small>Also becomes the exact validation candidate cap.</small>
        </label>
        <label for="vcetSearchSeed">
          <span>Design seed</span>
          <input
            id="vcetSearchSeed"
            type="number"
            min="0"
            :max="MAX_SAFE_VCET_SEARCH_SEED"
            step="1"
            :disabled="disabled"
            :value="draft.seed"
            @input="updateDraft('seed', $event)"
          />
          <small>Seeds sampled designs and remains sealed in exhaustive provenance.</small>
        </label>
      </div>

      <div class="rangeGrid">
        <fieldset
          v-for="range in rangeControls"
          :key="range.key"
          class="rangeCard"
          :disabled="disabled"
        >
          <legend>{{ range.title }}</legend>
          <p>{{ range.description }}</p>
          <div class="rangeInputs">
            <label :for="`vcetSearch_${range.key}_min`">
              <span>Minimum</span>
              <input
                :id="`vcetSearch_${range.key}_min`"
                type="number"
                :min="range.min"
                :max="range.max"
                :step="range.inputStep"
                :value="draft[range.minKey]"
                @input="updateDraft(range.minKey, $event)"
              />
            </label>
            <label :for="`vcetSearch_${range.key}_max`">
              <span>Maximum</span>
              <input
                :id="`vcetSearch_${range.key}_max`"
                type="number"
                :min="range.min"
                :max="range.max"
                :step="range.inputStep"
                :value="draft[range.maxKey]"
                @input="updateDraft(range.maxKey, $event)"
              />
            </label>
            <label :for="`vcetSearch_${range.key}_step`">
              <span>{{ range.stepLabel }}</span>
              <input
                :id="`vcetSearch_${range.key}_step`"
                type="number"
                :min="range.key.includes('alpha') || range.key.includes('mult') ? 0.000001 : 1"
                :max="range.max"
                :step="range.inputStep"
                :value="draft[range.stepKey]"
                @input="updateDraft(range.stepKey, $event)"
              />
            </label>
          </div>
        </fieldset>
      </div>

      <div class="booleanGrid">
        <label for="vcetSearchCeUseClose">
          <span>Chandelier source</span>
          <select
            id="vcetSearchCeUseClose"
            :disabled="disabled"
            :value="draft.ceUseClose"
            @change="updateDraft('ceUseClose', $event)"
          >
            <option value="both">Include close and high/low</option>
            <option value="true">Close only</option>
            <option value="false">High/low only</option>
          </select>
        </label>
        <label for="vcetSearchRequireSlope">
          <span>VIDYA slope gate</span>
          <select
            id="vcetSearchRequireSlope"
            :disabled="disabled"
            :value="draft.requireVidyaSlope"
            @change="updateDraft('requireVidyaSlope', $event)"
          >
            <option value="both">Include enabled and disabled</option>
            <option value="true">Enabled only</option>
            <option value="false">Disabled only</option>
          </select>
        </label>
      </div>
    </div>

    <div
      class="designValidation"
      :class="validation.ok ? 'valid' : 'invalid'"
      role="status"
      aria-live="polite"
      aria-atomic="true"
    >
      <strong>{{ validation.ok ? "Design ready" : "Design needs attention" }}</strong>
      <span>{{ validation.message }}</span>
    </div>
  </section>
</template>

<style scoped>
.vcetSearchControls {
  display: grid;
  gap: 0.7rem;
}

.vcetSearchHeader {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.8rem;
}

.vcetSearchHeader h4 {
  margin: 0;
  font-size: 0.9rem;
}

.vcetSearchHeader p,
.rangeCard p {
  margin: 0.25rem 0 0;
  color: var(--muted);
  font-size: 0.75rem;
  line-height: 1.35;
}

.designModeChoices {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.55rem;
  border: 0;
  margin: 0;
  padding: 0;
}

.designModeChoices > label {
  display: flex;
  align-items: flex-start;
  gap: 0.55rem;
  border: 1px solid var(--border);
  border-radius: 11px;
  background: var(--surface);
  padding: 0.62rem;
  cursor: pointer;
}

.designModeChoices > label.active {
  border-color: var(--focus-border);
  background: var(--selection-fill);
  box-shadow: inset 3px 0 0 var(--selection-rail);
}

.designModeChoices input {
  margin-top: 0.14rem;
}

.designModeChoices span {
  display: grid;
  gap: 0.16rem;
}

.designModeChoices strong {
  color: var(--text);
  font-size: 0.8rem;
}

.designModeChoices small,
.budgetGrid small {
  color: var(--muted);
  font-size: 0.7rem;
  line-height: 1.3;
}

.haltonControls {
  display: grid;
  gap: 0.62rem;
}

.budgetGrid,
.booleanGrid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.55rem;
}

.budgetGrid label,
.booleanGrid label {
  display: grid;
  gap: 0.3rem;
  color: var(--muted-strong);
  font-size: 0.72rem;
  font-weight: 700;
}

.rangeGrid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.55rem;
}

.rangeCard {
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: 11px;
  background: color-mix(in oklab, var(--surface) 90%, transparent);
  margin: 0;
  padding: 0.58rem;
}

.rangeCard legend {
  padding: 0 0.25rem;
  color: var(--text);
  font-size: 0.76rem;
  font-weight: 700;
}

.rangeInputs {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.38rem;
  margin-top: 0.5rem;
}

.rangeInputs label {
  display: grid;
  gap: 0.2rem;
  color: var(--muted);
  font-size: 0.66rem;
}

.rangeInputs input {
  min-width: 0;
  width: 100%;
}

.designValidation {
  display: grid;
  gap: 0.18rem;
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 0.55rem 0.62rem;
  font-size: 0.74rem;
  line-height: 1.35;
}

.designValidation.valid {
  border-color: color-mix(in oklab, var(--ok) 42%, var(--border));
  background: var(--surface-success);
  color: var(--ok-ink);
}

.designValidation.invalid {
  border-color: color-mix(in oklab, var(--danger) 42%, var(--border));
  background: var(--surface-danger);
  color: var(--danger-ink);
}

.srOnly {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}

@media (max-width: 900px) {
  .rangeGrid,
  .budgetGrid,
  .booleanGrid {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 640px) {
  .vcetSearchHeader,
  .designModeChoices {
    display: grid;
    grid-template-columns: 1fr;
  }

  .rangeInputs {
    grid-template-columns: 1fr;
  }
}
</style>
