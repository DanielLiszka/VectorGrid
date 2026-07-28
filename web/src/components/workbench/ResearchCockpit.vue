<script setup lang="ts">
import { computed } from 'vue'
import { useResearchCockpitModel, type ResearchCockpitStepId } from '../../composables/useResearchCockpitModel'

const { model, openStep } = useResearchCockpitModel()

const visibleCards = computed(() => model.value.cards)

function selectStep(id: ResearchCockpitStepId): void {
  openStep(id)
}
</script>

<template>
  <section class="researchCockpit" aria-label="Research cockpit" data-qa-panel="workbench.researchCockpit">
    <div class="cockpitLead">
      <span class="eyebrow">Workbench</span>
      <h2>{{ model.title }}</h2>
      <p>{{ model.summary }}</p>
    </div>

    <div class="cockpitSteps" role="group" aria-label="Research workflow progress">
      <button
        v-for="step in model.steps"
        :key="step.id"
        type="button"
        class="cockpitStep"
        :class="step.state"
        :data-state="step.state"
        :data-qa-action="step.actionId"
        :aria-current="step.state === 'active' ? 'step' : undefined"
        :title="`${step.label}: ${step.detail}`"
        @click="selectStep(step.id)"
      >
        <span>{{ step.label }}</span>
        <strong>{{ step.detail }}</strong>
      </button>
    </div>

    <div class="cockpitCards" role="region" aria-label="Research status cards" tabindex="0">
      <article
        v-for="card in visibleCards"
        :key="card.id"
        class="cockpitCard"
        :class="card.tone"
        :data-qa-state="`workbench.cockpit.card.${card.id}`"
      >
        <span>{{ card.title }}</span>
        <strong>{{ card.value }}</strong>
        <p>{{ card.detail }}</p>
      </article>
    </div>
  </section>
</template>

<style scoped>
.researchCockpit {
  min-width: 0;
  border-bottom: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  background: color-mix(in oklab, var(--surface-shell) 86%, var(--panel));
  padding: 0.58rem 0.65rem;
  display: grid;
  grid-template-columns: minmax(14rem, 0.72fr) minmax(32rem, 2fr);
  grid-template-areas:
    "lead steps"
    "cards cards";
  gap: 0.5rem 0.58rem;
}

.cockpitLead {
  grid-area: lead;
  min-width: 0;
  display: grid;
  align-content: center;
  gap: 0.08rem;
}

.eyebrow,
.cockpitCard span {
  font-size: var(--text-xs);
  font-weight: 700;
  letter-spacing: 0.09em;
  text-transform: uppercase;
  color: var(--muted);
}

.cockpitLead h2 {
  margin: 0;
  font-size: 0.95rem;
  line-height: 1.16;
}

.cockpitLead p,
.cockpitCard p {
  margin: 0;
  color: var(--muted-strong);
  font-size: var(--text-xs);
  line-height: 1.3;
}

.cockpitLead p {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.cockpitSteps {
  grid-area: steps;
  min-width: 0;
  display: grid;
  grid-template-columns: repeat(6, minmax(0, 1fr));
  gap: 0.32rem;
}

.cockpitStep {
  min-width: 0;
  min-height: 50px;
  border: 1px solid color-mix(in oklab, var(--border) 86%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-working) 94%, var(--panel));
  color: var(--muted-strong);
  padding: 0.36rem 0.42rem;
  display: grid;
  align-content: center;
  gap: 0.08rem;
  text-align: left;
}

.cockpitStep span {
  font-size: var(--text-xs);
  font-weight: 750;
}

.cockpitStep strong {
  min-width: 0;
  font-size: var(--text-xs);
  line-height: 1.15;
  color: var(--muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.cockpitStep.active {
  color: var(--text);
  border-color: color-mix(in oklab, var(--panel-focus-edge) 74%, var(--border));
  background: color-mix(in oklab, var(--selection-fill) 86%, var(--surface-working));
  box-shadow: inset 0 -2px 0 var(--selection-rail);
}

.cockpitStep.complete {
  border-color: color-mix(in oklab, var(--ok) 42%, var(--border));
}

.cockpitStep.blocked {
  opacity: 0.66;
}

.cockpitCards {
  grid-area: cards;
  min-width: 0;
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 0.4rem;
}

.cockpitCard {
  min-width: 0;
  border: 1px solid color-mix(in oklab, var(--border) 86%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-working) 94%, var(--panel));
  padding: 0.45rem 0.5rem;
  display: grid;
  gap: 0.08rem;
}

.cockpitCard strong {
  min-width: 0;
  font-size: 0.78rem;
  line-height: 1.16;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.cockpitCard p {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.cockpitCard.accent {
  border-color: color-mix(in oklab, var(--panel-focus-edge) 50%, var(--border));
}

.cockpitCard.success {
  background: color-mix(in oklab, var(--surface-success) 62%, var(--surface-working));
}

.cockpitCard.warning {
  background: color-mix(in oklab, var(--surface-warning) 62%, var(--surface-working));
}

.cockpitCard.danger {
  background: color-mix(in oklab, var(--surface-danger) 62%, var(--surface-working));
}

@media (max-width: 1180px) {
  .researchCockpit {
    grid-template-columns: minmax(10rem, 0.66fr) minmax(24rem, 1.6fr);
  }
}

@media (max-width: 780px) {
  .researchCockpit {
    padding: 0.48rem 0.5rem;
    grid-template-columns: minmax(0, 1fr);
    grid-template-areas:
      "lead"
      "steps"
      "cards";
  }

  .cockpitSteps {
    overflow-x: auto;
    grid-template-columns: repeat(6, minmax(7.2rem, 1fr));
    scrollbar-width: thin;
  }

  .cockpitCards {
    overflow-x: auto;
    grid-template-columns: repeat(5, minmax(9rem, 70vw));
    scrollbar-width: thin;
  }
}

@media (max-width: 460px) {
  .researchCockpit {
    grid-template-columns: minmax(0, 1fr);
  }
}

@media (max-height: 760px) {
  .researchCockpit {
    padding-block: 0.38rem;
    gap: 0.36rem 0.48rem;
  }

  .cockpitLead p,
  .cockpitCard p {
    display: none;
  }

  .cockpitStep {
    min-height: 40px;
    padding-block: 0.28rem;
  }

  .cockpitCard {
    padding-block: 0.34rem;
  }
}

@media (max-height: 680px) {
  .researchCockpit {
    padding-block: 0.2rem;
    gap: 0.24rem 0.42rem;
  }

  .cockpitCards {
    display: none;
  }

  .cockpitStep {
    min-height: 30px;
    padding-block: 0.18rem;
  }

  .cockpitStep strong,
  .cockpitLead .eyebrow {
    display: none;
  }
}
</style>
