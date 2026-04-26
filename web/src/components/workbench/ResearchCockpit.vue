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

    <div class="cockpitSteps" role="list" aria-label="Research workflow progress">
      <button
        v-for="step in model.steps"
        :key="step.id"
        type="button"
        class="cockpitStep"
        :class="step.state"
        role="listitem"
        :data-state="step.state"
        :data-qa-action="step.actionId"
        @click="selectStep(step.id)"
      >
        <span>{{ step.label }}</span>
        <strong>{{ step.detail }}</strong>
      </button>
    </div>

    <div class="cockpitAction" :class="{ blocked: model.primaryAction.disabled }">
      <span class="eyebrow">{{ model.modeLabel }}</span>
      <strong>{{ model.primaryAction.label }}</strong>
      <p>{{ model.primaryDetail }}</p>
    </div>

    <div class="cockpitCards" aria-label="Research status cards">
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
  grid-template-columns: minmax(14rem, 0.84fr) minmax(22rem, 1.45fr) minmax(15rem, 0.78fr);
  grid-template-areas:
    "lead steps action"
    "cards cards cards";
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
  font-size: 0.64rem;
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
.cockpitAction p,
.cockpitCard p {
  margin: 0;
  color: var(--muted-strong);
  font-size: 0.73rem;
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
  grid-template-columns: repeat(5, minmax(0, 1fr));
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
  font-size: 0.68rem;
  font-weight: 750;
}

.cockpitStep strong {
  min-width: 0;
  font-size: 0.68rem;
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

.cockpitAction {
  grid-area: action;
  min-width: 0;
  border: 1px solid color-mix(in oklab, var(--panel-focus-edge) 56%, var(--border));
  border-radius: 8px;
  background: color-mix(in oklab, var(--selection-fill) 84%, var(--surface-working));
  padding: 0.48rem 0.56rem;
  display: grid;
  align-content: center;
  gap: 0.08rem;
}

.cockpitAction.blocked {
  border-color: color-mix(in oklab, var(--warning) 42%, var(--border));
  background: color-mix(in oklab, var(--surface-warning) 62%, var(--surface-working));
}

.cockpitAction strong {
  min-width: 0;
  font-size: 0.86rem;
  line-height: 1.18;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
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
    grid-template-columns: minmax(0, 1fr) minmax(14rem, 0.45fr);
    grid-template-areas:
      "lead action"
      "steps steps"
      "cards cards";
  }

  .cockpitCards {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
}

@media (max-width: 780px) {
  .researchCockpit {
    padding: 0.48rem 0.5rem;
    grid-template-columns: minmax(0, 1fr);
    grid-template-areas:
      "lead"
      "action"
      "steps"
      "cards";
  }

  .cockpitSteps {
    overflow-x: auto;
    grid-template-columns: repeat(5, minmax(7.5rem, 1fr));
    scrollbar-width: thin;
  }

  .cockpitCards {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 460px) {
  .cockpitCards {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
