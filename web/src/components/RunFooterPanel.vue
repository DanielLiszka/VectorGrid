<script setup lang="ts">
import { computed } from "vue";
import { storeToRefs } from "pinia";
import { useStorage } from "@vueuse/core";
import { useShellMount } from "../composables/useShellMount";
import { useTaskLifecycleSummary } from "../composables/useTaskLifecycleSummary";
import { useAnalysisStore } from "../stores/analysisStore";
import { useOptimizeStore } from "../stores/optimizeStore";
import { useCommandActionStore } from "../stores/commandActionStore";
import { taskPostureSnapshot, useTaskStore } from "../stores/taskStore";
import { useWorkflowStore } from "../stores/workflowStore";
import {
  formatOverallProgressLabel,
  taskLifecycleLabel,
} from "../utils/statusPresentation";
import { formatUiTimeFromUnixMs } from "../utils/uiTime";

const analysisStore = useAnalysisStore();
const commandActionStore = useCommandActionStore();
const optimizeStore = useOptimizeStore();
const taskStore = useTaskStore();
const workflowStore = useWorkflowStore();
const {
  strategyKind,
  runDisabled,
  canCancel,
  progressMeta,
  runPlanMeta,
  runReadinessHint,
  runReadinessBadges,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(optimizeStore);
const { bestMeta, bestTitle, drillStatus, heatmapMeta, topRows } =
  storeToRefs(analysisStore);
const { overallProgress } = storeToRefs(taskStore);
const { resultsTab, workspace } = storeToRefs(workflowStore);
const {
  activeTaskCount,
  centerStatusLabel,
  overallProgressLabel: sharedOverallProgressLabel,
  taskHeadline,
} = useTaskLifecycleSummary();

const { mountReady } = useShellMount("#runFooterVueShellMount");
const footerExpanded = useStorage<boolean>("VTA_RUN_FOOTER_EXPANDED_V1", false);
const footerVisible = computed(
  () => String(workspace.value || "").trim() === "workflow",
);

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value));
const hasRunActivity = computed(() => {
  if (canCancel.value) return true;
  return String(progressMeta.value || "").trim().length > 0;
});
const showDetails = computed(
  () => hasRunActivity.value || footerExpanded.value,
);
const idleCompact = computed(() => !showDetails.value && !hasRunActivity.value);
const footerSummary = computed(() => {
  const badges = runReadinessBadges.value.filter(
    (badge) => String(badge || "").trim().length > 0,
  );
  if (hasRunActivity.value && String(progressMeta.value || "").trim())
    return String(progressMeta.value || "").trim();
  if (badges.length > 0) return badges.slice(0, 2).join(" · ");
  return (
    String(runReadinessHint.value || "").trim() ||
    "Ready to run once data and strategy inputs are configured."
  );
});
const runningCount = computed(() => activeTaskCount.value);
const analysisSummary = computed(() => {
  const title = String(bestTitle.value || "").trim();
  const meta = String(bestMeta.value || "").trim();
  if (title && title !== "—" && title !== "-")
    return `${title}${meta && meta !== "—" && meta !== "-" ? ` · ${meta}` : ""}`;
  return "No scored run yet.";
});
const taskSummary = computed(() => taskHeadline.value);
const taskPosture = computed(() => taskPostureSnapshot(taskStore.$state, 6));
const taskPostureChips = computed(() => {
  const counts = taskPosture.value.counts;
  return [
    counts.running ? `${counts.running} running` : "",
    counts.queued ? `${counts.queued} queued` : "",
    counts.cancelling ? `${counts.cancelling} cancelling` : "",
    counts.failed ? `${counts.failed} failed` : "",
    counts.cancelled ? `${counts.cancelled} cancelled` : "",
    counts.completed ? `${counts.completed} completed` : "",
  ].filter((chip) => chip.length > 0);
});
const latestAttentionTask = computed(
  () => taskPosture.value.failed[0] ?? taskPosture.value.cancelled[0] ?? null,
);
const latestCompletedTask = computed(() => taskPosture.value.completed[0] ?? null);
const overallProgressLabel = computed(() => {
  const shared = String(sharedOverallProgressLabel.value || "").trim();
  if (shared && shared !== "No active progress") return shared;
  return formatOverallProgressLabel(overallProgress.value);
});
const primaryActionLabel = computed(() =>
  String(strategyKind.value || "").trim() === "MlSignal"
    ? "Build score cache"
    : "Run optimization",
);

function runOptimizationAction(): void {
  commandActionStore.runOptimization();
}

function cancelRunAction(): void {
  commandActionStore.cancelOptimization();
}

function toggleFooterDetails(): void {
  footerExpanded.value = !footerExpanded.value;
}

</script>

<template>
  <Teleport v-if="mountReady && footerVisible" to="#runFooterVueShellMount">
    <section
      class="runFooterVueRoot"
      :class="{ compact: !showDetails, idleCompact }"
    >
      <div class="primaryRow">
        <header class="head">
          <div class="eyebrow">Run Control</div>
          <h2 class="title" style="margin: 0;">Execution readiness</h2>
          <div class="meta mono">Updated {{ stamp }}</div>
          <div class="summaryPill mono" role="status" aria-live="polite">{{ footerSummary }}</div>
        </header>

        <div class="actions">
          <button type="button" class="subtle" @click="toggleFooterDetails">
            {{ showDetails ? "Hide details" : "Show details" }}
          </button>
          <button
            type="button"
            class="primary"
            :disabled="runDisabled"
            @click="runOptimizationAction"
          >
            {{ primaryActionLabel }}
          </button>
          <button
            type="button"
            class="danger-btn"
            :disabled="!canCancel"
            @click="cancelRunAction"
          >
            Cancel
          </button>
        </div>
      </div>

      <div v-if="showDetails" class="summaryGrid">
        <section class="summaryCard summaryCardPrimary">
          <div class="summaryLabel">Plan</div>
          <div v-if="progressMeta" class="metaLine mono">
            {{ progressMeta }}
          </div>
          <div class="metaLine mono">{{ runPlanMeta }}</div>
          <div class="status mono" role="status" aria-live="polite">{{ statusLine }}</div>
        </section>

        <section class="summaryCard">
          <div class="summaryLabel">Readiness</div>
          <div class="badges" aria-live="polite" aria-label="Run readiness">
            <span
              v-for="badge in runReadinessBadges"
              :key="`run_badge_${badge}`"
              class="badge"
              >{{ badge }}</span
            >
          </div>
          <div class="metaLine mono">{{ runReadinessHint }}</div>
        </section>

        <section class="summaryCard">
          <div class="summaryLabel">Insights</div>
          <div class="metaLine mono">{{ analysisSummary }}</div>
          <div class="status mono">
            Tab {{ resultsTab || "overview" }} · Top {{ topRows }} · Drill
            {{ drillStatus }}
          </div>
          <div class="status mono">Heatmap {{ heatmapMeta }}</div>
        </section>

        <section class="summaryCard">
          <div class="summaryLabel">Tasks</div>
          <div class="metaLine mono">
            {{ centerStatusLabel }} · {{ runningCount }} active
          </div>
          <div v-if="taskPostureChips.length > 0" class="badges" aria-label="Task posture">
            <span
              v-for="chip in taskPostureChips"
              :key="`task_posture_${chip}`"
              class="badge postureBadge"
            >
              {{ chip }}
            </span>
          </div>
          <div class="status mono">{{ overallProgressLabel }}</div>
          <div class="status mono">{{ taskSummary }}</div>
          <div v-if="latestAttentionTask" class="status mono">
            Attention · {{ latestAttentionTask.label }} {{ taskLifecycleLabel(latestAttentionTask.status) }}
          </div>
          <div v-else-if="latestCompletedTask" class="status mono">
            Latest complete · {{ latestCompletedTask.label }}
          </div>
        </section>
      </div>
    </section>
  </Teleport>
</template>

<style scoped>
.runFooterVueRoot {
  display: grid;
  gap: 0.75rem;
}

.runFooterVueRoot.compact {
  gap: 0;
}

.runFooterVueRoot.idleCompact .primaryRow {
  align-items: center;
}

.runFooterVueRoot.idleCompact .eyebrow {
  font-size: 0.64rem;
}

.runFooterVueRoot.idleCompact .title {
  font-size: 0.9rem;
}

.runFooterVueRoot.idleCompact .meta {
  display: none;
}

.runFooterVueRoot.idleCompact .summaryPill {
  margin-top: 0.1rem;
  padding: 0.22rem 0.52rem;
  font-size: 0.72rem;
}

.runFooterVueRoot.idleCompact .actions button {
  min-height: 2.05rem;
  padding: 0.42rem 0.68rem;
  font-size: 0.76rem;
}

.primaryRow {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 1rem;
  align-items: end;
}

.head {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.18rem;
  min-width: 0;
}

.eyebrow {
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--muted);
}

.title {
  font-size: 1rem;
  font-weight: 650;
  letter-spacing: 0.01em;
}

.meta {
  font-size: 0.75rem;
  color: var(--muted);
}

.summaryPill {
  margin-top: 0.18rem;
  max-width: min(76ch, 100%);
  border: 1px solid color-mix(in oklab, var(--border) 80%, transparent);
  border-radius: 999px;
  background: color-mix(in oklab, var(--surface2) 88%, transparent);
  color: var(--muted);
  font-size: 0.76rem;
  line-height: 1.2;
  padding: 0.3rem 0.62rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
  justify-content: flex-end;
}

.actions button {
  min-height: 2.35rem;
  padding: 0.55rem 0.8rem;
  font-size: 0.8rem;
}

.actions .subtle {
  background: color-mix(in oklab, var(--surface2) 84%, transparent);
  color: var(--muted);
}

.actions .primary {
  border-color: color-mix(in oklab, var(--accent) 72%, var(--border));
}

.summaryGrid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.75rem;
}

.summaryCard {
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: 14px;
  background: color-mix(in oklab, var(--panel) 98%, var(--surface));
  padding: 0.7rem 0.8rem;
  display: grid;
  gap: 0.45rem;
}

.summaryCardPrimary {
  background: linear-gradient(
    135deg,
    color-mix(in oklab, var(--accent-soft) 38%, var(--panel)),
    color-mix(in oklab, var(--panel) 98%, var(--surface))
  );
}

.summaryLabel {
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--muted);
}

.metaLine {
  font-size: 0.8rem;
  color: var(--text);
  line-height: 1.3;
}

.badges {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}

.badge {
  border: 1px solid var(--border);
  border-radius: 999px;
  padding: 0.24rem 0.52rem;
  font-size: 0.72rem;
  background: color-mix(in oklab, var(--panel) 84%, var(--surface));
}

.postureBadge {
  background: color-mix(in oklab, var(--surface-inline) 88%, var(--panel));
}

.status {
  font-size: 0.78rem;
  color: var(--muted);
  padding-top: 0.1rem;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 980px) {
  .primaryRow,
  .summaryGrid {
    grid-template-columns: 1fr;
  }

  .actions {
    justify-content: flex-start;
  }

  .summaryPill {
    white-space: normal;
  }
}
</style>

<style>
</style>
